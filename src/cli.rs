use std::path::PathBuf;
use structopt::StructOpt;

#[derive(Debug, StructOpt)]
#[structopt(name = "_03-bot", about = "DES... TROY")]
pub struct Args {
    #[structopt(parse(from_os_str))]
    pub games_path: std::path::PathBuf,

    #[structopt(parse(from_os_str))]
    pub scoreboard_path: std::path::PathBuf,
}
