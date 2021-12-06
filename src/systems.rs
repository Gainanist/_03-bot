use bevy::prelude::*;
use enum_map::{enum_map, EnumMap};

use crate::components::{Attack, BygonePart, Enemy, Bygone03Stage, Vitality};

fn new_bygone03(parts_health: usize) -> (EnumMap<BygonePart, Vitality>, Attack, Bygone03Stage, Enemy) {
    let parts = enum_map! {
        BygonePart::Core => Vitality::new(parts_health, 80),
        BygonePart::Sensor => Vitality::new(parts_health, 70),
        BygonePart::Gun => Vitality::new(parts_health, 50),
        BygonePart::LeftWing => Vitality::new(parts_health, 30),
        BygonePart::RightWing => Vitality::new(parts_health, 50),
    };
    let attack = Attack::new(1, 100);

    (parts, attack, Bygone03Stage::Armored, Enemy)
}

pub fn add_enemy(mut commands: Commands) {
    commands.spawn_bundle(
        new_bygone03(1)
    );
}



// pub struct Game {
//     players: HashMap<i64, AggressiveEntity>,
//     enemy: Bygone03,
//     rng: Random,
// }

// impl Game {
//     pub fn new() -> Self {
//         Game {
//             players: HashMap::new(),
//             enemy: Bygone03::normal(),
//             rng: Random::new(),
//         }
//     }

//     pub fn finished(&self) -> bool {
//         !self.enemy.alive() || self.all_players_dead()
//     }

//     fn all_players_dead(&self) -> bool {
//         self.players.iter().all(|(_, entity)| !entity.vitality().health().alive())
//     }

//     pub fn update(&mut self, commands: &HashMap<i64, PlayerAction>) {
//         for (player_id, command) in commands {
//             let player = self.players
//                 .entry(*player_id)
//                 .or_insert(Entity::aggressive(6, 0, 1, 100));
            
//             if !player.vitality().health().alive() {
//                 continue;
//             }

//             match *command {
//                 PlayerAction::StrikeCore => self.enemy.damage_core(&player.attack(), &mut self.rng),
//                 PlayerAction::StrikeSensor => self.enemy.damage_sensor(&player.attack(), &mut self.rng),
//                 PlayerAction::StrikeGun => self.enemy.damage_gun(&player.attack(), &mut self.rng),
//                 PlayerAction::StrikeLeftWing => self.enemy.damage_left_wing(&player.attack(), &mut self.rng),
//                 PlayerAction::StrikeRightWing => self.enemy.damage_right_wing(&player.attack(), &mut self.rng),
//             }
//         }

//         if !self.enemy.alive() {
//             return;
//         }

//         let mut living_players = self.players
//             .values_mut()
//             .filter(|player| player.vitality().health().alive())
//             .collect::<Vec<_>>();
//         let victim = self.rng.choose_mut(&mut living_players);
//         self.rng.collide(self.enemy.attack(), victim.vitality_mut());
//     }
// }

// impl RenderText for Game {
//     fn render_text(&self, localization: &crate::localization::Localization) -> String {
//         format!(
// "
// {}
// {}

// {}
// ",
//             localization.title,
//             self.enemy.render_text(localization),
//             self.players.iter()
//                 .map(|(id, player)| {
//                     format!(
//                         "{}\t{}: {}\t {}: {}",
//                         id,
//                         localization.health,
//                         player.vitality().health().render_text(localization),
//                         localization.attack,
//                         player.attack().damage(),
//                     )
//                 })
//                 .join("\n"),
//         )
//     }
// }
