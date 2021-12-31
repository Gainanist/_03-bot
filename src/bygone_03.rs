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


