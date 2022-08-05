use std::{
    collections::{HashMap, HashSet, VecDeque},
    ops::DerefMut,
    sync::{
        mpsc::{Receiver, Sender},
        Mutex,
    },
    time::{Instant, SystemTime},
};

use bevy::prelude::*;
use bevy_turborand::GlobalRng;
use twilight_model::id::{marker::GuildMarker, Id};
use twilight_util::builder::embed::{EmbedBuilder, EmbedFieldBuilder, ImageSource};

use crate::{
    bundles::{Bygone03Bundle, BygoneParts, PlayerBundle},
    components::{
        Active, Attack, Bygone03Stage, BygonePart, Enemy, GuildIdComponent, Player, PlayerName,
        Ready, UserIdComponent, Vitality,
    },
    dice::{choose_mut, Dice},
    events::*,
    game_helpers::{EventDelay, Game, GameId, GameRenderMessage, GameStatus, GameTimer},
    localization::RenderText,
};

pub fn listen(
    input_receiver: Mutex<Receiver<InputEvent>>,
) -> impl FnMut(
    ResMut<HashMap<Id<GuildMarker>, Game>>,
    ResMut<HashMap<Id<GuildMarker>, Vec<String>>>,
    EventWriter<GameStartEvent>,
    EventWriter<PlayerAttackEvent>,
    EventWriter<DelayedEvent>,
    EventWriter<PlayerJoinEvent>,
    Query<(&UserIdComponent, Option<&Active>), (With<Player>,)>,
) {
    move |mut games,
          mut battle_log,
          mut ev_game_start,
          mut ev_player_attack,
          mut ev_delayed,
          mut ev_player_join,
          players| {
        let mut events = Vec::new();
        if let Ok(ref mut receiver_lock) = input_receiver.try_lock() {
            while let Ok(event) = receiver_lock.try_recv() {
                events.push(event);
            }
        }
        let events = events;

        let players: HashMap<_, _> = players.iter().collect();

        for (i, event) in events.into_iter().enumerate() {
            match event {
                InputEvent::GameStart(ev) => {
                    let should_start_new_game = match games.get(&ev.guild) {
                        Some(game) => {
                            game.status == GameStatus::Lost
                                || game.status != GameStatus::Ongoing
                                    && elapsed_since(&game.start_time) > 60 * 60
                        }
                        None => true,
                    };
                    if should_start_new_game {
                        games.insert(
                            ev.guild,
                            Game::new(
                                GameId::from_current_time(i as u128),
                                ev.localization.clone(),
                            ),
                        );
                        battle_log.remove(&ev.guild);
                        ev_game_start.send(ev.clone());
                        ev_player_join.send(PlayerJoinEvent::new(
                            ev.initial_player,
                            ev.initial_player_name,
                            ev.guild,
                        ));
                        ev_delayed.send(DelayedEvent::GameDraw(GameDrawEvent::new(ev.guild)));
                    }
                }
                InputEvent::PlayerAttack(ev) => {
                    if games.contains_key(&ev.guild) {
                        match players.get(&UserIdComponent(ev.player)) {
                            Some(maybe_active) => {
                                if let Some(_active) = maybe_active {
                                    ev_player_attack.send(ev);
                                }
                            }
                            None => {
                                ev_player_join.send(PlayerJoinEvent::new(
                                    ev.player.clone(),
                                    ev.player_name.clone(),
                                    ev.guild.clone(),
                                ));
                                ev_delayed.send(DelayedEvent::PlayerAttack(ev));
                            }
                        }
                    }
                }
            }
        }
    }
}

fn elapsed_since(system_time: &SystemTime) -> u64 {
    match system_time.elapsed() {
        Ok(dur) => dur.as_secs(),
        Err(_) => 0,
    }
}

pub fn delay_events(
    delay: Res<EventDelay>,
    mut buffer: Local<VecDeque<(Instant, DelayedEvent)>>,
    mut ev_delayed: EventReader<DelayedEvent>,
    mut ev_game_draw: EventWriter<GameDrawEvent>,
    mut ev_player_attack: EventWriter<PlayerAttackEvent>,
) {
    let ready_count = buffer
        .iter()
        .take_while(|(start, _)| start.elapsed() > delay.0)
        .count();
    for _ in 0..ready_count {
        match buffer.pop_front().unwrap().1 {
            DelayedEvent::GameDraw(ev) => ev_game_draw.send(ev),
            DelayedEvent::PlayerAttack(ev) => ev_player_attack.send(ev),
        }
    }

    for ev in ev_delayed.iter() {
        buffer.push_back((Instant::now(), ev.clone()));
    }
}

pub fn turn_timer(
    mut timers: Local<HashMap<Id<GuildMarker>, GameTimer>>,
    mut ev_player_attack: EventReader<PlayerAttackEvent>,
    mut ev_enemy_attack: EventWriter<EnemyAttackEvent>,
    mut ev_game_draw: EventWriter<GameDrawEvent>,
    mut ev_turn_end: EventWriter<TurnEndEvent>,
) {
    for (guild_id, timer) in timers.iter_mut() {
        if timer.enemy_attack() {
            ev_enemy_attack.send(EnemyAttackEvent::new(*guild_id));
        }
        if timer.turn_end() {
            ev_turn_end.send(TurnEndEvent::new(*guild_id));
            ev_game_draw.send(GameDrawEvent::new(*guild_id));
        }
    }

    timers.retain(|_, timer| !timer.depleted());

    for ev in ev_player_attack.iter() {
        timers.entry(ev.guild).or_insert(GameTimer::new());
    }
}

pub fn spawn_bygones(
    mut commands: Commands,
    mut global_rng: ResMut<GlobalRng>,
    mut ev_game_start: EventReader<GameStartEvent>,
) {
    for ev in ev_game_start.iter() {
        commands.spawn_bundle(Bygone03Bundle::with_normal_health(
            ev.guild,
            &mut global_rng,
        ));
    }
}

pub fn spawn_players(mut commands: Commands, mut ev_player_join: EventReader<PlayerJoinEvent>) {
    for ev in ev_player_join.iter() {
        commands.spawn_bundle(PlayerBundle::new(
            ev.player,
            ev.player_name.clone(),
            ev.guild,
        ));
    }
}

pub fn damage_bygone(
    mut commands: Commands,
    mut rng: ResMut<GlobalRng>,
    mut ev_player_attack: EventReader<PlayerAttackEvent>,
    mut ev_part_death: EventWriter<BygonePartDeathEvent>,
    mut ev_battle_log: EventWriter<(Id<GuildMarker>, BattleLogEvent)>,
    mut actors: ParamSet<(
        Query<
            (
                Entity,
                &UserIdComponent,
                &PlayerName,
                &GuildIdComponent,
                &Attack,
            ),
            (With<Player>, With<Active>, With<Ready>),
        >,
        Query<(Entity, &GuildIdComponent, &mut BygoneParts), (With<Enemy>, With<Active>)>,
    )>,
) {
    let target_parts: HashMap<_, _> = ev_player_attack
        .iter()
        .map(|ev| ((ev.player, ev.guild), ev.target))
        .collect();

    let attacks: HashMap<_, _> = actors
        .p0()
        .iter()
        .filter(|(_, user_id, _, guild_id, _)| target_parts.contains_key(&(user_id.0, guild_id.0)))
        .map(|(entity, user_id, user_name, guild_id, attack)| {
            (*guild_id, (entity, *user_id, user_name.clone(), *attack))
        })
        .collect();

    for (bygone_entity, enemy_guild, mut body_parts) in actors.p1().iter_mut() {
        if let Some((user_entity, user_id, user_name, attack)) = attacks.get(enemy_guild) {
            if let Some(part) = target_parts.get(&(user_id.0, enemy_guild.0)) {
                if !body_parts.0[*part].health().alive() {
                    continue;
                }
                let dice_roll = rng.d100();
                println!(
                    "Attacking bygone part, dodge {}, acc {}, roll {}",
                    body_parts.0[*part].dodge(),
                    attack.accuracy(),
                    dice_roll
                );
                if attack.attack(&mut body_parts.0[*part], dice_roll) {
                    ev_battle_log.send((
                        enemy_guild.0,
                        BattleLogEvent::PlayerHit(user_name.clone(), *part),
                    ));
                    if !body_parts.0[*part].health().alive() {
                        ev_part_death.send(BygonePartDeathEvent::new(bygone_entity, *part));
                    }
                } else {
                    ev_battle_log
                        .send((enemy_guild.0, BattleLogEvent::PlayerMiss(user_name.clone())));
                }
                commands.entity(*user_entity).remove::<Ready>();
            }
        }
    }
}

pub fn process_bygone_part_death(
    mut ev_part_death: EventReader<BygonePartDeathEvent>,
    mut ev_battle_log: EventWriter<(Id<GuildMarker>, BattleLogEvent)>,
    mut ev_deactivate: EventWriter<DeactivateEvent>,
    mut bygones: Query<
        (
            Entity,
            &GuildIdComponent,
            &mut BygoneParts,
            &mut Attack,
            &mut Bygone03Stage,
        ),
        (With<Enemy>, With<Active>),
    >,
) {
    for BygonePartDeathEvent { entity, part } in ev_part_death.iter() {
        for (bygone_entity, guild, ref mut parts, ref mut attack, ref mut stage) in
            bygones.iter_mut()
        {
            if bygone_entity != *entity {
                continue;
            }
            match part {
                BygonePart::Core => {
                    **stage = stage.next();
                    if stage.terminal() {
                        ev_deactivate.send(DeactivateEvent(bygone_entity));
                        ev_battle_log.send((guild.0, BattleLogEvent::BygoneDead));
                    } else {
                        let core_max_health = parts.0[BygonePart::Core].health().max();
                        let core_dodge = parts.0[BygonePart::Core].dodge();
                        parts.0[BygonePart::Core] = Vitality::new(core_max_health, core_dodge);
                    }
                }
                BygonePart::Sensor => {
                    attack.modify_accuracy(-40);
                }
                BygonePart::Gun => {
                    attack.modify_accuracy(-30);
                }
                BygonePart::LeftWing | BygonePart::RightWing => {
                    parts
                        .0
                        .values_mut()
                        .for_each(|vitality| vitality.modify_dodge(-10));
                }
            }
        }
    }
}

pub fn damage_players(
    mut rng: ResMut<GlobalRng>,
    mut ev_enemy_attack: EventReader<EnemyAttackEvent>,
    mut ev_battle_log: EventWriter<(Id<GuildMarker>, BattleLogEvent)>,
    mut ev_deactivate: EventWriter<DeactivateEvent>,
    mut players: Query<
        (Entity, &GuildIdComponent, &PlayerName, &mut Vitality),
        (With<Player>, With<Active>),
    >,
    enemies: Query<(&GuildIdComponent, &Attack), (With<Enemy>, With<Active>)>,
) {
    for EnemyAttackEvent { guild } in ev_enemy_attack.iter() {
        let mut players: Vec<_> = players
            .iter_mut()
            .filter(|(_, player_guild_id, _, _)| player_guild_id.0 == *guild)
            .map(|(entity, _, name, vitality)| (entity, name, vitality))
            .collect();
        let enemies = enemies
            .iter()
            .filter(|(enemy_guild_id, _)| enemy_guild_id.0 == *guild);

        for (guild_id, attack) in enemies {
            if let Some((entity, name, target)) = choose_mut(&mut rng, &mut players) {
                if attack.attack(target.deref_mut(), rng.d100()) {
                    ev_battle_log.send((guild_id.0, BattleLogEvent::BygoneHit(name.clone())));
                    if !target.health().alive() {
                        ev_deactivate.send(DeactivateEvent(*entity));
                        ev_battle_log.send((guild_id.0, BattleLogEvent::PlayerDead(name.clone())));
                    }
                } else {
                    ev_battle_log.send((guild_id.0, BattleLogEvent::BygoneMiss));
                }
            }
        }
    }
}

pub fn deactivate(mut commands: Commands, mut ev_deactivate: EventReader<DeactivateEvent>) {
    for ev in ev_deactivate.iter() {
        commands.entity(ev.0).remove::<Active>();
    }
}

pub fn update_game_status(
    mut games: ResMut<HashMap<Id<GuildMarker>, Game>>,
    mut ev_deactivate: EventReader<DeactivateEvent>,
    active_players: Query<(Entity, &GuildIdComponent), (With<Player>, With<Active>)>,
    active_enemies: Query<(Entity, &GuildIdComponent), (With<Enemy>, With<Active>)>,
    entities: Query<(Entity, &GuildIdComponent), (Or<(With<Enemy>, With<Player>)>,)>,
) {
    let deactivated: HashSet<_> = ev_deactivate
        .iter()
        .filter_map(|ev| entities.get(ev.0).ok())
        .map(|(entity, _)| entity)
        .collect();

    for (guild_id, game) in games
        .iter_mut()
        .filter(|(_, game)| game.status == GameStatus::Ongoing)
    {
        let initialized = entities
            .iter()
            .any(|(_, enemy_guild_id)| enemy_guild_id.0 == *guild_id);
        if !initialized {
            continue;
        }
        if active_enemies.iter().all(|(entity, enemy_guild_id)| {
            enemy_guild_id.0 != *guild_id || deactivated.contains(&entity)
        }) {
            game.status = GameStatus::Won;
        } else if active_players.iter().all(|(entity, player_guild_id)| {
            player_guild_id.0 != *guild_id || deactivated.contains(&entity)
        }) {
            game.status = GameStatus::Lost;
        }
    }
}

pub fn log_battle(
    games: Res<HashMap<Id<GuildMarker>, Game>>,
    mut rng: ResMut<GlobalRng>,
    mut battle_log: ResMut<HashMap<Id<GuildMarker>, Vec<String>>>,
    mut ev_battle_log: EventReader<(Id<GuildMarker>, BattleLogEvent)>,
    mut ev_player_join: EventReader<PlayerJoinEvent>,
) {
    for (guild_id, ev) in ev_battle_log.iter() {
        if let Some(game) = games.get(guild_id) {
            let loc = &game.localization;
            let log_line = match ev {
                BattleLogEvent::PlayerDead(name) => rng
                    .sample(&loc.player_dead)
                    .unwrap()
                    .insert_player_name(name),
                BattleLogEvent::PlayerHit(name, part) => rng
                    .sample(&loc.player_hit)
                    .unwrap()
                    .insert_player_name(name)
                    .insert_bygone_part_name(&part.render_text(loc)),
                BattleLogEvent::PlayerMiss(name) => rng
                    .sample(&loc.player_miss)
                    .unwrap()
                    .insert_player_name(name),
                BattleLogEvent::BygoneHit(name) => rng
                    .sample(&loc.bygone03_hit)
                    .unwrap()
                    .insert_player_name(name)
                    .insert_enemy_name("_03"),
                BattleLogEvent::BygoneMiss => rng
                    .sample(&loc.bygone03_miss)
                    .unwrap()
                    .insert_enemy_name("_03"),
                BattleLogEvent::BygoneDead => rng
                    .sample(&loc.bygone03_dead)
                    .unwrap()
                    .insert_enemy_name("_03"),
            };
            battle_log
                .entry(*guild_id)
                .or_insert(Vec::new())
                .push(log_line.0);
        }
    }
    for ev in ev_player_join.iter() {
        if let Some(game) = games.get(&ev.guild) {
            let loc = &game.localization;
            let log_line = rng
                .sample(&loc.player_join)
                .unwrap()
                .insert_player_name(&ev.player_name);
            battle_log
                .entry(ev.guild)
                .or_insert(Vec::new())
                .push(log_line.0);
        }
    }
}

pub fn render(
    sender: Mutex<Sender<GameRenderMessage>>,
) -> impl FnMut(
    Res<HashMap<Id<GuildMarker>, Game>>,
    ResMut<HashMap<Id<GuildMarker>, Vec<String>>>,
    EventReader<GameDrawEvent>,
    Query<(&PlayerName, &GuildIdComponent, &Vitality), (With<Player>,)>,
    Query<(&GuildIdComponent, &BygoneParts, &Attack, &Bygone03Stage), (With<Enemy>,)>,
) {
    move |games, mut battle_log, mut ev_game_draw, players, enemies| {
        for GameDrawEvent { guild_id } in ev_game_draw.iter() {
            if let Some(game) = games.get(guild_id) {
                let loc = &game.localization;
                let mut message = GameRenderMessage::new(*guild_id, game.game_id);
                message.embeds.title = Some(match game.status {
                    GameStatus::Ongoing => EmbedBuilder::new()
                        .description(&loc.title)
                        .image(
                            ImageSource::url(
                                "http://www.uof7.com/wp-content/uploads/2016/09/15-Bygone-UPD.gif",
                            )
                            .unwrap(),
                        )
                        .build(),
                    GameStatus::Won => EmbedBuilder::new().description(&loc.won).build(),
                    GameStatus::Lost => EmbedBuilder::new().description(&loc.lost).build(),
                });
                if game.status == GameStatus::Ongoing {
                    for (enemy_guild_id, BygoneParts(parts), attack, stage) in enemies.iter() {
                        if enemy_guild_id.0 != *guild_id {
                            continue;
                        }
                        let loc = &game.localization;
                        let status = format!(
                            " • {}\n • {}: {}",
                            attack.render_text(loc),
                            &loc.core,
                            stage.render_text(loc)
                        );
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
                                .field(
                                    EmbedFieldBuilder::new(&loc.left_wing_title, left_wing)
                                        .inline(),
                                )
                                .field(
                                    EmbedFieldBuilder::new(&loc.right_wing_title, right_wing)
                                        .inline(),
                                )
                                .field(EmbedFieldBuilder::new(&loc.gun_title, gun).inline())
                                .build(),
                        );
                    }

                    if let Some(battle_log_lines) = battle_log.remove(guild_id) {
                        let battle_log_contents =
                            " • ".to_string() + &battle_log_lines.join("\n • ");
                        message.embeds.log = Some(
                            EmbedBuilder::new()
                                .field(EmbedFieldBuilder::new(&loc.log_title, battle_log_contents))
                                .build(),
                        );
                    }

                    let mut players_embed_builder = EmbedBuilder::new();
                    for (name, player_guild_id, vitality) in players.iter() {
                        if player_guild_id.0 != *guild_id {
                            continue;
                        }
                        players_embed_builder = players_embed_builder.field(
                            EmbedFieldBuilder::new(&name.0, vitality.health().render_text(loc)),
                        );
                    }
                    let players_embed = players_embed_builder.build();
                    if players_embed.fields.len() > 0 {
                        message.embeds.players = Some(players_embed);
                    }
                }

                if let Ok(ref mut sender_lock) = sender.lock() {
                    sender_lock.send(message);
                }
            }
        }
    }
}

pub fn ready_players(
    mut commands: Commands,
    mut ev_turn_end: EventReader<TurnEndEvent>,
    players: Query<(Entity, &GuildIdComponent), (With<Player>, With<Active>, Without<Ready>)>,
) {
    for ev in ev_turn_end.iter() {
        for (entity, guild_id) in players.iter() {
            if guild_id.0 == ev.guild_id {
                commands.entity(entity).insert(Ready);
            }
        }
    }
}

pub fn cleanup(
    mut commands: Commands,
    games: Res<HashMap<Id<GuildMarker>, Game>>,
    entities: Query<(Entity, &GuildIdComponent)>,
) {
    for (entity, guild_id) in entities.iter() {
        if let Some(game) = games.get(&guild_id.0) {
            if game.status != GameStatus::Ongoing {
                commands.entity(entity).despawn();
            }
        }
    }
}

pub fn save_games(
    sender: Mutex<Sender<HashMap<Id<GuildMarker>, Game>>>,
) -> impl FnMut(Res<HashMap<Id<GuildMarker>, Game>>) {
    move |games| {
        if let Ok(ref mut sender_lock) = sender.lock() {
            sender_lock.send(games.clone());
        }
    }
}

// pub fn save_scoreboard(sender: Mutex<Sender<HashMap::<GuildId, HashMap<Id<UserMarker>, usize>>>>) -> impl FnMut(
//     Res<HashMap::<GuildId, HashMap<Id<UserMarker>, usize>>>,
// ) {
//     move |scoreboard| {
//         if let Ok(ref mut sender_lock) = sender.lock() {
//             sender_lock.send(scoreboard.clone());
//         }
//     }
// }
