use bevy::prelude::*;
use twilight_model::id::{Id, marker::{GuildMarker, UserMarker}};

use crate::{components::{BygonePart, PlayerName}, localization::Localization};

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
    pub player: Id<UserMarker>,
    pub player_name: PlayerName,
    pub guild: Id<GuildMarker>,
    pub target: BygonePart,
}

impl PlayerAttackEvent {
    pub fn new(player: Id<UserMarker>, player_name: PlayerName, guild: Id<GuildMarker>, target: BygonePart) -> Self {
        Self {
            player,
            player_name,
            guild,
            target,
        }
    }
}

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct EnemyAttackEvent {
    pub guild: Id<GuildMarker>,
}

impl EnemyAttackEvent {
    pub fn new(guild: Id<GuildMarker>) -> Self {
        Self {
            guild,
        }
    }
}

#[derive(Clone, Debug)]
pub struct GameStartEvent {
    pub initial_player: Id<UserMarker>,
    pub initial_player_name: PlayerName,
    pub channel: Id<GuildMarker>,
    pub localization: Localization,
}

impl GameStartEvent {
    pub fn new(
        initial_player: Id<UserMarker>,
        initial_player_name: PlayerName,
        channel: Id<GuildMarker>,
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
    pub channel_id: Id<GuildMarker>,
}

impl GameDrawEvent {
    pub fn new(channel_id: Id<GuildMarker>) -> Self {
        Self {
            channel_id,
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct TurnEndEvent {
    pub channel_id: Id<GuildMarker>,
}

impl TurnEndEvent {
    pub fn new(channel_id: Id<GuildMarker>) -> Self {
        Self {
            channel_id,
        }
    }
}

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct PlayerJoinEvent {
    pub player: Id<UserMarker>,
    pub player_name: PlayerName,
    pub channel: Id<GuildMarker>,
}

impl PlayerJoinEvent {
    pub fn new(player: Id<UserMarker>, player_name: PlayerName, channel: Id<GuildMarker>) -> Self {
        Self {
            player,
            player_name,
            channel,
        }
    }
}

#[derive(Clone, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct EventsPlugin;

impl Plugin for EventsPlugin {
    fn build(&self, app: &mut App) {
        app
            .add_event::<(Id<GuildMarker>, BattleLogEvent)>()
            .add_event::<BygonePartDeathEvent>()
            .add_event::<DeactivateEvent>()
            .add_event::<DelayedEvent>()
            .add_event::<EnemyAttackEvent>()
            .add_event::<GameDrawEvent>()
            .add_event::<GameStartEvent>()
            .add_event::<PlayerAttackEvent>()
            .add_event::<PlayerJoinEvent>()
            .add_event::<TurnEndEvent>();
    }
}

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum BattleLogEvent {
    PlayerDead(PlayerName),
    PlayerHit(PlayerName, BygonePart),
    PlayerMiss(PlayerName),
    BygoneHit(PlayerName),
    BygoneMiss,
    BygoneDead,
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
