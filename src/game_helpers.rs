use std::{
    time::{Duration, Instant, SystemTime},
};

use arrayvec::ArrayVec;
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
    pub controls: Vec<Component>,
}

impl GameEmbedsBuilder {
    fn new() -> Self {
        Self::default()
    }

    pub fn build(self, finished: bool) -> GameEmbeds {
        let mut embeds = GameEmbeds::new();
        if !finished {
            if let Some(title) = self.title {
                embeds.upper_embeds.push(title);
            }
            if let Some(enemies) = self.enemies {
                embeds.upper_embeds.push(enemies);
            }

            if self.controls.len() > 0 {
                embeds.controls.push(
                    Component::ActionRow(ActionRow { components: self.controls  })
                );
            }

            if let Some(log) = self.log {
                embeds.lower_embeds.push(log);
            }
            if let Some(players) = self.players {
                embeds.lower_embeds.push(players);
            }
        } else {
            if let Some(title) = self.title {
                embeds.lower_embeds.push(title);
            }
        }
        embeds
    }
}

#[derive(Clone, Debug, Default)]
pub struct GameEmbeds {
    pub upper_embeds: ArrayVec<Embed, 2>,
    pub controls: ArrayVec<Component, 1>,
    pub lower_embeds: ArrayVec<Embed, 2>,
}

impl GameEmbeds {
    fn new() -> Self {
        Self::default()
    }

    pub fn builder() -> GameEmbedsBuilder {
        GameEmbedsBuilder::new()
    }
}

#[derive(Clone, Debug)]
pub struct GameRenderMessage {
    pub guild_id: Id<GuildMarker>,
    pub game_id: GameId,
    pub embeds: GameEmbeds,
}

impl GameRenderMessage {
    pub fn new(guild_id: Id<GuildMarker>, game_id: GameId, embeds: GameEmbeds) -> Self {
        Self {
            guild_id,
            game_id,
            embeds,
        }
    }
}
