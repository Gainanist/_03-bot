use bevy::prelude::*;
use twilight_model::id::UserId;

use crate::components::BygonePart;

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct DeactivateEvent(pub Entity);

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct PlayerAttackEvent(pub UserId, pub BygonePart);

