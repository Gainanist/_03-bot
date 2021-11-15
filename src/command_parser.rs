use crate::{command::TextCommand, language::Language};

pub fn parse_command(source: &str) -> Option<(TextCommand, Language)> {
    if source.contains("битв") || source.contains("сраж") {
        Some((TextCommand::StartGame, Language::Ru))
    } else if source.contains("battl") || source.contains("fight") {
        Some((TextCommand::StartGame, Language::En))
    } else {
        None
    }
}