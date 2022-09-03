use clap::Parser;
use std::path::PathBuf;

/// Discord bot to fight enemies from Uof7
#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
pub struct Cli {
    /// Saved games file
    #[clap(short, long, value_parser, value_name = "FILE")]
    pub games_path: PathBuf,
    // /// Scoreboard file
    // #[clap(short, long, value_parser, value_name = "FILE")]
    // pub scoreboard_path: PathBuf,\
    /// Update bot commands
    #[clap(short, long, action)]
    pub update_commands: bool,
}
