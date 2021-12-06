use crate::{player_command::PlayerCommand, language::Language};

pub fn parse_command(source: &str) -> Option<(PlayerCommand, Language)> {
    if source.contains("битв") || source.contains("сраж") {
        Some((PlayerCommand::StartGame, Language::Ru))
    } else if source.contains("battl") || source.contains("fight") {
        Some((PlayerCommand::StartGame, Language::En))
    } else {
        None
    }
}