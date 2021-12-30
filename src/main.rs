mod components;
mod bygone_03;
mod localization;
mod command_parser;
mod language;
mod dice;
mod events;

use std::{env, error::Error, sync::{Arc, mpsc::Receiver, Mutex}, time::{Duration, SystemTime, Instant}, collections::{HashMap, HashSet}, fs, path::{PathBuf}};
use arrayvec::ArrayVec;
use command_parser::is_game_starting;
use components::{Player, Active, Vitality, Enemy, BygonePart, Attack, Bygone03Stage};
use enum_map::{EnumMap, Enum};
use events::*;
use futures::{stream::{StreamExt}};
use localization::{Localizations, Localization, RenderText};
use std::sync::mpsc::{self, Sender};

use crate::{bygone_03::*, command_parser::BYGONE_PARTS_FROM_EMOJI_NAME};

use bevy::{prelude::*, app::ScheduleRunnerSettings};
use bevy_rng::*;

use serde::{Deserialize, Serialize};

use twilight_embed_builder::{EmbedBuilder, EmbedFieldBuilder, ImageSource};
use twilight_gateway::{cluster::{Cluster, ShardScheme}, Event};
use twilight_http::{Client as HttpClient, request::channel::reaction::RequestReactionType};
use twilight_model::{gateway::{Intents, payload::incoming::{MessageCreate}}, id::{ChannelId, MessageId, UserId}, channel::{Reaction, ReactionType, embed::Embed}, user::CurrentUser};

fn get_games_filename() -> PathBuf {
    let dir = env::current_dir().unwrap();
    dir.join("games.json")
}

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
                    if msg.author.id != me_input.id {
                        if let Some(language) = is_game_starting(&msg.content) {
                            let localization = localizations.get(language).clone();
                            start_game(&input_sender, localization, &msg);
                        }
                    }
                },
                Event::ReactionAdd(reaction) => process_reaction(
                    &reaction.0,
                    &input_sender,
                    &me_input,
                    &game_message_ids_input
                ),
                Event::ReactionRemove(reaction) => process_reaction(
                    &reaction.0,
                    &input_sender,
                    &me_input,
                    &game_message_ids_input
                ),
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
                if let Ok(message_id) = send_game_message(&http_write, message_id, msg).await {
                    message_ids.insert(game_id, message_id);
                    if let Ok(mut game_message_ids_output_lock) = game_message_ids_output.lock() {
                        game_message_ids_output_lock.insert(message_id);
                    }
                }
            }
        }
    });

    let games = match std::env::args().nth(1) {
        Some(arg) => {
            if arg != "-p" {
                match fs::read(get_games_filename()) {
                    Ok(games_data) => match serde_json::from_slice(&games_data) {
                        Ok(deserialized_games) => deserialized_games,
                        Err(_) => HashMap::<ChannelId, Game>::new(),
                    },
                    Err(_) => HashMap::<ChannelId, Game>::new(),
                }
            } else {
                HashMap::new()
            }
        },
        None => HashMap::new(),
    };

    let listen_label = "listen";
    let spawn_label = "spawn";
    let damage_label = "damage";
    let on_death_events_label = "on_death_events";
    let deactivate_label = "deactivate";
    let update_label = "update";
    let render_label = "render";

    App::build()
        .insert_resource(ScheduleRunnerSettings::run_loop(Duration::from_millis(100)))
        .insert_resource(EventDelay(Duration::from_millis(150)))
        .insert_resource(games)
        .add_event::<BygonePartDeathEvent>()
        .add_event::<DeactivateEvent>()
        .add_event::<DelayedEvent>()
        .add_event::<EnemyAttackEvent>()
        .add_event::<GameDrawEvent>()
        .add_event::<GameStartEvent>()
        .add_event::<PlayerAttackEvent>()
        .add_event::<PlayerJoinEvent>()
        .add_plugins(MinimalPlugins)
        .add_plugin(RngPlugin::default())
        .add_system(listen.system().config(|params| {
            params.0 = Some(Some(Mutex::new(input_receiver)));
        }).label(listen_label))
        .add_system(turn_timer.system())
        .add_system(delay_events.system())
        .add_system(spawn_bygones.system().label(spawn_label).after(listen_label))
        .add_system(spawn_players.system().label(spawn_label).after(listen_label))
        .add_system(damage_bygone.system().label(damage_label).after(spawn_label))
        .add_system(damage_players.system().label(damage_label).after(spawn_label))
        .add_system(process_bygone_part_death.system().label(on_death_events_label).after(damage_label))
        .add_system(deactivate.system().label(deactivate_label).after(on_death_events_label))
        .add_system(update_game_status.system().label(update_label).after(deactivate_label))
        .add_system(render.system().config(|params| {
            params.0 = Some(Some(Mutex::new(output_sender)));
        }).label(render_label).after(update_label))
        .add_system(cleanup.system().after(render_label))
        .add_system(save_games.system())
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

#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
pub struct GameId(pub u128);

impl GameId {
    pub fn from_current_time(salt: u128) -> Self {
        let timestamp = match SystemTime::now().duration_since(SystemTime::UNIX_EPOCH) {
            Ok(dur) => dur.as_nanos(),
            Err(err) => err.duration().as_nanos(),
        };
        Self(timestamp + salt)
    }
}


#[derive(Clone, Debug)]
struct GameRenderMessage {
    channel_id: ChannelId,
    game_id: GameId,
    embeds: GameEmbeds,
}

impl GameRenderMessage {
    fn new(channel_id: ChannelId, game_id: GameId) -> Self {
        Self {
            channel_id,
            game_id,
            embeds: GameEmbeds::new(),
        }
    }
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
            let user_name = match &reaction.member {
                Some(member) => match &member.nick {
                    Some(nick) => nick,
                    None => &member.user.name,
                },
                None => "Anon",
            }.to_string();

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
    let initial_player_name = match &msg.member {
        Some(member) => match &member.nick {
            Some(nick) => nick,
            None => &msg.author.name,
        },
        None => &msg.author.name,
    }.to_string();
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

fn listen(
    mut input_receiver: Local<Option<Mutex<Receiver<InputEvent>>>>,
    mut games: ResMut<HashMap<ChannelId, Game>>,
    mut ev_game_start: EventWriter<GameStartEvent>,
    mut ev_player_attack: EventWriter<PlayerAttackEvent>,
    mut ev_delayed: EventWriter<DelayedEvent>,
    mut ev_player_join: EventWriter<PlayerJoinEvent>,
    players: Query<(&UserId, Option<&Active>), (With<Player>,)>,
) {
    let mut events = Vec::new();
    if let Some(input_receiver) = &mut *input_receiver {
        if let Ok(ref mut receiver_lock) = input_receiver.try_lock() {
            while let Ok(event) = receiver_lock.try_recv() {
                events.push(event);
            }
        }
    }
    let events = events;

    let players: HashMap<_, _> = players.iter().collect();

    for (i, event) in events.into_iter().enumerate() {
        match event {
            InputEvent::GameStart(ev) => {
                let should_start_new_game = match games.get(&ev.channel) {
                    Some(game) =>
                        game.status == GameStatus::Lost
                        || game.status != GameStatus::Ongoing && elapsed_since(&game.start_time) > 60*60*20,
                    None => true,
                };
                if should_start_new_game {
                    ev_game_start.send(ev.clone());
                    games.insert(ev.channel, Game::new(
                        GameId::from_current_time(i as u128),
                        ev.localization
                    ));
                    ev_player_join.send(PlayerJoinEvent::new(ev.initial_player, ev.initial_player_name, ev.channel));
                    ev_delayed.send(DelayedEvent::GameDraw(GameDrawEvent::new(ev.channel)));
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
                                ev.player_name.clone(),
                                ev.channel.clone(),
                            ));
                            ev_delayed.send(DelayedEvent::PlayerAttack(ev));
                        }
                    }
                }
            },
        }
    }
}

fn elapsed_since(system_time: &SystemTime) -> u64 {
    match system_time.elapsed() {
        Ok(dur) => dur.as_secs(),
        Err(_) => 0,
    }
}

fn turn_timer(
    mut timers: Local<HashMap<ChannelId, GameTimer>>,
    mut ev_player_attack: EventReader<PlayerAttackEvent>,
    mut ev_enemy_attack: EventWriter<EnemyAttackEvent>,
    mut ev_game_draw: EventWriter<GameDrawEvent>,
) {
    for (channel_id, timer) in timers.iter_mut() {
        if timer.enemy_attack() {
            ev_enemy_attack.send(EnemyAttackEvent::new(*channel_id));
        }
        if timer.turn_end() {
            ev_game_draw.send(GameDrawEvent::new(*channel_id));
        }
    }

    timers.retain(|_, timer| !timer.depleted());

    for ev in ev_player_attack.iter() {
        timers.entry(ev.channel).or_insert(GameTimer::new());
    }
}

#[derive(Clone, Debug)]
struct GameTimer {
    start: Instant,
    enemy_attacked: bool,
    turn_ended: bool,
}

impl GameTimer {
    const TURN_DURATION: Duration = Duration::from_secs(5);
    const ENEMY_ATTACK_DELAY: Duration = Duration::from_secs(2);

    pub fn new() -> Self {
        Self {
            start: Instant::now(),
            enemy_attacked: false,
            turn_ended: false,
        }
    }

    pub fn depleted(&self) -> bool {
        self.enemy_attacked && self.turn_ended
    }

    pub fn enemy_attack(&mut self) -> bool {
        if self.enemy_attacked || self.start.elapsed() < Self::ENEMY_ATTACK_DELAY {
            false
        } else {
            self.enemy_attacked = true;
            true
        }
    }

    pub fn turn_end(&mut self) -> bool {
        if self.turn_ended || self.start.elapsed() < Self::TURN_DURATION {
            false
        } else {
            self.turn_ended = true;
            true
        }
    }
}

fn update_game_status(
    mut games: ResMut<HashMap<ChannelId, Game>>,
    mut ev_deactivate: EventReader<DeactivateEvent>,
    mut ev_game_draw: EventWriter<GameDrawEvent>,
    active_players: Query<(Entity, &ChannelId,), (With<Player>, With<Active>)>,
    active_enemies: Query<(Entity, &ChannelId,), (With<Enemy>, With<Active>)>,
    entities: Query<(Entity, &ChannelId), (Or<(With<Enemy>, With<Player>)>,)>,
) {
    let deactivated: HashSet<_> = ev_deactivate.iter()
        .filter_map(|ev| entities.get(ev.0).ok())
        .map(|(entity, _)| entity)
        .collect();

    for (channel_id, game) in games.iter_mut().filter(|(_, game)| game.status == GameStatus::Ongoing) {
        let initialized = entities.iter().any(|(_, enemy_channel_id)| enemy_channel_id == channel_id);
        if !initialized {
            continue;
        }
        if active_enemies.iter().all(|(entity, enemy_channel_id,)|
            enemy_channel_id != channel_id || deactivated.contains(&entity)
        ) {
            game.status = GameStatus::Won;
        } else if active_players.iter().all(|(entity, player_channel_id,)|
            player_channel_id != channel_id || deactivated.contains(&entity)
        ) {
            game.status = GameStatus::Lost;
        }
        if game.status != GameStatus::Ongoing {
            ev_game_draw.send(GameDrawEvent::new(*channel_id));
        }
    }

}

#[derive(Clone, Copy, Debug, Deserialize, Enum, Eq, Hash, PartialEq, Serialize)]
pub enum GameStatus {
    Ongoing,
    Won,
    Lost,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Game {
    pub start_time: SystemTime,
    pub game_id: GameId,
    pub localization: Localization,
    pub status: GameStatus,
}

impl Game {
    pub fn new(game_id: GameId, localization: Localization) -> Self {
        Self {
            start_time: SystemTime::now(),
            game_id,
            localization,
            status: GameStatus::Ongoing,
        }
    }
}

fn render(
    sender: Local<Option<Mutex<Sender<GameRenderMessage>>>>,
    games: Res<HashMap<ChannelId, Game>>,
    mut ev_game_draw: EventReader<GameDrawEvent>,
    players: Query<(&String, &ChannelId, &Vitality), (With<Player>,)>,
    enemies: Query<(&ChannelId, &EnumMap<BygonePart, Vitality>, &Attack, &Bygone03Stage), (With<Enemy>,)>,
) {
    for GameDrawEvent{ channel_id } in ev_game_draw.iter() {
        if let Some(game) = games.get(channel_id) {
            let loc = &game.localization;
            let mut message = GameRenderMessage::new(
                *channel_id,
                game.game_id,
            );
            message.embeds.title = Some(match game.status {
                GameStatus::Ongoing => EmbedBuilder::new()
                    .description(&loc.title)
                    .image(
                        ImageSource::url(
                            "http://www.uof7.com/wp-content/uploads/2016/09/15-Bygone-UPD.gif"
                        ).unwrap()
                    )
                    .build()
                    .unwrap(),
                GameStatus::Won => EmbedBuilder::new()
                    .description(&loc.won)
                    .build()
                    .unwrap(),
                GameStatus::Lost => EmbedBuilder::new()
                    .description(&loc.lost)
                    .build()
                    .unwrap(),
            });
            if game.status == GameStatus::Ongoing {
                for (enemy_channel_id, parts, attack, stage) in enemies.iter() {
                    if enemy_channel_id != channel_id {
                        continue;
                    }
                    let loc = &game.localization;
                    let status = format!("{}, {}: {}", attack.render_text(loc), &loc.core, stage.render_text(loc));
                    let core = parts[BygonePart::Core].render_text(loc);
                    let sensor = parts[BygonePart::Sensor].render_text(loc);
                    let left_wing = parts[BygonePart::LeftWing].render_text(loc);
                    let right_wing = parts[BygonePart::RightWing].render_text(loc);
                    let gun = parts[BygonePart::Gun].render_text(loc);

                    message.embeds.enemies = Some(
                        EmbedBuilder::new()
                            .field(EmbedFieldBuilder::new(&loc.status_title, status).build())
                            .field(EmbedFieldBuilder::new(&loc.core_title, core).inline())
                            .field(EmbedFieldBuilder::new(&loc.sensor_title, sensor).inline())
                            .field(EmbedFieldBuilder::new(&loc.left_wing_title, left_wing).inline())
                            .field(EmbedFieldBuilder::new(&loc.right_wing_title, right_wing).inline())
                            .field(EmbedFieldBuilder::new(&loc.gun_title, gun).inline())
                            .build()
                            .unwrap()
                    );
                }

                let mut players_embed_builder = EmbedBuilder::new();
                for (name, player_channel_id, vitality) in players.iter() {
                    if player_channel_id != channel_id {
                        continue;
                    }
                    players_embed_builder = players_embed_builder.field(EmbedFieldBuilder::new(
                        name,
                        vitality.health().render_text(loc)
                    ));
                }
                message.embeds.players = Some(players_embed_builder.build().unwrap());
            }

            if let Some(sender) = &*sender {
                if let Ok(ref mut sender_lock) = sender.lock() {    
                    sender_lock.send(message);
                }
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

pub fn save_games(games: Res<HashMap<ChannelId, Game>>) {
    if let Ok(serialized_games) = serde_json::to_string(&*games) {
        fs::write(get_games_filename(), serialized_games);
    }
}
