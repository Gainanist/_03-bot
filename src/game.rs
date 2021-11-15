use std::{cell::Cell, collections::HashMap};

use itertools::Itertools;

use crate::{bygone_03::Bygone03, entities::{AggressiveEntity, Entity, Random}, localization::Localize, player_action::PlayerAction};

pub struct Game {
    players: HashMap<i64, AggressiveEntity>,
    enemy: Bygone03,
    rng: Cell<Random>,
}

impl Game {
    pub fn new() -> Self {
        Game {
            players: HashMap::new(),
            enemy: Bygone03::normal(),
            rng: Cell::new(Random::new()),
        }
    }

    pub fn finished(&self) -> bool {
        !self.enemy.alive() || self.players.iter().all(|(_, entity)| !entity.vitality().health().alive())
    }

    pub fn update(&mut self, commands: &HashMap<i64, PlayerAction>) {
        for (player_id, command) in commands {
            let player = self.players
                .entry(*player_id)
                .or_insert(Entity::aggressive(6, 0, 1, 100));
            
            if !player.vitality().health().alive() {
                continue;
            }

            match *command {
                PlayerAction::StrikeCore => self.enemy.damage_core(&player.attack(), self.rng.get_mut()),
                PlayerAction::StrikeSensor => self.enemy.damage_sensor(&player.attack(), self.rng.get_mut()),
                PlayerAction::StrikeGun => self.enemy.damage_gun(&player.attack(), self.rng.get_mut()),
                PlayerAction::StrikeLeftWing => self.enemy.damage_left_wing(&player.attack(), self.rng.get_mut()),
                PlayerAction::StrikeRightWing => self.enemy.damage_right_wing(&player.attack(), self.rng.get_mut()),
            }
        }

        if !self.enemy.alive() {
            return;
        }

        let mut living_players = self.players
            .values_mut()
            .filter(|player| player.vitality().health().alive())
            .collect::<Vec<_>>();
        let victim = self.rng.get_mut().choose_mut(&mut living_players);
        self.rng.get_mut().collide(self.enemy.attack(), victim.vitality_mut());
    }
}

impl Localize for Game {
    fn localize(&self, localization: &crate::localization::Localization) -> String {
        format!(
"
{}
{}

{}
",
            localization.title,
            self.enemy.localize(localization),
            self.players.iter()
                .map(|(id, player)| {
                    format!(
                        "{}\t{}: {}\t {}: {}",
                        id,
                        localization.health,
                        player.vitality().health().localize(localization),
                        localization.attack,
                        player.attack().damage(),
                    )
                })
                .join("\n"),
        )
    }
}
