

use bevy::{prelude::*};
use bevy_turborand::GlobalRng;
use enum_map::{enum_map, EnumMap};
use rand::{Rng};

use crate::{components::*};

#[derive(Clone, Copy, Component, Debug, Eq, Hash, PartialEq)]
pub struct BygoneParts(pub EnumMap<BygonePart, Vitality>);

#[derive(Bundle, Clone, Debug)]
pub struct Bygone03Bundle {
    guild: GuildIdComponent,
    parts: BygoneParts,
    attack: Attack,
    attack_dice: AttackDice,
    stage: Bygone03Stage,
    _enemy: Enemy,
    _active: Active,
}

impl Bygone03Bundle {
    pub fn new(
        min_parts_health: usize,
        max_parts_health: usize,
        min_attack: usize,
        max_attack: usize,
        guild: GuildIdComponent,
        rng: &mut GlobalRng,
    ) -> Self {
        let parts = BygoneParts(enum_map! {
            BygonePart::Core => Vitality::new(rng.gen_range(min_parts_health..=max_parts_health), 80),
            BygonePart::Sensor => Vitality::new(rng.gen_range(min_parts_health..=max_parts_health), 70),
            BygonePart::Gun => Vitality::new(rng.gen_range(min_parts_health..=max_parts_health), 50),
            BygonePart::LeftWing => Vitality::new(rng.gen_range(min_parts_health..=max_parts_health), 30),
            BygonePart::RightWing => Vitality::new(rng.gen_range(min_parts_health..=max_parts_health), 30),
        });
        let attack = Attack::new(rng.gen_range(min_attack..=max_attack), 100);
        let attack_dice = AttackDice::new_d100(rng);
    
        Self {
            guild,
            parts,
            attack,
            attack_dice,
            stage: Bygone03Stage::Armored,
            _enemy: Enemy,
            _active: Active,
        }
    }

    pub fn with_normal_health(guild: GuildIdComponent, rng: &mut GlobalRng) -> Self {
        Self::new(1, 3, 1, 3, guild, rng)
    }
}

#[derive(Bundle, Clone, Debug)]
pub struct PlayerBundle {
    user_id: UserIdComponent,
    name: PlayerName,
    guild: GuildIdComponent,
    vitality: Vitality,
    attack: Attack,
    _player: Player,
    _active: Active,
    _ready: Ready,
}

impl PlayerBundle {
    pub fn new(user_id: UserIdComponent, name: PlayerName, guild: GuildIdComponent) -> Self {
        Self {
            user_id,
            name,
            guild,
            vitality: Vitality::new(6, 50),
            attack: Attack::new(1, 50),
            _player: Player,
            _active: Active,
            _ready: Ready,
        }
    }
}


