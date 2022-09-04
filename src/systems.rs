use std::{
    collections::{HashMap, HashSet, VecDeque},
    ops::DerefMut,
    sync::Mutex,
    time::{Duration, Instant},
};

use bevy::prelude::*;
use bevy_turborand::GlobalRng;
use crossbeam_channel::{Receiver, Sender};
use enum_map::EnumMap;
use rand::Rng;
use twilight_model::id::{marker::GuildMarker, Id};

use crate::{
    bundles::{Bygone03Bundle, BygoneParts, PlayerBundle},
    components::{
        Active, Attack, Bygone03Stage, BygonePart, Enemy, GameId, Player, PlayerName, Ready,
        UserIdComponent, Vitality,
    },
    dice::{choose_mut, Dice},
    events::*,
    game_helpers::{EventDelay, FinishedGameStatus, Game, GameStatus, GameTimer},
    localization::RenderText,
    logging::format_time,
};

const INTERACTION_TOKEN_TTL_SECS: u64 = 15 * 60;
const MAX_GAME_DURATION_SECS: u64 = INTERACTION_TOKEN_TTL_SECS - 10;
const GAME_COOLDOWN_SECONDS: u64 = INTERACTION_TOKEN_TTL_SECS - 5;

pub fn listen(
    input_receiver: Mutex<Receiver<InputEvent>>,
    game_render_sender: Mutex<Sender<GameRenderEvent>>,
) -> impl FnMut(
    ResMut<HashMap<Id<GuildMarker>, Game>>,
    ResMut<HashMap<Id<GuildMarker>, Vec<String>>>,
    EventWriter<GameStartEvent>,
    EventWriter<(GameId, PlayerAttackEvent)>,
    EventWriter<DelayedEvent>,
    EventWriter<PlayerJoinEvent>,
    EventWriter<BygoneSpawnEvent>,
    EventWriter<DeallocateGameResourcesEvent>,
    Query<(&UserIdComponent, Option<&Active>), (With<Player>,)>,
) {
    move |mut games,
          mut battle_log,
          mut ev_game_start,
          mut ev_player_attack,
          mut ev_delayed,
          mut ev_player_join,
          mut ev_bygone_spawn,
          mut ev_deallocate_game_resources,
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
                    let oneshot_type = match games.get(&ev.guild_id) {
                        Some(game) => {
                            let game_duration = game.duration_secs();
                            if game_duration < GAME_COOLDOWN_SECONDS {
                                if game.status == GameStatus::Ongoing {
                                    Some(OneshotType::OtherGameInProgress)
                                } else {
                                    Some(OneshotType::Cooldown(Duration::from_secs(
                                        GAME_COOLDOWN_SECONDS - game_duration,
                                    )))
                                }
                            } else {
                                None
                            }
                        }
                        None => None,
                    };
                    if let Some(oneshot_type) = oneshot_type {
                        if let Ok(game_render_sender_lock) = game_render_sender.lock() {
                            if let Err(err) = game_render_sender_lock.send(GameRenderEvent::new(
                                ev.guild_id,
                                ev.interaction,
                                ev.localization.clone(),
                                GameRenderPayload::OneshotMessage(oneshot_type),
                            )) {
                                println!(
                                    "{} - systems - FAILED to send render oneshot event: {}",
                                    format_time(),
                                    err
                                );
                            }
                        }
                    } else {
                        let new_game_id = GameId::from_current_time(i as u128);
                        let old_game = games.insert(
                            ev.guild_id,
                            Game::new(new_game_id, ev.interaction, ev.localization.clone()),
                        );
                        if let Some(old_game) = old_game {
                            ev_deallocate_game_resources
                                .send(DeallocateGameResourcesEvent::new(old_game.id));
                        }
                        battle_log.remove(&ev.guild_id);
                        ev_game_start.send(ev.clone());
                        ev_player_join.send(PlayerJoinEvent::new(
                            ev.initial_player,
                            ev.initial_player_name,
                            new_game_id,
                            ev.guild_id,
                        ));
                        ev_bygone_spawn.send(BygoneSpawnEvent::new(ev.difficulty, new_game_id));
                        ev_delayed.send(DelayedEvent::GameDraw(GameDrawEvent::new(ev.guild_id)));
                    }
                }
                InputEvent::PlayerAttack(ev) => {
                    if let Some(game) = games.get(&ev.guild_id) {
                        match players.get(&UserIdComponent(ev.player)) {
                            Some(maybe_active) => {
                                if let Some(_active) = maybe_active {
                                    ev_player_attack.send((game.id, ev));
                                }
                            }
                            None => {
                                ev_player_join.send(PlayerJoinEvent::new(
                                    ev.player.clone(),
                                    ev.player_name.clone(),
                                    game.id,
                                    ev.guild_id,
                                ));
                                ev_delayed.send(DelayedEvent::PlayerAttack((game.id, ev)));
                            }
                        }
                    }
                }
            }
        }
    }
}

pub fn delay_events(
    delay: Res<EventDelay>,
    mut buffer: Local<VecDeque<(Instant, DelayedEvent)>>,
    mut ev_delayed: EventReader<DelayedEvent>,
    mut ev_game_draw: EventWriter<GameDrawEvent>,
    mut ev_player_attack: EventWriter<(GameId, PlayerAttackEvent)>,
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
    mut timers: Local<HashMap<(Id<GuildMarker>, GameId), GameTimer>>,
    mut ev_player_attack: EventReader<(GameId, PlayerAttackEvent)>,
    mut ev_enemy_attack: EventWriter<EnemyAttackEvent>,
    mut ev_game_draw: EventWriter<GameDrawEvent>,
    mut ev_turn_end: EventWriter<TurnEndEvent>,
    mut ev_progress_bar_update: EventWriter<ProgressBarUpdateEvent>,
) {
    for ((guild_id, game_id), timer) in timers.iter_mut() {
        if timer.enemy_attack() {
            ev_enemy_attack.send(EnemyAttackEvent::new(*guild_id, *game_id));
        }
        if timer.turn_end() {
            ev_turn_end.send(TurnEndEvent::new(*game_id));
            ev_game_draw.send(GameDrawEvent::new(*guild_id));
        }
        if let Some(progress) = timer.progress_bar_update() {
            ev_progress_bar_update.send(ProgressBarUpdateEvent::new(*guild_id, progress));
        }
    }

    timers.retain(|_, timer| !timer.depleted());

    for (game_id, ev) in ev_player_attack.iter() {
        timers
            .entry((ev.guild_id, *game_id))
            .or_insert(GameTimer::new());
    }
}

pub fn spawn_bygones(
    mut commands: Commands,
    mut global_rng: ResMut<GlobalRng>,
    mut ev_game_start: EventReader<BygoneSpawnEvent>,
) {
    for ev in ev_game_start.iter() {
        commands.spawn_bundle(Bygone03Bundle::with_difficulty(
            ev.game_id,
            ev.difficulty,
            &mut global_rng,
        ));
    }
}

pub fn spawn_players(mut commands: Commands, mut ev_player_join: EventReader<PlayerJoinEvent>) {
    for ev in ev_player_join.iter() {
        commands.spawn_bundle(PlayerBundle::new(
            ev.player,
            ev.player_name.clone(),
            ev.game_id,
        ));
    }
}

pub fn damage_bygone(
    mut commands: Commands,
    mut rng: ResMut<GlobalRng>,
    mut ev_player_attack: EventReader<(GameId, PlayerAttackEvent)>,
    mut ev_part_death: EventWriter<BygonePartDeathEvent>,
    mut ev_battle_log: EventWriter<(Id<GuildMarker>, BattleLogEvent)>,
    mut actors: ParamSet<(
        Query<
            (Entity, &UserIdComponent, &PlayerName, &GameId, &Attack),
            (With<Player>, With<Active>, With<Ready>),
        >,
        Query<(Entity, &GameId, &mut BygoneParts), (With<Enemy>, With<Active>)>,
    )>,
) {
    let target_parts: HashMap<_, _> = ev_player_attack
        .iter()
        .map(|(game_id, ev)| ((ev.player, *game_id), (ev.guild_id, ev.target)))
        .collect();

    let attacks: HashMap<_, _> = actors
        .p0()
        .iter()
        .filter(|(_, user_id, _, game_id, _)| target_parts.contains_key(&(user_id.0, **game_id)))
        .map(|(entity, user_id, user_name, game_id, attack)| {
            (*game_id, (entity, *user_id, user_name.clone(), *attack))
        })
        .collect();

    for (bygone_entity, enemy_game_id, mut body_parts) in actors.p1().iter_mut() {
        if let Some((user_entity, user_id, user_name, attack)) = attacks.get(enemy_game_id) {
            if let Some((guild_id, part)) = target_parts.get(&(user_id.0, *enemy_game_id)) {
                if !body_parts.0[*part].health().alive() {
                    continue;
                }
                let dice_roll = rng.d100();
                println!(
                    "{} - systems - Attacking bygone part, dodge {}, acc {}, roll {}",
                    format_time(),
                    body_parts.0[*part].dodge(),
                    attack.accuracy(),
                    dice_roll
                );
                if attack.attack(&mut body_parts.0[*part], dice_roll) {
                    ev_battle_log.send((
                        *guild_id,
                        BattleLogEvent::PlayerHit(user_name.clone(), *part),
                    ));
                    if !body_parts.0[*part].health().alive() {
                        ev_part_death.send(BygonePartDeathEvent::new(
                            bygone_entity,
                            *part,
                            *guild_id,
                        ));
                    }
                } else {
                    ev_battle_log.send((*guild_id, BattleLogEvent::PlayerMiss(user_name.clone())));
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
        (Entity, &mut BygoneParts, &mut Attack, &mut Bygone03Stage),
        (With<Enemy>, With<Active>),
    >,
) {
    for BygonePartDeathEvent {
        entity,
        part,
        guild_id,
    } in ev_part_death.iter()
    {
        for (bygone_entity, ref mut parts, ref mut attack, ref mut stage) in bygones.iter_mut() {
            if bygone_entity != *entity {
                continue;
            }
            match part {
                BygonePart::Core => {
                    **stage = stage.next();
                    if stage.terminal() {
                        ev_deactivate.send(DeactivateEvent(bygone_entity));
                        ev_battle_log.send((*guild_id, BattleLogEvent::BygoneDead));
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
                    attack.modify_accuracy(-25);
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
    mut players: Query<(Entity, &GameId, &PlayerName, &mut Vitality), (With<Player>, With<Active>)>,
    enemies: Query<(&GameId, &Attack), (With<Enemy>, With<Active>)>,
) {
    for EnemyAttackEvent { guild_id, game_id } in ev_enemy_attack.iter() {
        let mut players: Vec<_> = players
            .iter_mut()
            .filter(|(_, player_game_id, _, _)| *player_game_id == game_id)
            .map(|(entity, _, name, vitality)| (entity, name, vitality))
            .collect();
        let enemies = enemies
            .iter()
            .filter(|(enemy_game_id, _)| *enemy_game_id == game_id);

        for (_game_id, attack) in enemies {
            if let Some((entity, name, target)) = choose_mut(&mut rng, &mut players) {
                if attack.attack(target.deref_mut(), rng.d100()) {
                    ev_battle_log.send((*guild_id, BattleLogEvent::BygoneHit(name.clone())));
                    if !target.health().alive() {
                        ev_deactivate.send(DeactivateEvent(*entity));
                        ev_battle_log.send((*guild_id, BattleLogEvent::PlayerDead(name.clone())));
                    }
                } else {
                    ev_battle_log.send((*guild_id, BattleLogEvent::BygoneMiss));
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
    active_players: Query<(Entity, &GameId), (With<Player>, With<Active>)>,
    active_enemies: Query<(Entity, &GameId), (With<Enemy>, With<Active>)>,
    entities: Query<(Entity, &GameId), (Or<(With<Enemy>, With<Player>)>,)>,
) {
    let deactivated: HashSet<_> = ev_deactivate
        .iter()
        .filter_map(|ev| entities.get(ev.0).ok())
        .map(|(entity, _)| entity)
        .collect();

    for game in games
        .values_mut()
        .filter(|game| game.status == GameStatus::Ongoing)
    {
        let initialized = entities
            .iter()
            .any(|(_, enemy_game_id)| *enemy_game_id == game.id);
        if !initialized {
            continue;
        }
        if active_enemies.iter().all(|(entity, enemy_game_id)| {
            *enemy_game_id != game.id || deactivated.contains(&entity)
        }) {
            game.status = FinishedGameStatus::Won.into();
        } else if active_players.iter().all(|(entity, player_game_id)| {
            *player_game_id != game.id || deactivated.contains(&entity)
        }) {
            game.status = FinishedGameStatus::Lost.into();
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
        if let Some(game) = games.get(&ev.guild_id) {
            let loc = &game.localization;
            let log_line = rng
                .sample(&loc.player_join)
                .unwrap()
                .insert_player_name(&ev.player_name);
            battle_log
                .entry(ev.guild_id)
                .or_insert(Vec::new())
                .push(log_line.0);
        }
    }
}

pub fn render(
    sender: Mutex<Sender<GameRenderEvent>>,
) -> impl FnMut(
    Res<HashMap<Id<GuildMarker>, Game>>,
    ResMut<HashMap<Id<GuildMarker>, Vec<String>>>,
    ResMut<GlobalRng>,
    EventReader<GameDrawEvent>,
    EventReader<ProgressBarUpdateEvent>,
    Query<(&PlayerName, &GameId, &Vitality), (With<Player>,)>,
    Query<(&GameId, &BygoneParts, &Attack, &Bygone03Stage), (With<Enemy>,)>,
) {
    move |games,
          mut battle_log,
          _rng,
          mut ev_game_draw,
          mut ev_progress_bar_update,
          all_players,
          enemies| {
        for ProgressBarUpdateEvent { guild_id, progress } in ev_progress_bar_update.iter() {
            if let Some(game) = games.get(guild_id) {
                if let Ok(ref mut sender_lock) = sender.lock() {
                    if let Err(err) = sender_lock.send(GameRenderEvent {
                        guild_id: *guild_id,
                        interaction_id: game.interaction_id,
                        loc: game.localization.clone(),
                        payload: GameRenderPayload::TurnProgress(*progress),
                    }) {
                        println!(
                            "{} - systems - FAILED to send render progressbar event: {}",
                            format_time(),
                            err
                        );
                    }
                }
            }
        }
        for GameDrawEvent { guild_id } in ev_game_draw.iter() {
            if let Some(game) = games.get(guild_id) {
                let game_render_ev = if let GameStatus::Finished(finished_status) = game.status {
                    GameRenderEvent {
                        guild_id: *guild_id,
                        interaction_id: game.interaction_id,
                        loc: game.localization.clone(),
                        payload: GameRenderPayload::FinishedGame(finished_status),
                    }
                } else {
                    let mut bygone_attack = Attack::default();
                    let mut bygone_parts = EnumMap::<BygonePart, Vitality>::default();
                    let mut bygone_stage = Bygone03Stage::Armored;
                    for (enemy_game_id, BygoneParts(parts), attack, stage) in enemies.iter() {
                        if *enemy_game_id != game.id {
                            continue;
                        }
                        bygone_attack = *attack;
                        bygone_parts = *parts;
                        bygone_stage = *stage;
                    }

                    let battle_log_lines = battle_log.remove(guild_id).unwrap_or_default();

                    let mut players = Vec::new();
                    for (name, player_game_id, vitality) in all_players.iter() {
                        if *player_game_id != game.id {
                            continue;
                        }
                        players.push((name.clone(), *vitality));
                    }

                    GameRenderEvent {
                        guild_id: *guild_id,
                        interaction_id: game.interaction_id,
                        loc: game.localization.clone(),
                        payload: GameRenderPayload::OngoingGame(OngoingGamePayload {
                            bygone_parts,
                            bygone_attack,
                            bygone_stage,
                            battle_log_lines,
                            players,
                        }),
                    }
                };

                if let Ok(sender_lock) = sender.lock() {
                    if let Err(err) = sender_lock.send(game_render_ev) {
                        println!(
                            "{} - systems - FAILED to send render game event: {}",
                            format_time(),
                            err
                        );
                    }
                }
            }
        }
    }
}

pub fn ready_players(
    mut commands: Commands,
    mut ev_turn_end: EventReader<TurnEndEvent>,
    players: Query<(Entity, &GameId), (With<Player>, With<Active>, Without<Ready>)>,
) {
    for ev in ev_turn_end.iter() {
        for (entity, game_id) in players.iter() {
            if *game_id == ev.game_id {
                commands.entity(entity).insert(Ready);
            }
        }
    }
}

pub fn cleanup(
    mut commands: Commands,
    mut games: ResMut<HashMap<Id<GuildMarker>, Game>>,
    mut ev_deallocate_game_resources: EventReader<DeallocateGameResourcesEvent>,
    mut ev_delayed: EventWriter<DelayedEvent>,
    entities: Query<(Entity, &GameId)>,
) {
    for (guild_id, game) in games.iter_mut() {
        if game.status == GameStatus::Ongoing && game.duration_secs() >= MAX_GAME_DURATION_SECS {
            game.status = FinishedGameStatus::Expired.into();
            ev_delayed.send(DelayedEvent::GameDraw(GameDrawEvent::new(*guild_id)));
        }
    }
    for ev in ev_deallocate_game_resources.iter() {
        for (entity, game_id) in entities.iter() {
            if *game_id == ev.game_id {
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
            if let Err(err) = sender_lock.send(games.clone()) {
                println!(
                    "{} - systems - FAILED to send save games event: {}",
                    format_time(),
                    err
                );
            }
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
