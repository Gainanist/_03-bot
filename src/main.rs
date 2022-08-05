mod components;
mod bundles;
mod localization;
mod command_parser;
mod dice;
mod events;
mod game_helpers;
mod systems;
mod cli;
mod io;

use std::{env, error::Error, sync::{Arc, Mutex}, time::{Duration}, collections::{HashMap, HashSet}, fs};

use bevy_turborand::RngPlugin;
use clap::Parser;
use io::{read_json, write_json_from_channel};
use command_parser::is_game_starting;

use components::PlayerName;
use events::{EventsPlugin, InputEvent, GameStartEvent, PlayerAttackEvent};
use futures::{stream::{StreamExt}};
use game_helpers::{GameRenderMessage, Game, EventDelay};
use localization::{Localizations, Localization};
use std::sync::mpsc::{self, Sender};

use crate::cli::Cli;
use crate::{command_parser::BYGONE_PARTS_FROM_EMOJI_NAME, systems::*};

use bevy::{prelude::*, app::ScheduleRunnerSettings};

use twilight_gateway::{cluster::{Cluster, ShardScheme}, Event};
use twilight_http::{Client as HttpClient, request::channel::reaction::RequestReactionType};
use twilight_model::{gateway::{Intents, payload::incoming::{MessageCreate}}, id::{Id, marker::{ChannelMarker, UserMarker, GuildMarker, MessageMarker}}, channel::{Reaction, ReactionType}, user::CurrentUser};

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

    let game_message_ids = Arc::new(Mutex::new(
        HashSet::<Id<MessageMarker>>::new()
    ));
    let game_channel_ids = Arc::new(Mutex::new(
        HashMap::<Id<GuildMarker>, Id<ChannelMarker>>::new()
    ));

    let game_message_ids_input = Arc::clone(&game_message_ids);
    let game_channel_ids_input = Arc::clone(&game_channel_ids);
    let me_input = me.clone();
    let (input_sender, input_receiver) = mpsc::channel();

    tokio::spawn(async move {
        let localizations = Localizations::new();
        // Process each event as they come in.
        while let Some((shard_id, event)) = events.next().await {
            match event {
                Event::MessageCreate(msg) => {
                    println!("Received MessageCreate event from channel {}", msg.channel_id);
                    if let Some(guild_id) = msg.guild_id {
                        if msg.author.id != me_input.id {
                            if let Some(language) = is_game_starting(&msg.content) {
                                let localization = localizations.get(language).clone();
                                start_game(&input_sender, localization, &msg);
                                if let Ok(mut game_channel_ids_input_lock) = game_channel_ids_input.lock() {
                                    game_channel_ids_input_lock.insert(guild_id, msg.channel_id);
                                }
                            }
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
    let game_channel_ids_output = Arc::clone(&game_channel_ids);
    let http_write = Arc::clone(&http);
    let (output_sender, output_receiver) = mpsc::channel::<GameRenderMessage>();

    tokio::spawn(async move {
        let mut message_ids = HashMap::new();
        loop {
            let msg = output_receiver.recv_timeout(Duration::from_secs(1));
            if let Ok(msg) = msg {
                let mut channel_id = None;
                if let Ok(game_channel_ids_output_lock) = game_channel_ids_output.lock() {
                    if let Some(_channel_id) = game_channel_ids_output_lock.get(&msg.guild_id) {
                        channel_id = Some(*_channel_id);
                    }
                }

                if let Some(channel_id) = channel_id {
                    let game_id = msg.game_id;
                    let message_id = message_ids.get(&game_id);
                    match send_game_message(&http_write, message_id, msg, channel_id).await {
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
        }
    });

    let cli = Cli::parse();
    let games = read_json::<HashMap::<Id<GuildMarker>, Game>>(&cli.games_path);
    let scoreboard = read_json::<HashMap::<Id<GuildMarker>, HashMap<Id<UserMarker>, usize>>>(&cli.scoreboard_path);

    let (games_sender, games_receiver) = mpsc::channel::<HashMap::<Id<GuildMarker>, Game>>();
    let games_path = cli.games_path.clone();

    tokio::spawn(async move {
        loop {
            write_json_from_channel(&games_receiver, &games_path);
        }
    });

    // let (scoreboard_sender, scoreboard_receiver) = mpsc::channel::<HashMap::<Id<GuildMarker>, HashMap<Id<UserMarker>, usize>>>();
    // let scoreboard_path = cli.scoreboard_path.clone();

    // tokio::spawn(async move {
    //     loop {
    //         write_json_from_channel(&scoreboard_receiver, &scoreboard_path);
    //     }
    // });

    let render_label = "render";

    App::new()
        .insert_resource(ScheduleRunnerSettings::run_loop(Duration::from_millis(100)))
        .insert_resource(EventDelay(Duration::from_millis(150)))
        .insert_resource(games)
        .insert_resource(scoreboard)
        .insert_resource(HashMap::<Id<ChannelMarker>, Vec<String>>::new())
        .add_plugins(MinimalPlugins)
        .add_plugin(RngPlugin::default())
        .add_plugin(EventsPlugin::default())
        .add_system(listen(Mutex::new(input_receiver)))
        .add_system(delay_events)
        .add_system(turn_timer)
        .add_system(spawn_bygones)
        .add_system(spawn_players)
        .add_system(damage_bygone)
        .add_system(damage_players)
        .add_system(process_bygone_part_death)
        .add_system(deactivate)
        .add_system(update_game_status)
        .add_system(log_battle.before(render_label))
        .add_system(render(Mutex::new(output_sender)).label(render_label))
        .add_system(ready_players)
        .add_system(cleanup)
        .add_system(save_games(Mutex::new(games_sender)))
        // .add_system(save_scoreboard(Mutex::new(scoreboard_sender)))
        .run();

    Ok(())
}

fn process_reaction(
    reaction: &Reaction,
    sender: &Sender<InputEvent>,
    current_user: &CurrentUser,
    game_message_ids: &Mutex<HashSet<Id<MessageMarker>>>,
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
            if let Some(guild) = reaction.guild_id {
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
                        reaction.guild_id.unwrap(),
                        *bygone_part,
                    )
                ));
            }
        }
    }
}

fn start_game(sender: &Sender<InputEvent>, localization: Localization, msg: &MessageCreate) {
    if let Some(guild) = msg.guild_id {
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
                guild,
                localization,
            ))
        );
    }
}

async fn send_game_message(
    http: &HttpClient,
    message_id: Option<&Id<MessageMarker>>,
    msg: GameRenderMessage,
    channel_id: Id<ChannelMarker>,
) -> Result<Id<MessageMarker>, Box<dyn Error + Send + Sync>> {
    match message_id {
        Some(message_id) => {
            http.update_message(channel_id, *message_id)
                .embeds(&[])?
                .embeds(&msg.embeds.render())?
                .exec()
                .await?;
            Ok(*message_id)
        },
        None => {
            let message_id = http
                .create_message(channel_id)
                .embeds(&msg.embeds.render())?
                .exec()
                .await?
                .model()
                .await?
                .id;
            for emoji_name in BYGONE_PARTS_FROM_EMOJI_NAME.keys() {
                http.create_reaction(
                        channel_id,
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