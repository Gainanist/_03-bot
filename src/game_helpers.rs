use std::{time::{Duration, Instant, SystemTime}, path::PathBuf, env, ops::Sub};

use arrayvec::ArrayVec;
use enum_map::Enum;
use serde::{Deserialize, Serialize};
use twilight_model::{channel::embed::Embed, id::{Id, marker::GuildMarker}};

use crate::localization::Localization;

const fn div_ceil(a: u64, b: u64) -> u64 {
    (a + b - 1) / b
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct EventDelay(pub Duration);

#[derive(Clone, Debug)]
pub struct GameTimer {
    start: Instant,
    enemy_attacked: bool,
    turn_ended: bool,
    progress_bar_ticks: u64,
}

impl GameTimer {
    const TURN_DURATION_SECONDS: u64 = 10;
    const PROGRESS_BAR_TICKS_COUNT: u64 = 5;
    const PROGRESS_BAR_TICK_DURATION_SECS: u64 = div_ceil(Self::TURN_DURATION_SECONDS, Self::PROGRESS_BAR_TICKS_COUNT);

    const TURN_DURATION: Duration = Duration::from_secs(Self::TURN_DURATION_SECONDS);
    const ENEMY_ATTACK_DELAY: Duration = Duration::from_millis(Self::TURN_DURATION_SECONDS*1000 - 500);

    pub fn new() -> Self {
        Self {
            start: Instant::now(),
            enemy_attacked: false,
            turn_ended: false,
            progress_bar_ticks: 0,
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

    pub fn progress_bar_tick(&mut self) -> Option<f32> {
        if self.turn_ended && self.progress_bar_ticks < Self::PROGRESS_BAR_TICKS_COUNT {
            self.progress_bar_ticks = Self::PROGRESS_BAR_TICKS_COUNT;
            Some(1.0)
        } else {
            let next_tick = Self::PROGRESS_BAR_TICK_DURATION_SECS * (self.progress_bar_ticks + 1);
            let cur_progress = self.start.elapsed().as_secs();

            if cur_progress > next_tick {
                self.progress_bar_ticks = cur_progress / Self::PROGRESS_BAR_TICK_DURATION_SECS;
                Some(cur_progress as f32 / Self::TURN_DURATION_SECONDS as f32)
            } else {
                None
            }
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Enum, Eq, Hash, PartialEq, Serialize)]
pub enum GameStatus {
    Ongoing,
    Won,
    Lost,
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
pub struct GameEmbeds {
    pub title: Option<Embed>,
    pub enemies: Option<Embed>,
    pub log: Option<Embed>,
    pub players: Option<Embed>,
}

impl GameEmbeds {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn render(self) -> ArrayVec<Embed, 4> {
        let mut embeds = ArrayVec::new();
        if let Some(title) = self.title {
            embeds.push(title);
        }
        if let Some(enemies) = self.enemies {
            embeds.push(enemies);
        }
        if let Some(log) = self.log {
            embeds.push(log);
        }
        if let Some(players) = self.players {
            embeds.push(players);
        }
        embeds
    }
}

#[derive(Clone, Debug)]
pub struct GameRenderMessage {
    pub guild_id: Id<GuildMarker>,
    pub game_id: GameId,
    pub embeds: GameEmbeds,
}

impl GameRenderMessage {
    pub fn new(guild_id: Id<GuildMarker>, game_id: GameId) -> Self {
        Self {
            guild_id,
            game_id,
            embeds: GameEmbeds::new(),
        }
    }
}
