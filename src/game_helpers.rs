use std::time::{Duration, Instant, SystemTime};

use derive_new::new;
use enum_map::Enum;
use serde::{Deserialize, Serialize};
use twilight_model::id::{
    marker::{ApplicationMarker, InteractionMarker},
    Id,
};

use crate::localization::Localization;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Difficulty {
    Easy,
    Medium,
    Hard,
    RealBullets,
}

impl Difficulty {
    pub fn from_str(difficulty: &str) -> Option<Self> {
        match difficulty {
            "easy" => Some(Difficulty::Easy),
            "medium" => Some(Difficulty::Medium),
            "hard" => Some(Difficulty::Hard),
            "real_bullets" => Some(Difficulty::RealBullets),
            _ => None,
        }
    }
}

impl From<Difficulty> for &str {
    fn from(difficulty: Difficulty) -> Self {
        match difficulty {
            Difficulty::Easy => "easy",
            Difficulty::Medium => "medium",
            Difficulty::Hard => "hard",
            Difficulty::RealBullets => "real_bullets",
        }
    }
}

impl From<Difficulty> for String {
    fn from(difficulty: Difficulty) -> Self {
        let slice: &str = difficulty.into();
        slice.to_owned()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct EventDelay(pub Duration);

#[derive(Clone, Debug)]
pub struct GameTimer {
    start: Instant,
    enemy_attacked: bool,
    turn_ended: bool,
    progress_bar_ticks: u64,
}

impl GameTimer {
    const TURN_DURATION_SECS: u64 = 10;
    const PROGRESS_BAR_TICK_SECS: u64 = 2;

    const TURN_DURATION: Duration = Duration::from_secs(Self::TURN_DURATION_SECS);
    const ENEMY_ATTACK_DELAY: Duration =
        Duration::from_millis(Self::TURN_DURATION_SECS * 1000 - 500);

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
        if self.timer_finished()
            || self.enemy_attacked
            || self.start.elapsed() < Self::ENEMY_ATTACK_DELAY
        {
            false
        } else {
            self.enemy_attacked = true;
            true
        }
    }

    pub fn turn_end(&mut self) -> bool {
        if self.turn_ended || !self.timer_finished() {
            false
        } else {
            self.turn_ended = true;
            true
        }
    }

    pub fn progress_bar_update(&mut self) -> Option<f32> {
        if self.timer_finished() {
            return None;
        }
        let elapsed = self.start.elapsed().as_secs();
        let next_progress_bar_pos = ((self.progress_bar_ticks + 1) * Self::PROGRESS_BAR_TICK_SECS)
            .min(Self::TURN_DURATION_SECS);
        if elapsed >= next_progress_bar_pos {
            self.progress_bar_ticks +=
                1 + (elapsed - next_progress_bar_pos) / Self::PROGRESS_BAR_TICK_SECS;
            Some(
                (elapsed as f32 / Self::TURN_DURATION_SECS as f32)
                    .max(0.0)
                    .min(1.0),
            )
        } else {
            None
        }
    }

    fn timer_finished(&self) -> bool {
        self.start.elapsed() > Self::TURN_DURATION
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
    pub interaction_id: Id<InteractionMarker>,
    pub localization: Localization,
    pub status: GameStatus,
}

impl Game {
    pub fn new(
        game_id: GameId,
        interaction_id: Id<InteractionMarker>,
        localization: Localization,
    ) -> Self {
        Self {
            start_time: SystemTime::now(),
            game_id,
            interaction_id,
            localization,
            status: GameStatus::Ongoing,
        }
    }
}

#[derive(Clone, Debug, new)]
pub struct InteractionIds {
    pub id: Id<InteractionMarker>,
    pub app_id: Id<ApplicationMarker>,
    pub token: String,
}
