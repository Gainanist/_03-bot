use std::time::Duration;

use bevy::prelude::*;
use derive_new::new;
use enum_map::EnumMap;
use twilight_model::id::{
    marker::{GuildMarker, InteractionMarker, UserMarker},
    Id,
};

use crate::{
    components::{Attack, Bygone03Stage, BygonePart, GameId, PlayerName, Vitality},
    game_helpers::{Difficulty, FinishedGameStatus},
    localization::Localization,
};

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct DeactivateEvent(pub Entity);

#[derive(Clone, Copy, Debug, Eq, Hash, new, Ord, PartialEq, PartialOrd)]
pub struct BygonePartDeathEvent {
    pub entity: Entity,
    pub part: BygonePart,
    pub guild_id: Id<GuildMarker>,
}

#[derive(Clone, Debug, Eq, Hash, new, Ord, PartialEq, PartialOrd)]
pub struct PlayerAttackEvent {
    pub player: Id<UserMarker>,
    pub player_name: PlayerName,
    pub guild_id: Id<GuildMarker>,
    pub target: BygonePart,
}

#[derive(Clone, Debug, Eq, Hash, new, Ord, PartialEq, PartialOrd)]
pub struct EnemyAttackEvent {
    pub guild_id: Id<GuildMarker>,
    pub game_id: GameId,
}

#[derive(Clone, Debug, new)]
pub struct GameStartEvent {
    pub initial_player: Id<UserMarker>,
    pub initial_player_name: PlayerName,
    pub difficulty: Difficulty,
    pub guild_id: Id<GuildMarker>,
    pub interaction: Id<InteractionMarker>,
    pub localization: Localization,
}

#[derive(Clone, Copy, Debug, new)]
pub struct GameDrawEvent {
    pub guild_id: Id<GuildMarker>,
}
#[derive(Clone, Copy, Debug, new)]
pub struct TurnEndEvent {
    pub game_id: GameId,
}

#[derive(Clone, Copy, Debug, new)]
pub struct ProgressBarUpdateEvent {
    pub guild_id: Id<GuildMarker>,
    pub progress: f32,
}

#[derive(Clone, Debug, Eq, Hash, new, Ord, PartialEq, PartialOrd)]
pub struct PlayerJoinEvent {
    pub player: Id<UserMarker>,
    pub player_name: PlayerName,
    pub game_id: GameId,
    pub guild_id: Id<GuildMarker>,
}

#[derive(Clone, Debug, Eq, Hash, new, Ord, PartialEq, PartialOrd)]
pub struct BygoneSpawnEvent {
    pub difficulty: Difficulty,
    pub game_id: GameId,
}

#[derive(Clone, Debug, Eq, Hash, new, Ord, PartialEq, PartialOrd)]
pub struct DeallocateGameResourcesEvent {
    pub game_id: GameId,
}

#[derive(Clone, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct EventsPlugin;

impl Plugin for EventsPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<(Id<GuildMarker>, BattleLogEvent)>()
            .add_event::<BygonePartDeathEvent>()
            .add_event::<DeactivateEvent>()
            .add_event::<DelayedEvent>()
            .add_event::<EnemyAttackEvent>()
            .add_event::<GameDrawEvent>()
            .add_event::<GameStartEvent>()
            .add_event::<(GameId, PlayerAttackEvent)>()
            .add_event::<PlayerJoinEvent>()
            .add_event::<BygoneSpawnEvent>()
            .add_event::<DeallocateGameResourcesEvent>()
            .add_event::<TurnEndEvent>()
            .add_event::<ProgressBarUpdateEvent>();
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
    PlayerAttack((GameId, PlayerAttackEvent)),
}

#[derive(Clone, Debug)]
pub struct OngoingGamePayload {
    pub bygone_parts: EnumMap<BygonePart, Vitality>,
    pub bygone_attack: Attack,
    pub bygone_stage: Bygone03Stage,
    pub battle_log_lines: Vec<String>,
    pub players: Vec<(PlayerName, Vitality)>,
}

#[derive(Clone, Copy, Debug)]
pub enum OneshotType {
    Cooldown(Duration),
    OtherGameInProgress,
}

#[derive(Clone, Debug)]
pub enum GameRenderPayload {
    OngoingGame(OngoingGamePayload),
    FinishedGame(FinishedGameStatus),
    TurnProgress(f32),
    OneshotMessage(OneshotType),
}

#[derive(Clone, Debug, new)]
pub struct GameRenderEvent {
    pub guild_id: Id<GuildMarker>,
    pub interaction_id: Id<InteractionMarker>,
    pub loc: Localization,
    pub payload: GameRenderPayload,
}
