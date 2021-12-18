use phf::phf_map;

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

pub const BYGONE_PARTS_FROM_EMOJI_NAME: phf::Map<&str, BygonePart> = phf_map! {
    ":regional_indicator_c:" => BygonePart::Core,
    ":regional_indicator_s:" => BygonePart::Sensor,
    ":regional_indicator_l:" => BygonePart::LeftWing,
    ":regional_indicator_r:" => BygonePart::RightWing,
    ":regional_indicator_g:" => BygonePart::Gun,
};
