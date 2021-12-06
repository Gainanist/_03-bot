use bevy::{prelude::*};
use enum_map::{enum_map, EnumMap};

use crate::{components::*, localization::{Localization, RenderText}, dice::Dice};

#[derive(Bundle, Clone, Debug)]
pub struct Bygone03Bundle {
    parts: EnumMap<BygonePart, Vitality>,
    attack: Attack,
    stage: Bygone03Stage,
    _enemy: Enemy,
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

fn damage_players(
    mut commands: Commands,
    mut dice: Local<Dice>,
    players: Query<(Entity, &Vitality), (With<Player>,)>,
    enemies: Query<(&Attack,), (With<Enemy>,)>,
) {
    let living_players = players.iter()
        .filter(|(_, vitality)| vitality.health().alive())
        .map(|(entity, _)| entity)
        .collect::<Vec<Entity>>();

    for (&attack,) in enemies.iter() {

    }
}

impl RenderText for Bygone03Bundle {
    fn render_text(&self, localization: &Localization) -> String {
        format!(
"
{}

{}: {} - {}
{}: {}
{}: {}
{}: {}
{}: {}
",
            self.attack().render_text(localization),
            localization.core, self.core.vitality().render_text(localization), self.stage.render_text(localization),
            localization.sensor, self.sensor.vitality().render_text(localization),
            localization.left_wing, self.left_wing.vitality().render_text(localization),
            localization.right_wing, self.right_wing.vitality().render_text(localization),
            localization.gun, self.gun.vitality().render_text(localization),
        )
    }
}
