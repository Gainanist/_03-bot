use std::{ops::DerefMut, collections::HashMap};

use bevy::{prelude::*};
use enum_map::{enum_map, EnumMap};
use twilight_model::id::UserId;

use crate::{components::*, localization::{Localization, RenderText}, dice::Dice, events::*};

#[derive(Bundle, Clone, Debug)]
pub struct Bygone03Bundle {
    parts: EnumMap<BygonePart, Vitality>,
    attack: Attack,
    stage: Bygone03Stage,
    _enemy: Enemy,
    _active: Active,
}

impl Bygone03Bundle {
    pub fn new(parts_health: usize) -> Self {
        let parts = enum_map! {
            BygonePart::Core => Vitality::new(parts_health, 80),
            BygonePart::Sensor => Vitality::new(parts_health, 70),
            BygonePart::Gun => Vitality::new(parts_health, 50),
            BygonePart::LeftWing => Vitality::new(parts_health, 30),
            BygonePart::RightWing => Vitality::new(parts_health, 50),
        };
        let attack = Attack::new(1, 100);
    
        Self {
            parts,
            attack,
            stage: Bygone03Stage::Armored,
            _enemy: Enemy,
            _active: Active,
        }
    }

    pub fn with_normal_health() -> Self {
        Self::new(1)
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
    vitality: Vitality,
    attack: Attack,
    _player: Player,
    _active: Active,
}

impl PlayerBundle {
    pub fn new(user_id: UserId) -> Self {
        Self {
            user_id,
            vitality: Vitality::new(6, 0),
            attack: Attack::new(1, 0),
            _player: Player,
            _active: Active,
        }
    }

}

pub fn spawn_bygones(mut commands: Commands, mut ev_game_start: EventReader<GameStartEvent>) {
    for _ev in ev_game_start.iter() {
        commands.spawn_bundle(Bygone03Bundle::with_normal_health());
    }
}

pub fn spawn_players(mut commands: Commands, mut ev_player_join: EventReader<PlayerJoinEvent>) {
    for ev in ev_player_join.iter() {
        commands.spawn_bundle(PlayerBundle::new(ev.0));
    }
}

pub fn cleanup(
    mut commands: Commands,
    mut ev_game_end: EventReader<GameEndEvent>,
    entities: Query<(Entity,)>,
) {
    for _ev in ev_game_end.iter() {
        for (entity,) in entities.iter() {
            commands.entity(entity).despawn();
        }
    }
}

pub fn damage_bygone(
    mut ev_player_attack: EventReader<PlayerAttackEvent>,
    mut ev_deactivate: EventWriter<DeactivateEvent>,
    mut dice: Local<bevy_rng::Rng>,
    players: Query<(&UserId, &Attack), (With<Player>, With<Active>)>,
    mut enemies: Query<(Entity, &mut EnumMap<BygonePart, Vitality>, &mut Attack, &mut Bygone03Stage), (With<Enemy>, With<Active>)>,
) {
    if let Ok((
        bygone_entity,
        mut body_parts,
        mut bygone_attack,
        mut stage
    )) = enemies.single_mut() {
        let target_parts: HashMap<_, _> = ev_player_attack.iter()
            .map(|ev| (ev.0, ev.1))
            .collect();

        for (user_id, attack) in players.iter() {
            if let Some(part) = target_parts.get(user_id) {
                attack.attack(&mut body_parts[*part], dice.roll(100));
                if !body_parts[*part].health().alive() {
                    on_bygone_part_death(
                        *part,
                        body_parts.deref_mut(),
                        bygone_attack.deref_mut(),
                        stage.deref_mut(),
                    );
                    if *part == BygonePart::Core && stage.terminal() {
                        ev_deactivate.send(DeactivateEvent(bygone_entity));
                    }
                }
            }
        }
    }
}

fn on_bygone_part_death(
    dead_part: BygonePart,
    body_parts: &mut EnumMap<BygonePart, Vitality>,
    attack: &mut Attack,
    stage: &mut Bygone03Stage
) {
    match dead_part {
        BygonePart::Core => {
            let core_max_health = body_parts[BygonePart::Core].health().max();
            let core_dodge = body_parts[BygonePart::Core].dodge();
            body_parts[BygonePart::Core] = Vitality::new(core_max_health, core_dodge);
            *stage = stage.next();
        }
        BygonePart::Sensor => {
            attack.modify_accuracy(-40);
        },
        BygonePart::Gun => {
            attack.modify_accuracy(-30);
        },
        BygonePart::LeftWing | BygonePart::RightWing => {
            body_parts.values_mut()
                .for_each(|vitality| vitality.modify_dodge(-10));
        },
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

// impl RenderText for Bygone03Bundle {
//     fn render_text(&self, localization: &Localization) -> String {
//         format!(
// "
// {}

// {}: {} - {}
// {}: {}
// {}: {}
// {}: {}
// {}: {}
// ",
//             self.attack().render_text(localization),
//             localization.core, self.core.vitality().render_text(localization), self.stage.render_text(localization),
//             localization.sensor, self.sensor.vitality().render_text(localization),
//             localization.left_wing, self.left_wing.vitality().render_text(localization),
//             localization.right_wing, self.right_wing.vitality().render_text(localization),
//             localization.gun, self.gun.vitality().render_text(localization),
//         )
//     }
// }
