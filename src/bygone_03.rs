use std::{ops::DerefMut, collections::HashMap};

use bevy::{prelude::*};
use enum_map::{enum_map, EnumMap};
use twilight_model::id::{UserId, ChannelId};

use crate::{components::*, localization::{Localization, RenderText}, dice::Dice, events::*};

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
            BygonePart::RightWing => Vitality::new(parts_health, 50),
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

    // pub fn alive(&self) -> bool {
    //     !self.stage.terminal()
    // }

    // pub fn attack(&self) -> &Attack {
    //     &self.core.attack()
    // }

    // pub fn damage_core(&mut self, attack: &Attack, rng: &mut Random) {
    //     if self.try_destroy_part(&mut |bygone: &mut Bygone03| bygone.core.vitality_mut(), attack, rng) {
    //         self.advance_stage();
    //     }
    // }

    // fn advance_stage(&mut self) {
    //     self.core = Entity::aggressive(
    //         self.core.vitality().health().max(),
    //         self.core.vitality().dodge(),
    //         self.core.attack().damage(),
    //         self.core.attack().accuracy(),
    //     );
    //     self.stage = self.stage.next();
    // }

    // pub fn damage_sensor(&mut self, attack: &Attack, rng: &mut Random) {
    //     if self.try_destroy_part(&mut |bygone: &mut Bygone03| bygone.sensor.vitality_mut(), attack, rng) {
    //         self.core.attack_mut().reduce_accuracy(40);
    //     }
    // }

    // pub fn damage_gun(&mut self, attack: &Attack, rng: &mut Random) {
    //     if self.try_destroy_part(&mut |bygone: &mut Bygone03| bygone.gun.vitality_mut(), attack, rng) {
    //         self.core.attack_mut().reduce_accuracy(30);
    //     }
    // }

    // pub fn damage_left_wing(&mut self, attack: &Attack, rng: &mut Random) {
    //     if self.try_destroy_part(&mut |bygone: &mut Bygone03| bygone.left_wing.vitality_mut(), attack, rng) {
    //         self.modify_dodge(-10)
    //     }
    // }

    // pub fn damage_right_wing(&mut self, attack: &Attack, rng: &mut Random) {
    //     if self.try_destroy_part(&mut |bygone: &mut Bygone03| bygone.right_wing.vitality_mut(), attack, rng) {
    //         self.modify_dodge(-10)
    //     }
    // }

    // fn modify_dodge(&mut self, modifier: isize) {
    //     self.core.vitality_mut().modify_dodge(modifier);
    //     self.sensor.vitality_mut().modify_dodge(modifier);
    //     self.gun.vitality_mut().modify_dodge(modifier);
    //     self.left_wing.vitality_mut().modify_dodge(modifier);
    //     self.right_wing.vitality_mut().modify_dodge(modifier);
    // }

    // fn try_destroy_part<F>(&mut self, choose_part: &mut F, attack: &Attack, rng: &mut Random) -> bool
    //     where F : Fn(&mut Bygone03)->&mut Vitality
    // {
    //     if !self.alive() {
    //         return false;
    //     }

    //     let part = choose_part(self);
    //     if !part.health().alive() {
    //         return false;
    //     }

    //     rng.collide(attack, part);

    //     !part.health().alive()
    // }

    // fn try_destroy_part1(&self, vitality: &mut Vitality, attack: &Attack, rng: &mut Random) -> bool {
    //     if self.stage.terminal() {
    //         return false;
    //     }

    //     if !vitality.health().alive() {
    //         return false;
    //     }

    //     rng.collide(attack, vitality);

    //     !vitality.health().alive()
    // }
}

#[derive(Bundle, Clone, Debug)]
pub struct PlayerBundle {
    user_id: UserId,
    channel: ChannelId,
    vitality: Vitality,
    attack: Attack,
    _player: Player,
    _active: Active,
}

impl PlayerBundle {
    pub fn new(user_id: UserId, channel: ChannelId) -> Self {
        Self {
            user_id,
            channel,
            vitality: Vitality::new(6, 0),
            attack: Attack::new(1, 0),
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
        commands.spawn_bundle(PlayerBundle::new(ev.player, ev.channel));
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
                attack.attack(&mut body_parts[*part], dice.roll(100));
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
                    let core_max_health = parts[BygonePart::Core].health().max();
                    let core_dodge = parts[BygonePart::Core].dodge();
                    parts[BygonePart::Core] = Vitality::new(core_max_health, core_dodge);
                    **stage = stage.next();
                    if stage.terminal() {
                        ev_deactivate.send(DeactivateEvent(bygone_entity));
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
    mut ev_deactivate: EventWriter<DeactivateEvent>,
    mut dice: Local<bevy_rng::Rng>,
    mut players: Query<(Entity, &mut Vitality), (With<Player>, With<Active>)>,
    enemies: Query<&Attack, (With<Enemy>, With<Active>)>,
) {
    let mut players: Vec<_> = players.iter_mut().collect();

    for attack in enemies.iter() {
        let (entity, target) = dice.choose_mut(&mut players);
        attack.attack(target.deref_mut(), dice.roll(100));
        if !target.health().alive() {
            ev_deactivate.send(DeactivateEvent(*entity));
        }
    }
}

pub fn deativate(
    mut commands: Commands,
    mut ev_deactivate: EventReader<DeactivateEvent>,
) {
    for ev in ev_deactivate.iter() {
        commands.entity(ev.0).remove::<Active>();
    }
}
