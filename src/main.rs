mod components;
mod bygone_03;
mod localization;
mod command_parser;
mod language;
mod dice;
mod events;

use std::{env, error::Error, sync::{Arc, mpsc::Receiver, Mutex}, time::{Duration, Instant}, collections::HashMap};
use arrayvec::ArrayVec;
use command_parser::is_game_starting;
use components::{Player, Active, Vitality, Enemy, BygonePart, Attack, Bygone03Stage};
use enum_map::{EnumMap, Enum};
use events::*;
use futures::{stream::{StreamExt}};
use localization::{Localizations, Localization, RenderText};
use std::sync::mpsc::{self, Sender};
// use systems::Game;

use crate::{bygone_03::*, command_parser::BYGONE_PARTS_FROM_EMOJI_NAME};

use bevy::{prelude::*, app::ScheduleRunnerSettings};
use bevy_rng::*;

use twilight_embed_builder::{EmbedBuilder, EmbedFieldBuilder, ImageSource};
use twilight_gateway::{cluster::{Cluster, ShardScheme}, Event};
use twilight_http::{Client as HttpClient};
use twilight_model::{gateway::{Intents, payload::incoming::{MessageCreate}}, id::{ChannelId, MessageId, UserId}, channel::{Reaction, ReactionType, embed::Embed}};

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

    let http_input = Arc::clone(&http);
    let (input_sender, input_receiver) = mpsc::channel();

    tokio::spawn(async move {
        let localizations = Localizations::new();
        // Process each event as they come in.
        while let Some((shard_id, event)) = events.next().await {
            match event {
                Event::MessageCreate(msg) => {
                    if !msg.author.bot {
                        if let Some(language) = is_game_starting(&msg.content) {
                            let localization = localizations.get(language).clone();
                            let game_start_sender = input_sender.clone();
                            let game_start_http = Arc::clone(&http_input);

                            tokio::spawn(async move {
                                start_game(game_start_sender, game_start_http, localization, msg).await;
                            });
                        }
                    }
                },
                Event::ReactionAdd(reaction) => process_reaction(&reaction.0, &input_sender),
                Event::ReactionRemove(reaction) => process_reaction(&reaction.0, &input_sender),
                Event::ShardConnected(_) => {
                    println!("Connected on shard {}", shard_id);
                }
                _ => {}
            }
        }
    });

    let http_write = Arc::clone(&http);
    let (output_sender, output_receiver) = mpsc::channel::<GameRenderMessage>();

    tokio::spawn(async move {
        loop {
            let msg = output_receiver.recv_timeout(Duration::from_secs(1));
            if let Ok(msg) = msg {
                send_game_message(&http_write, msg).await;
            }
        }
    });

    let listen_label = "listen";
    let spawn_label = "spawn";
    let damage_label = "damage";
    let on_death_events_label = "on_death_events";
    let deactivate_label = "deactivate";
    let update_label = "update";
    let render_label = "render";

    App::build()
        .insert_resource(ScheduleRunnerSettings::run_loop(Duration::from_secs(5)))
        .insert_resource(HashMap::<ChannelId, Game>::new())
        .add_event::<BygonePartDeathEvent>()
        .add_event::<DeactivateEvent>()
        .add_event::<GameStartEvent>()
        .add_event::<GameEndEvent>()
        .add_event::<PlayerAttackEvent>()
        .add_event::<PlayerJoinEvent>()
        .add_plugins(MinimalPlugins)
        .add_plugin(RngPlugin::default())
        .add_system(listen.system().config(|params| {
            params.0 = Some(Some(Mutex::new(input_receiver)));
        }).label(listen_label))
        .add_system(spawn_bygones.system().label(spawn_label).after(listen_label))
        .add_system(spawn_players.system().label(spawn_label).after(listen_label))
        .add_system(damage_bygone.system().label(damage_label).after(spawn_label))
        .add_system(damage_players.system().label(damage_label).after(spawn_label))
        .add_system(process_bygone_part_death.system().label(on_death_events_label).after(damage_label))
        .add_system(deativate.system().label(deactivate_label).after(on_death_events_label))
        .add_system(update_game_status.system().label(update_label).after(deactivate_label))
        .add_system(render.system().config(|params| {
            params.0 = Some(Some(Mutex::new(output_sender)));
        }).label(render_label).after(update_label))
        .add_system(cleanup.system().after(render_label))
        .run();

    Ok(())
}


#[derive(Clone, Debug, Default)]
struct GameEmbeds {
    title: Option<Embed>,
    enemies: Option<Embed>,
    log: Option<Embed>,
    players: Option<Embed>,
}

impl GameEmbeds {
    fn new() -> Self {
        Self::default()
    }

    fn render(self) -> ArrayVec<Embed, 4> {
        let mut embeds = ArrayVec::new();
        if let Some(title) = self.title {
            embeds.push(title);
        }
        if let Some(enemies) = self.enemies {
            embeds.push(enemies);
        }
        if let Some(log) = self.log {
            embeds.push(log);
        }
        if let Some(players) = self.players {
            embeds.push(players);
        }
        embeds
    }
}

#[derive(Clone, Debug)]
struct GameRenderMessage {
    channel_id: ChannelId,
    message_id: MessageId,
    embeds: GameEmbeds,
}

impl GameRenderMessage {
    fn new(channel_id: ChannelId, message_id: MessageId) -> Self {
        Self {
            channel_id,
            message_id,
            embeds: GameEmbeds::new(),
        }
    }
}

fn process_reaction(reaction: &Reaction, sender: &Sender<InputEvent>) {
    if let ReactionType::Unicode { name } = &reaction.emoji {
        if let Some(bygone_part) = BYGONE_PARTS_FROM_EMOJI_NAME.get(name) {
            sender.send(InputEvent::PlayerAttack(
                PlayerAttackEvent::new(
                    reaction.user_id,
                    reaction.message_id,
                    reaction.channel_id,
                    *bygone_part,
                )
            )
        );
        }
    }
}

async fn start_game(sender: Sender<InputEvent>, http: Arc<HttpClient>, localization: Localization, msg: Box<MessageCreate>)
    -> Result<(), Box<dyn Error + Sync + Send>>
{
    let emdeds = [
        EmbedBuilder::new()
            .description(&localization.intro)
            .build()
            .unwrap(),
    ];
    let message_id = http
        .create_message(msg.channel_id)
        .embeds(&emdeds)?
        .exec()
        .await?
        .model()
        .await?
        .id;
    sender.send(
        InputEvent::GameStart(GameStartEvent::new(
            msg.author.id,
            message_id,
            msg.channel_id,
            localization,
        ))
    )?;
    Ok(())
}

async fn send_game_message(
    http: &HttpClient,
    msg: GameRenderMessage
) -> Result<(), Box<dyn Error + Send + Sync>> {
    http.update_message(msg.channel_id, msg.message_id)
        .embeds(&[])?
        .embeds(&msg.embeds.render())?
        .exec()
        .await?;
    // TODO: Add reactions
    Ok(())
}

fn listen(
    mut input_receiver: Local<Option<Mutex<Receiver<InputEvent>>>>,
    mut games: ResMut<HashMap<ChannelId, Game>>,
    mut ev_game_start: EventWriter<GameStartEvent>,
    mut ev_player_attack: EventWriter<PlayerAttackEvent>,
    mut ev_player_join: EventWriter<PlayerJoinEvent>,
    players: Query<(&UserId, Option<&Active>), (With<Player>,)>,
) {
    let mut events = Vec::new();
    if let Some(input_receiver) = &mut *input_receiver {
        if let Ok(ref mut receiver_lock) = input_receiver.try_lock() {
            while let Ok(event) = receiver_lock.recv() {
                events.push(event);
            }
        }
    }
    let events = events;

    let players: HashMap<_, _> = players.iter().collect();

    for event in events.into_iter() {
        match event {
            InputEvent::GameStart(ev) => {
                let should_start_new_game = match games.get(&ev.channel) {
                    Some(game) => game.status != GameStatus::Ongoing || game.start_time.elapsed().as_secs() > 60*60*24,
                    None => true,
                };
                if should_start_new_game {
                    ev_game_start.send(ev.clone());
                    games.insert(ev.channel, Game::new(ev.message, ev.localization));
                    ev_player_join.send(PlayerJoinEvent::new(ev.initial_player, ev.channel));
                }
            }
            InputEvent::PlayerAttack(ev) => {
                if games.contains_key(&ev.channel) {
                    match players.get(&ev.player) {
                        Some(maybe_active) => {
                            if let Some(_active) = maybe_active {
                                ev_player_attack.send(ev);
                            }
                        },
                        None => {
                            ev_player_join.send(PlayerJoinEvent::new(
                                ev.player.clone(),
                                ev.channel.clone(),
                            ));
                            ev_player_attack.send(ev);
                        }
                    }
                }
            },
        }
    }
}

fn update_game_status(
    mut games: ResMut<HashMap<ChannelId, Game>>,
    players: Query<(&ChannelId,), (With<Player>, With<Active>)>,
    enemies: Query<(&ChannelId,), (With<Enemy>, With<Active>)>,
) {
    for (channel_id, game) in games.iter_mut().filter(|(_, game)| game.status == GameStatus::Ongoing) {
        if enemies.iter().all(|(enemy_channel_id,)| enemy_channel_id != channel_id) {
            game.status = GameStatus::Won;
        } else if players.iter().all(|(player_channel_id,)| player_channel_id != channel_id) {
            game.status = GameStatus::Lost;
        }
    }

}

#[derive(Clone, Copy, Debug, Enum, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum GameStatus {
    Ongoing,
    Won,
    Lost,
}

#[derive(Clone, Debug)]
pub struct Game {
    pub start_time: Instant,
    pub message_id: MessageId,
    pub localization: Localization,
    pub status: GameStatus,
}

impl Game {
    pub fn new(message_id: MessageId, localization: Localization) -> Self {
        Self {
            start_time: Instant::now(),
            message_id,
            localization,
            status: GameStatus::Ongoing,
        }
    }
}

fn render(
    sender: Local<Option<Mutex<Sender<GameRenderMessage>>>>,
    games: Res<HashMap<ChannelId, Game>>,
    players: Query<(&UserId, &ChannelId, &Vitality), (With<Player>,)>,
    enemies: Query<(&ChannelId, &EnumMap<BygonePart, Vitality>, &Attack, &Bygone03Stage), (With<Enemy>,)>,
) {
    let mut embeds: HashMap<_, _> = games.iter()
        .map(|(channel_id, game)| {
            let mut message = GameRenderMessage::new(*channel_id, game.message_id);
            message.embeds.title = Some(match game.status {
                GameStatus::Ongoing => EmbedBuilder::new()
                    .description("**A wild _03 appeared!**")
                    .image(
                        ImageSource::url(
                            "http://www.uof7.com/wp-content/uploads/2016/09/15-Bygone-UPD.gif"
                        ).unwrap()
                    )
                    .build()
                    .unwrap(),
                GameStatus::Won => EmbedBuilder::new()
                    .description("**Man triumphs over machine!**")
                    .build()
                    .unwrap(),
                GameStatus::Lost => EmbedBuilder::new()
                    .description("**This darkness… Am I… dead? It’s so peaceful.**")
                    .build()
                    .unwrap(),
            });

            (channel_id, message)
        })
        .collect();

    for (channel_id, parts, attack, stage) in enemies.iter() {
        if let Some(game) = games.get(channel_id) {
            if game.status != GameStatus::Ongoing {
                continue;
            }
            if let Some(message) = embeds.get_mut(channel_id) {
                let loc = &game.localization;
                let status = format!("{}, Core: {}", attack.render_text(loc), stage.render_text(loc));
                let core = parts[BygonePart::Core].render_text(loc);
                let sensor = parts[BygonePart::Sensor].render_text(loc);
                let left_wing = parts[BygonePart::LeftWing].render_text(loc);
                let right_wing = parts[BygonePart::RightWing].render_text(loc);
                let gun = parts[BygonePart::Gun].render_text(loc);

                message.embeds.enemies = Some(
                    EmbedBuilder::new()
                        .field(EmbedFieldBuilder::new("Status", status).build())
                        .field(EmbedFieldBuilder::new(":regional_indicator_c:ore", core).inline())
                        .field(EmbedFieldBuilder::new(":regional_indicator_s:ensor", sensor).inline())
                        .field(EmbedFieldBuilder::new(":regional_indicator_l:eft wing", left_wing).inline())
                        .field(EmbedFieldBuilder::new(":regional_indicator_r:ight wing", right_wing).inline())
                        .field(EmbedFieldBuilder::new(":regional_indicator_g:un", gun).inline())
                        .build()
                        .unwrap()
                );
            }
        }
    }

    for (channel_id, game) in games.iter() {
        if game.status != GameStatus::Ongoing {
            continue;
        }
        if let Some(message) = embeds.get_mut(channel_id) {
            let loc = &game.localization;
            let mut players_embed_builder = EmbedBuilder::new();

            for (user_id, player_channel_id, vitality) in players.iter() {
                if player_channel_id != channel_id {
                    continue;
                }
                players_embed_builder = players_embed_builder.field(EmbedFieldBuilder::new(
                    user_id.0.to_string(),
                    vitality.health().render_text(loc)
                ));
            }
            message.embeds.players = Some(players_embed_builder.build().unwrap());
        }
    }

    if let Some(sender) = & *sender {
        if let Ok(ref mut sender_lock) = sender.try_lock() {    
            for message in embeds.into_values() {
                sender_lock.send(message);
            }
        }
    }
}

pub fn cleanup(
    mut commands: Commands,
    games: Res<HashMap<ChannelId, Game>>,
    entities: Query<(Entity, &ChannelId)>,
) {
    for (entity, channel_id) in entities.iter() {
        if let Some(game) = games.get(channel_id) {
            if game.status != GameStatus::Ongoing {
                commands.entity(entity).despawn();
            }
        }
    }
}
