use std::{ops::DerefMut, collections::{HashMap, VecDeque}, time::{Duration, Instant}};

use bevy::{prelude::*};
use enum_map::{enum_map, EnumMap};
use twilight_model::id::{UserId, ChannelId};

use crate::{components::*, dice::Dice, events::*};

#[derive(Bundle, Clone, Debug)]
pub struct Bygone03Bundle {
    channel: ChannelId,
    parts: EnumMap<BygonePart, Vitality>,
    attack: Attack,
    stage: Bygone03Stage,
    _enemy: Enemy,
    _active: Active,
}

impl Bygone03Bundle {
    pub fn new(parts_health: usize, channel: ChannelId) -> Self {
        let parts = enum_map! {
            BygonePart::Core => Vitality::new(parts_health, 80),
            BygonePart::Sensor => Vitality::new(parts_health, 70),
            BygonePart::Gun => Vitality::new(parts_health, 50),
            BygonePart::LeftWing => Vitality::new(parts_health, 30),
            BygonePart::RightWing => Vitality::new(parts_health, 30),
        };
        let attack = Attack::new(1, 100);
    
        Self {
            channel,
            parts,
            attack,
            stage: Bygone03Stage::Armored,
            _enemy: Enemy,
            _active: Active,
        }
    }

    pub fn with_normal_health(channel: ChannelId) -> Self {
        Self::new(1, channel)
    }
}

#[derive(Bundle, Clone, Debug)]
pub struct PlayerBundle {
    user_id: UserId,
    name: String,
    channel: ChannelId,
    vitality: Vitality,
    attack: Attack,
    _player: Player,
    _active: Active,
}

impl PlayerBundle {
    pub fn new(user_id: UserId, name: String, channel: ChannelId) -> Self {
        Self {
            user_id,
            name,
            channel,
            vitality: Vitality::new(6, 50),
            attack: Attack::new(1, 50),
            _player: Player,
            _active: Active,
        }
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
    mut ev_player_attack: EventReader<PlayerAttackEvent>,
    mut ev_part_death: EventWriter<BygonePartDeathEvent>,
    mut dice: Local<bevy_rng::Rng>,
    mut actors: QuerySet<(
        Query<(&UserId, &ChannelId, &Attack), (With<Player>, With<Active>)>,
        Query<(Entity, &ChannelId,  &mut EnumMap<BygonePart, Vitality>), (With<Enemy>, With<Active>)>,
    )>,
) {
    let target_parts: HashMap<_, _> = ev_player_attack.iter()
        .map(|ev| ((ev.player, ev.channel), ev.target))
        .collect();
    
    let attacks: HashMap<_, _> = actors.q0().iter()
        .map(|(user_id, channel_id, attack)| (*channel_id, (*user_id, *attack)))
        .collect();

    for (bygone_entity,
        enemy_channel,
        mut body_parts,
    ) in actors.q1_mut().iter_mut() {
        if let Some((user_id, attack)) = attacks.get(enemy_channel) {
            if let Some(part) = target_parts.get(&(*user_id, *enemy_channel)) {
                if !body_parts[*part].health().alive() {
                    continue;
                }
                attack.attack(&mut body_parts[*part], dice.iroll(-50, 50));
                if !body_parts[*part].health().alive() {
                    ev_part_death.send(BygonePartDeathEvent::new(bygone_entity, *part));
                }
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

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct EventDelay(pub Duration);

pub fn deactivate(
    mut commands: Commands,
    mut ev_deactivate: EventReader<DeactivateEvent>,
) {
    for ev in ev_deactivate.iter() {
        commands.entity(ev.0).remove::<Active>();
    }
}
