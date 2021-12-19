use bevy::prelude::*;
use twilight_model::id::{UserId, MessageId};

use crate::{components::BygonePart, localization::Localization};

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct DeactivateEvent(pub Entity);

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct PlayerAttackEvent(pub UserId, pub BygonePart);

#[derive(Clone, Debug)]
pub struct GameStartEvent(pub UserId, pub Localization);

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct GameEndEvent;

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct PlayerJoinEvent(pub UserId);

#[derive(Clone, Debug)]
pub enum InputEvent {
    GameStart(GameStartEvent),
    PlayerAttack(PlayerAttackEvent),
}
