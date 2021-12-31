use std::{sync::{mpsc::{Receiver, Sender}, Mutex}, collections::{HashMap, VecDeque, HashSet}, time::{SystemTime, Instant}, ops::DerefMut, fs};

use bevy::prelude::*;
use enum_map::EnumMap;
use twilight_embed_builder::{EmbedBuilder, ImageSource, EmbedFieldBuilder};
use twilight_model::id::{ChannelId, UserId};

use crate::{events::*, localization::RenderText, game_helpers::{Game, GameStatus, GameId, EventDelay, GameTimer, GameRenderMessage, get_games_filename}, components::{Active, Player, Attack, Vitality, BygonePart, Enemy, Bygone03Stage, Ready}, bundles::{Bygone03Bundle, PlayerBundle}, dice::Dice};

pub fn listen(
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

pub fn delay_events(
    delay: Res<EventDelay>,
    mut buffer: Local<VecDeque<(Instant, DelayedEvent)>>,
    mut ev_delayed: EventReader<DelayedEvent>,
    mut ev_game_draw: EventWriter<GameDrawEvent>,
    mut ev_player_attack: EventWriter<PlayerAttackEvent>,
) {
    let ready_count = buffer.iter()
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
    mut timers: Local<HashMap<ChannelId, GameTimer>>,
    mut ev_player_attack: EventReader<PlayerAttackEvent>,
    mut ev_enemy_attack: EventWriter<EnemyAttackEvent>,
    mut ev_game_draw: EventWriter<GameDrawEvent>,
    mut ev_turn_end: EventWriter<TurnEndEvent>,
) {
    for (channel_id, timer) in timers.iter_mut() {
        if timer.enemy_attack() {
            ev_enemy_attack.send(EnemyAttackEvent::new(*channel_id));
        }
        if timer.turn_end() {
            ev_turn_end.send(TurnEndEvent::new(*channel_id));
            ev_game_draw.send(GameDrawEvent::new(*channel_id));
        }
    }

    timers.retain(|_, timer| !timer.depleted());

    for ev in ev_player_attack.iter() {
        timers.entry(ev.channel).or_insert(GameTimer::new());
    }
}

pub fn spawn_bygones(mut commands: Commands, mut ev_game_start: EventReader<GameStartEvent>) {
    for ev in ev_game_start.iter() {
        commands.spawn_bundle(Bygone03Bundle::with_normal_health(ev.channel));
    }
}

pub fn spawn_players(mut commands: Commands, mut ev_player_join: EventReader<PlayerJoinEvent>) {
    for ev in ev_player_join.iter() {
        commands.spawn_bundle(PlayerBundle::new(ev.player, ev.player_name.clone(), ev.channel));
    }
}

pub fn damage_bygone(
    mut commands: Commands,
    mut ev_player_attack: EventReader<PlayerAttackEvent>,
    mut ev_part_death: EventWriter<BygonePartDeathEvent>,
    mut dice: Local<bevy_rng::Rng>,
    mut actors: QuerySet<(
        Query<(Entity, &UserId, &ChannelId, &Attack), (With<Player>, With<Active>, With<Ready>)>,
        Query<(Entity, &ChannelId,  &mut EnumMap<BygonePart, Vitality>), (With<Enemy>, With<Active>)>,
    )>,
) {
    let target_parts: HashMap<_, _> = ev_player_attack.iter()
        .map(|ev| ((ev.player, ev.channel), ev.target))
        .collect();
    
    let attacks: HashMap<_, _> = actors.q0().iter()
        .filter(|(_, user_id, channel_id, _)| target_parts.contains_key(&(**user_id, **channel_id)))
        .map(|(entity, user_id, channel_id, attack)| (*channel_id, (entity, *user_id, *attack)))
        .collect();

    for (bygone_entity,
        enemy_channel,
        mut body_parts,
    ) in actors.q1_mut().iter_mut() {
        if let Some((user_entity, user_id, attack)) = attacks.get(enemy_channel) {
            if let Some(part) = target_parts.get(&(*user_id, *enemy_channel)) {
                if !body_parts[*part].health().alive() {
                    continue;
                }
                attack.attack(&mut body_parts[*part], dice.iroll(-50, 50));
                if !body_parts[*part].health().alive() {
                    ev_part_death.send(BygonePartDeathEvent::new(bygone_entity, *part));
                }
                commands.entity(*user_entity).remove::<Ready>();
            }

        }
    }
}

pub fn process_bygone_part_death(
    mut ev_part_death: EventReader<BygonePartDeathEvent>,
    mut ev_deactivate: EventWriter<DeactivateEvent>,
    mut bygones: Query<(Entity, &ChannelId,  &mut EnumMap<BygonePart, Vitality>, &mut Attack, &mut Bygone03Stage), (With<Enemy>, With<Active>)>,
) {
    for BygonePartDeathEvent { entity, part } in ev_part_death.iter() {
        for (bygone_entity,
            _channel,
            ref mut parts,
            ref mut attack,
            ref mut stage
        ) in bygones.iter_mut() {
            if bygone_entity != *entity {
                continue;
            }
            match part {
                BygonePart::Core => {
                    **stage = stage.next();
                    if stage.terminal() {
                        ev_deactivate.send(DeactivateEvent(bygone_entity));
                    } else {
                        let core_max_health = parts[BygonePart::Core].health().max();
                        let core_dodge = parts[BygonePart::Core].dodge();
                        parts[BygonePart::Core] = Vitality::new(core_max_health, core_dodge);
                    }
                }
                BygonePart::Sensor => {
                    attack.modify_accuracy(-40);
                },
                BygonePart::Gun => {
                    attack.modify_accuracy(-30);
                },
                BygonePart::LeftWing | BygonePart::RightWing => {
                    parts.values_mut()
                        .for_each(|vitality| vitality.modify_dodge(-10));
                },
            }
        }
    }
}

pub fn damage_players(
    mut ev_enemy_attack: EventReader<EnemyAttackEvent>,
    mut ev_deactivate: EventWriter<DeactivateEvent>,
    mut dice: Local<bevy_rng::Rng>,
    mut players: Query<(Entity, &ChannelId, &mut Vitality), (With<Player>, With<Active>)>,
    enemies: Query<(&ChannelId, &Attack), (With<Enemy>, With<Active>)>,
) {
    for EnemyAttackEvent{ channel } in ev_enemy_attack.iter() {
        let mut players: Vec<_> = players.iter_mut()
            .filter(|(_, player_channel_id, _)| *player_channel_id == channel)
            .map(|(entity, _, vitality)| (entity, vitality))
            .collect();
        let enemies = enemies.iter()
            .filter(|(enemy_channel_id, _)| *enemy_channel_id == channel)
            .map(|(_, attack)| attack);

        for attack in enemies {
            if let Some((entity, target)) = dice.choose_mut(&mut players) {
                attack.attack(target.deref_mut(), dice.iroll(-50, 50));
                if !target.health().alive() {
                    ev_deactivate.send(DeactivateEvent(*entity));
                }
            }
        }
    }
}

pub fn deactivate(
    mut commands: Commands,
    mut ev_deactivate: EventReader<DeactivateEvent>,
) {
    for ev in ev_deactivate.iter() {
        commands.entity(ev.0).remove::<Active>();
    }
}

pub fn update_game_status(
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

pub fn render(
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

pub fn ready_players(
    mut commands: Commands,
    mut ev_turn_end: EventReader<TurnEndEvent>,
    players: Query<(Entity, &ChannelId), (With<Player>, With<Active>, Without<Ready>)>,
) {
    for ev in ev_turn_end.iter() {
        for (entity, channel_id) in players.iter() {
            if *channel_id == ev.channel_id {
                commands.entity(entity).insert(Ready);
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