use std::{path::PathBuf};
use clap::Parser;

/// Discord bot to fight enemies from Uof7
#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
pub struct Cli {
    /// SAved games file
    #[clap(short, long, value_parser, value_name = "FILE")]
    pub games_path: PathBuf,

    // /// Scoreboard file
    // #[clap(short, long, value_parser, value_name = "FILE")]
    // pub scoreboard_path: PathBuf,
}
