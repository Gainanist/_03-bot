use phf::phf_ordered_map;
use twilight_model::application::{interaction::application_command::{CommandData, CommandOptionValue}, command::CommandType};

use crate::{components::BygonePart, localization::Language, game_helpers::Difficulty};

pub fn is_game_starting(command: &CommandData) -> Option<(Language, Difficulty)> {
    if command.name != "battle" {
        return None;
    }
    let mut language = Language::En;
    let mut difficulty = Difficulty::Medium;
    for option in &command.options {
        if option.name == "language" {
            if let CommandOptionValue::String(lang_name) = &option.value {
                if lang_name == "ru" {
                    language = Language::Ru;
                }
            }
        }
        if option.name == "difficulty" {
            if let CommandOptionValue::String(level) = &option.value {
                if let Some(level) = Difficulty::from_str(level) {
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
pub const AUXILIARY_EMOJIS: [&str; 5] = [
    ":Nod_shy:",
    ":Ollie_in_sunglasses:",
    ":Mentor_really:",
    ":Unter_shocked:",
    ":Thea_thinking:",
];
