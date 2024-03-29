use std::str::FromStr;

use phf::phf_ordered_map;
use twilight_model::application::interaction::application_command::{
    CommandData, CommandOptionValue,
};

use crate::{components::BygonePart, game_helpers::Difficulty, localization::Language};

pub const BATTLE_COMMAND: &str = "battle";
pub const LANGUAGE_COMMAND_OPTION: &str = "language";
pub const DIFFICULTY_COMMAND_OPTION: &str = "difficulty";

pub fn is_game_starting(command: &CommandData) -> Option<(Language, Difficulty)> {
    if command.name != BATTLE_COMMAND {
        return None;
    }
    let mut language = Language::En;
    let mut difficulty = Difficulty::Medium;
    for option in &command.options {
        if option.name == LANGUAGE_COMMAND_OPTION {
            if let CommandOptionValue::String(lang_name) = &option.value {
                if let Ok(lang) = Language::from_str(lang_name) {
                    language = lang;
                }
            }
        }
        if option.name == DIFFICULTY_COMMAND_OPTION {
            if let CommandOptionValue::String(level) = &option.value {
                if let Ok(level) = Difficulty::from_str(level) {
                    difficulty = level;
                }
            }
        }
    }
    Some((language, difficulty))
}

pub const BYGONE_PARTS_FROM_EMOJI_NAME: phf::OrderedMap<&str, BygonePart> = phf_ordered_map! {
    "🇨" => BygonePart::Core,
    "🇸" => BygonePart::Sensor,
    "🇱" => BygonePart::LeftWing,
    "🇷" => BygonePart::RightWing,
    "🇬" => BygonePart::Gun,
};

// Maybe use Ъ with a very small chance
// pub const AUXILIARY_EMOJIS: [&str; 5] = [
//     ":Nod_shy:",
//     ":Ollie_in_sunglasses:",
//     ":Mentor_really:",
//     ":Unter_shocked:",
//     ":Thea_thinking:",
// ];
