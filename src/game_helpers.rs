use std::{
    time::{Duration, Instant, SystemTime},
};

use arrayvec::ArrayVec;
use derive_new::new;
use enum_map::Enum;
use serde::{Deserialize, Serialize};
use twilight_model::{
    channel::embed::Embed,
    id::{marker::GuildMarker, Id}, application::component::{Component, ActionRow},
};

use crate::localization::Localization;

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct EventDelay(pub Duration);

#[derive(Clone, Debug)]
pub struct GameTimer {
    start: Instant,
    enemy_attacked: bool,
    turn_ended: bool,
}

impl GameTimer {
    const TURN_DURATION: Duration = Duration::from_secs(10);
    const ENEMY_ATTACK_DELAY: Duration = Duration::from_millis(9500);

    pub fn new() -> Self {
        Self {
            start: Instant::now(),
            enemy_attacked: false,
            turn_ended: false,
        }
    }

    pub fn depleted(&self) -> bool {
        self.enemy_attacked && self.turn_ended
    }

    pub fn enemy_attack(&mut self) -> bool {
        if self.enemy_attacked || self.start.elapsed() < Self::ENEMY_ATTACK_DELAY {
            false
        } else {
            self.enemy_attacked = true;
            true
        }
    }

    pub fn turn_end(&mut self) -> bool {
        if self.turn_ended || self.start.elapsed() < Self::TURN_DURATION {
            false
        } else {
            self.turn_ended = true;
            true
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Enum, Eq, Hash, PartialEq, Serialize)]
pub enum GameStatus {
    Ongoing,
    Won,
    Lost,
}

#[derive(Clone, Copy, Debug, Deserialize, Enum, Eq, Hash, PartialEq, Serialize)]
pub enum FinishedGameStatus {
    Won,
    Lost,
}

impl From<GameStatus> for Option<FinishedGameStatus> {
    fn from(status: GameStatus) -> Self {
        match status {
            GameStatus::Ongoing => None,
            GameStatus::Won => Some(FinishedGameStatus::Won),
            GameStatus::Lost => Some(FinishedGameStatus::Lost),
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
pub struct GameId(pub u128);

impl GameId {
    pub fn from_current_time(salt: u128) -> Self {
        let timestamp = match SystemTime::now().duration_since(SystemTime::UNIX_EPOCH) {
            Ok(dur) => dur.as_nanos(),
            Err(err) => err.duration().as_nanos(),
        };
        Self(timestamp + salt)
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Game {
    pub start_time: SystemTime,
    pub game_id: GameId,
    pub localization: Localization,
    pub status: GameStatus,
}

impl Game {
    pub fn new(game_id: GameId, localization: Localization) -> Self {
        Self {
            start_time: SystemTime::now(),
            game_id,
            localization,
            status: GameStatus::Ongoing,
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct GameEmbedsBuilder {
    pub title: Option<Embed>,
    pub enemies: Option<Embed>,
    pub log: Option<Embed>,
    pub players: Option<Embed>,
    pub controls: Vec<ActionRow>,
}

impl GameEmbedsBuilder {
    fn new() -> Self {
        Self::default()
    }

    pub fn build(self, finished: bool) -> GameEmbeds {
        let mut upper_message = GameMessageEmbeds::new();
        let mut lower_message = GameMessageEmbeds::new();
        if !finished {
            if let Some(title) = self.title {
                upper_message.embeds.push(title);
            }
            if let Some(enemies) = self.enemies {
                upper_message.embeds.push(enemies);
            }

            for action_row in self.controls {
                upper_message.controls.push(Component::ActionRow(action_row));
            }

            if let Some(log) = self.log {
                lower_message.embeds.push(log);
            }
            if let Some(players) = self.players {
                lower_message.embeds.push(players);
            }
        } else {
            if let Some(title) = self.title {
                lower_message.embeds.push(title);
            }
        }
        GameEmbeds::new(upper_message.to_option(), lower_message.to_option())
    }
}

#[derive(Clone, Debug, Default)]
pub struct GameMessageEmbeds {
    pub embeds: Vec<Embed>,
    pub controls: Vec<Component>,
}

impl GameMessageEmbeds {
    fn new() -> Self {
        Self::default()
    }

    fn to_option(self) -> Option<Self> {
        if self.embeds.is_empty() && self.controls.is_empty() {
            None
        } else {
            Some(self)
        }
    }
}

#[derive(Clone, Debug, Default, new)]
pub struct GameEmbeds {
    pub upper_message: Option<GameMessageEmbeds>,
    pub lower_message: Option<GameMessageEmbeds>,
}

impl GameEmbeds {
    pub fn builder() -> GameEmbedsBuilder {
        GameEmbedsBuilder::new()
    }
}

#[derive(Clone, Debug, new)]
pub struct GameRenderMessage {
    pub guild_id: Id<GuildMarker>,
    pub game_id: GameId,
    pub embeds: GameEmbeds,
}
