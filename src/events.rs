use bevy::prelude::*;
use twilight_model::id::{UserId, ChannelId};

use crate::{components::BygonePart, localization::Localization};

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct DeactivateEvent(pub Entity);

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct BygonePartDeathEvent {
    pub entity: Entity,
    pub part: BygonePart,
}

impl BygonePartDeathEvent {
    pub fn new(entity: Entity, part: BygonePart) -> Self {
        Self {
            entity,
            part,
        }
    }
}

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct PlayerAttackEvent {
    pub player: UserId,
    pub player_name: String,
    pub channel: ChannelId,
    pub target: BygonePart,
}

impl PlayerAttackEvent {
    pub fn new(player: UserId, player_name: String, channel: ChannelId, target: BygonePart) -> Self {
        Self {
            player,
            player_name,
            channel,
            target,
        }
    }
}

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct EnemyAttackEvent {
    pub channel: ChannelId,
}

impl EnemyAttackEvent {
    pub fn new(channel: ChannelId) -> Self {
        Self {
            channel,
        }
    }
}

#[derive(Clone, Debug)]
pub struct GameStartEvent {
    pub initial_player: UserId,
    pub initial_player_name: String,
    pub channel: ChannelId,
    pub localization: Localization,
}

impl GameStartEvent {
    pub fn new(
        initial_player: UserId,
        initial_player_name: String,
        channel: ChannelId,
        localization: Localization,
    ) -> Self {
        Self {
            initial_player,
            initial_player_name,
            channel,
            localization,
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct GameDrawEvent {
    pub channel_id: ChannelId,
}

impl GameDrawEvent {
    pub fn new(channel_id: ChannelId) -> Self {
        Self {
            channel_id,
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct TurnEndEvent {
    pub channel_id: ChannelId,
}

impl TurnEndEvent {
    pub fn new(channel_id: ChannelId) -> Self {
        Self {
            channel_id,
        }
    }
}

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct PlayerJoinEvent {
    pub player: UserId,
    pub player_name: String,
    pub channel: ChannelId,
}

impl PlayerJoinEvent {
    pub fn new(player: UserId, player_name: String, channel: ChannelId) -> Self {
        Self {
            player,
            player_name,
            channel,
        }
    }
}

#[derive(Clone, Debug)]
pub enum InputEvent {
    GameStart(GameStartEvent),
    PlayerAttack(PlayerAttackEvent),
}

#[derive(Clone, Debug)]
pub enum DelayedEvent {
    GameDraw(GameDrawEvent),
    PlayerAttack(PlayerAttackEvent),
}
