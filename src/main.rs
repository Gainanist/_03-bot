mod components;
mod bundles;
mod localization;
mod command_parser;
mod dice;
mod events;
mod game_helpers;
mod systems;
mod cli;

use std::{env, error::Error, sync::{Arc, Mutex}, time::{Duration}, collections::{HashMap, HashSet}, fs};

use command_parser::is_game_starting;

use components::PlayerName;
use events::{EventsPlugin, InputEvent, GameStartEvent, PlayerAttackEvent};
use futures::{stream::{StreamExt}};
use game_helpers::{GameRenderMessage, Game, EventDelay};
use localization::{Localizations, Localization};
use structopt::StructOpt;
use std::sync::mpsc::{self, Sender};

use crate::cli::Args;
use crate::{command_parser::BYGONE_PARTS_FROM_EMOJI_NAME, systems::*};

use bevy::{prelude::*, app::ScheduleRunnerSettings};
use bevy_rng::*;

use twilight_gateway::{cluster::{Cluster, ShardScheme}, Event};
use twilight_http::{Client as HttpClient, request::channel::reaction::RequestReactionType};
use twilight_model::{gateway::{Intents, payload::incoming::{MessageCreate}}, id::{ChannelId, MessageId, GuildId, UserId}, channel::{Reaction, ReactionType}, user::CurrentUser};

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error + Send + Sync>> {
    let token = env::var("DISCORD_TOKEN")?;

    // This is the default scheme. It will automatically create as many
    // shards as is suggested by Discord.
    let scheme = ShardScheme::Auto;

    // Use intents to only receive guild message events.
    let (cluster, mut events) = Cluster::builder(
        token.to_owned(),
        Intents::GUILD_MESSAGES | Intents::GUILD_MESSAGE_REACTIONS
    )
        .shard_scheme(scheme)
        .build()
        .await?;
    let cluster = Arc::new(cluster);

    // Start up the cluster.
    let cluster_spawn = Arc::clone(&cluster);

    // Start all shards in the cluster in the background.
    tokio::spawn(async move {
        cluster_spawn.up().await;
    });

    // HTTP is separate from the gateway, so create a new client.
    let http = Arc::new(HttpClient::new(token));

    let me = http.current_user().exec().await?.model().await?;

    let game_message_ids = Arc::new(Mutex::new(HashSet::<MessageId>::new()));

    let game_message_ids_input = Arc::clone(&game_message_ids);
    let me_input = me.clone();
    let (input_sender, input_receiver) = mpsc::channel();

    tokio::spawn(async move {
        let localizations = Localizations::new();
        // Process each event as they come in.
        while let Some((shard_id, event)) = events.next().await {
            match event {
                Event::MessageCreate(msg) => {
                    println!("Received MessageCreate event from channel {}", msg.channel_id);

                    if msg.author.id != me_input.id {
                        if let Some(language) = is_game_starting(&msg.content) {
                            let localization = localizations.get(language).clone();
                            start_game(&input_sender, localization, &msg);
                        }
                    }
                },
                Event::ReactionAdd(reaction) => {
                    println!("Received ReactionAdd event from channel {}", reaction.channel_id);

                    process_reaction(
                        &reaction.0,
                        &input_sender,
                        &me_input,
                        &game_message_ids_input
                    )
                },
                Event::ReactionRemove(reaction) => {
                    println!("Received ReactionRemove event from channel {}", reaction.channel_id);

                    process_reaction(
                        &reaction.0,
                        &input_sender,
                        &me_input,
                        &game_message_ids_input
                    )
                },
                Event::ShardConnected(_) => {
                    println!("Connected on shard {}", shard_id);
                }
                _ => {}
            }
        }
    });

    let game_message_ids_output = Arc::clone(&game_message_ids);
    let http_write = Arc::clone(&http);
    let (output_sender, output_receiver) = mpsc::channel::<GameRenderMessage>();

    tokio::spawn(async move {
        let mut message_ids = HashMap::new();
        loop {
            let msg = output_receiver.recv_timeout(Duration::from_secs(1));
            if let Ok(msg) = msg {
                let game_id = msg.game_id;
                let message_id = message_ids.get(&game_id);
                let channel_id = msg.channel_id;
                match send_game_message(&http_write, message_id, msg).await {
                    Ok(message_id) => {
                        println!("Successfully sent/updated a game message in channel {}", channel_id);
                        message_ids.insert(game_id, message_id);
                        if let Ok(mut game_message_ids_output_lock) = game_message_ids_output.lock() {
                            game_message_ids_output_lock.insert(message_id);
                        }
                    },
                    Err(err) => {
                        println!("Error sending/updating a game message in channel {}: {}", channel_id, err);
                    },
                }
            }
        }
    });

    let args = Args::from_args();
    let games = match fs::read(&args.games_path) {
        Ok(games_data) => match serde_json::from_slice(&games_data) {
            Ok(deserialized_games) => deserialized_games,
            Err(_) => HashMap::<ChannelId, Game>::new(),
        },
        Err(_) => HashMap::<ChannelId, Game>::new(),
    };
    let scoreboard = match fs::read(&args.scoreboard_path) {
        Ok(scoreboard_data) => match serde_json::from_slice(&scoreboard_data) {
            Ok(deserialized_scoreboard) => deserialized_scoreboard,
            Err(_) => HashMap::<GuildId, HashMap<UserId, usize>>::new(),
        },
        Err(_) => HashMap::<GuildId, HashMap<UserId, usize>>::new(),
    };

    let (games_sender, games_receiver) = mpsc::channel::<HashMap::<ChannelId, Game>>();
    let games_path = args.games_path.clone();

    tokio::spawn(async move {
        loop {
            let games = games_receiver.recv_timeout(Duration::from_secs(1));
            if let Ok(games) = games {
                if let Ok(serialized_games) = serde_json::to_string(&games) {
                    fs::write(&games_path, serialized_games);
                }
            }
        }
    });

    let (scoreboard_sender, scoreboard_receiver) = mpsc::channel::<HashMap::<GuildId, HashMap<UserId, usize>>>();
    let scoreboard_path = args.scoreboard_path.clone();

    tokio::spawn(async move {
        loop {
            let scoreboard = scoreboard_receiver.recv_timeout(Duration::from_secs(1));
            if let Ok(scoreboard) = scoreboard {
                if let Ok(serialized_scoreboard) = serde_json::to_string(&scoreboard) {
                    fs::write(&scoreboard_path, serialized_scoreboard);
                }
            }
        }
    });

    let render_label = "render";

    App::build()
        .insert_resource(ScheduleRunnerSettings::run_loop(Duration::from_millis(100)))
        .insert_resource(EventDelay(Duration::from_millis(150)))
        .insert_resource(games)
        .insert_resource(scoreboard)
        .insert_resource(HashMap::<ChannelId, Vec<String>>::new())
        .add_plugins(MinimalPlugins)
        .add_plugin(RngPlugin::default())
        .add_plugin(EventsPlugin::default())
        .add_system(listen.system().config(|params| {
            params.0 = Some(Some(Mutex::new(input_receiver)));
        }))
        .add_system(delay_events.system())
        .add_system(turn_timer.system())
        .add_system(spawn_bygones.system())
        .add_system(spawn_players.system())
        .add_system(damage_bygone.system())
        .add_system(damage_players.system())
        .add_system(process_bygone_part_death.system())
        .add_system(deactivate.system())
        .add_system(update_game_status.system())
        .add_system(log_battle.system().before(render_label))
        .add_system(render.system().config(|params| {
            params.0 = Some(Some(Mutex::new(output_sender)));
        }).label(render_label))
        .add_system(ready_players.system())
        .add_system(cleanup.system())
        .add_system(save_games.system().config(|params| {
            params.0 = Some(Some(Mutex::new(games_sender)));
        }))
        .add_system(save_scoreboard.system().config(|params| {
            params.0 = Some(Some(Mutex::new(scoreboard_sender)));
        }))
        .run();

    Ok(())
}

fn process_reaction(
    reaction: &Reaction,
    sender: &Sender<InputEvent>,
    current_user: &CurrentUser,
    game_message_ids: &Mutex<HashSet<MessageId>>,
) {
    if reaction.user_id == current_user.id {
        return;
    }
    if let Ok(game_message_ids_lock) = game_message_ids.lock() {
        if !game_message_ids_lock.contains(&reaction.message_id) {
            return;
        }
    } else {
        return;
    }

    if let ReactionType::Unicode { name } = &reaction.emoji {
        if let Some(bygone_part) = BYGONE_PARTS_FROM_EMOJI_NAME.get(name) {
            let user_name = PlayerName(match &reaction.member {
                Some(member) => match &member.nick {
                    Some(nick) => nick,
                    None => &member.user.name,
                },
                None => "Anon",
            }.to_string());

            sender.send(InputEvent::PlayerAttack(
                PlayerAttackEvent::new(
                    reaction.user_id,
                    user_name,
                    reaction.channel_id,
                    *bygone_part,
                )
            ));
        }
    }
}

fn start_game(sender: &Sender<InputEvent>, localization: Localization, msg: &MessageCreate) {
    let initial_player_name = PlayerName(match &msg.member {
        Some(member) => match &member.nick {
            Some(nick) => nick,
            None => &msg.author.name,
        },
        None => &msg.author.name,
    }.to_string());
    sender.send(
        InputEvent::GameStart(GameStartEvent::new(
            msg.author.id,
            initial_player_name,
            msg.channel_id,
            localization,
        ))
    );
}

async fn send_game_message(
    http: &HttpClient,
    message_id: Option<&MessageId>,
    msg: GameRenderMessage
) -> Result<MessageId, Box<dyn Error + Send + Sync>> {
    match message_id {
        Some(message_id) => {
            http.update_message(msg.channel_id, *message_id)
                .embeds(&[])?
                .embeds(&msg.embeds.render())?
                .exec()
                .await?;
            Ok(*message_id)
        },
        None => {
            let message_id = http
                .create_message(msg.channel_id)
                .embeds(&msg.embeds.render())?
                .exec()
                .await?
                .model()
                .await?
                .id;
            for emoji_name in BYGONE_PARTS_FROM_EMOJI_NAME.keys() {
                http.create_reaction(
                        msg.channel_id,
                        message_id,
                        &RequestReactionType::Unicode { name: emoji_name }
                    )
                    .exec()
                    .await?;
            }
            Ok(message_id)
        }
    }
}