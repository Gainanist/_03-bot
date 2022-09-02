use std::ops::RangeBounds;

use bevy::prelude::*;
use bevy_turborand::GlobalRng;
use enum_map::{enum_map, EnumMap};

use twilight_model::id::{
    marker::{GuildMarker, UserMarker},
    Id,
};

use crate::{components::*, game_helpers::Difficulty};

#[derive(Clone, Copy, Component, Debug, Eq, Hash, PartialEq)]
pub struct BygoneParts(pub EnumMap<BygonePart, Vitality>);

#[derive(Bundle, Clone, Debug)]
pub struct Bygone03Bundle {
    guild: GuildIdComponent,
    parts: BygoneParts,
    attack: Attack,
    stage: Bygone03Stage,
    _enemy: Enemy,
    _active: Active,
}

impl Bygone03Bundle {
    pub fn new(
        parts_health_range: impl RangeBounds<usize> + Clone,
        attack_range: impl RangeBounds<usize> + Clone,
        guild: Id<GuildMarker>,
        rng: &mut GlobalRng,
    ) -> Self {
        let wings_hp = rng.usize(parts_health_range.clone());
        let parts = BygoneParts(enum_map! {
            BygonePart::Core => Vitality::new(rng.usize(parts_health_range.clone()), 70),
            BygonePart::Sensor => Vitality::new(rng.usize(parts_health_range.clone()), 80),
            BygonePart::Gun => Vitality::new(rng.usize(parts_health_range.clone()), 50),
            BygonePart::LeftWing => Vitality::new(wings_hp, 30),
            BygonePart::RightWing => Vitality::new(wings_hp, 30),
        });
        let attack = Attack::new(rng.usize(attack_range), 100);

        Self {
            guild: GuildIdComponent(guild),
            parts,
            attack,
            stage: Bygone03Stage::Armored,
            _enemy: Enemy,
            _active: Active,
        }
    }

    pub fn with_difficulty(guild: Id<GuildMarker>, difficulty: Difficulty, rng: &mut GlobalRng) -> Self {
        let parts_health_range = match difficulty {
            Difficulty::Easy => 1..=1,
            Difficulty::Medium => 1..=2,
            Difficulty::Hard => 1..=3,
            Difficulty::RealBullets => 1..=3,
        };
        let attack_range = match difficulty {
            Difficulty::Easy => 1..=1,
            Difficulty::Medium => 1..=2,
            Difficulty::Hard => 1..=3,
            Difficulty::RealBullets => 6..=6,
        };
        Self::new(parts_health_range, attack_range, guild, rng)
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
    pub fn new(user_id: Id<UserMarker>, name: PlayerName, guild: Id<GuildMarker>) -> Self {
        Self {
            user_id: UserIdComponent(user_id),
            name,
            guild: GuildIdComponent(guild),
            vitality: Vitality::new(6, 100),
            attack: Attack::new(1, 0),
            _player: Player,
            _active: Active,
            _ready: Ready,
        }
    }
}
