use bevy::prelude::*;
use twilight_model::id::{UserId, MessageId, ChannelId};

use crate::{components::BygonePart, localization::Localization};

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct DeactivateEvent(pub Entity);

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct PlayerAttackEvent {
    pub player: UserId,
    pub message: MessageId,
    pub channel: ChannelId,
    pub target: BygonePart,
}

impl PlayerAttackEvent {
    pub fn new(player: UserId, message: MessageId, channel: ChannelId, target: BygonePart) -> Self {
        Self {
            player,
            message,
            channel,
            target,
        }
    }
}

#[derive(Clone, Debug)]
pub struct GameStartEvent {
    pub initial_player: UserId,
    pub message: MessageId,
    pub channel: ChannelId,
    pub localization: Localization,
}

impl GameStartEvent {
    pub fn new(initial_player: UserId, message: MessageId, channel: ChannelId, localization: Localization) -> Self {
        Self {
            initial_player,
            message,
            channel,
            localization,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct GameEndEvent;

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct PlayerJoinEvent {
    pub player: UserId,
    pub channel: ChannelId,
}

impl PlayerJoinEvent {
    pub fn new(player: UserId, channel: ChannelId) -> Self {
        Self {
            player,
            channel,
        }
    }
}

#[derive(Clone, Debug)]
pub enum InputEvent {
    GameStart(GameStartEvent),
    PlayerAttack(PlayerAttackEvent),
}
