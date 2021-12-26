use phf::phf_ordered_map;

use crate::{language::Language, components::BygonePart};

pub fn is_game_starting(source: &str) -> Option<Language> {
    if source.contains("битв") || source.contains("сраж") {
        Some(Language::Ru)
    } else if source.contains("battl") || source.contains("fight") {
        Some(Language::En)
    } else {
        None
    }
}

pub const BYGONE_PARTS_FROM_EMOJI_NAME: phf::OrderedMap<&str, BygonePart> = phf_ordered_map! {
    "🇨" => BygonePart::Core,
    "🇸" => BygonePart::Sensor,
    "🇱" => BygonePart::LeftWing,
    "🇷" => BygonePart::RightWing,
    "🇬" => BygonePart::Gun,
};
