

use bevy::{prelude::*};
use enum_map::{enum_map, EnumMap};
use rand::{distributions::uniform::SampleRange, Rng};
use twilight_model::id::{UserId, ChannelId};

use crate::{components::*, dice::Dice};

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
    pub fn new(
        min_parts_health: usize,
        max_parts_health: usize,
        min_attack: usize,
        max_attack: usize,
        channel: ChannelId,
        rng: &mut bevy_rng::Rng,
    ) -> Self {
        let parts = enum_map! {
            BygonePart::Core => Vitality::new(rng.gen_range(min_parts_health..=max_parts_health), 80),
            BygonePart::Sensor => Vitality::new(rng.gen_range(min_parts_health..=max_parts_health), 70),
            BygonePart::Gun => Vitality::new(rng.gen_range(min_parts_health..=max_parts_health), 50),
            BygonePart::LeftWing => Vitality::new(rng.gen_range(min_parts_health..=max_parts_health), 30),
            BygonePart::RightWing => Vitality::new(rng.gen_range(min_parts_health..=max_parts_health), 30),
        };
        let attack = Attack::new(rng.gen_range(min_attack..=max_attack), 100);
    
        Self {
            channel,
            parts,
            attack,
            stage: Bygone03Stage::Armored,
            _enemy: Enemy,
            _active: Active,
        }
    }

    pub fn with_normal_health(channel: ChannelId, rng: &mut bevy_rng::Rng) -> Self {
        Self::new(1, 3, 1, 3, channel, rng)
    }
}

#[derive(Bundle, Clone, Debug)]
pub struct PlayerBundle {
    user_id: UserId,
    name: PlayerName,
    channel: ChannelId,
    vitality: Vitality,
    attack: Attack,
    _player: Player,
    _active: Active,
    _ready: Ready,
}

impl PlayerBundle {
    pub fn new(user_id: UserId, name: PlayerName, channel: ChannelId) -> Self {
        Self {
            user_id,
            name,
            channel,
            vitality: Vitality::new(6, 50),
            attack: Attack::new(1, 50),
            _player: Player,
            _active: Active,
            _ready: Ready,
        }
    }
}


