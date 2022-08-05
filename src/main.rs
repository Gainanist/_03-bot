mod bundles;
mod cli;
mod command_parser;
mod components;
mod controller;
mod dice;
mod discord_client;
mod events;
mod game_helpers;
mod io;
mod localization;
mod systems;

use std::{
    collections::{HashMap, HashSet},
    env,
    error::Error,
    fs,
    sync::{Arc, Mutex},
    time::Duration,
};

use bevy_turborand::RngPlugin;
use clap::Parser;
use command_parser::is_game_starting;
use discord_client::DiscordClient;
use io::{read_json, write_json_from_channel};

use components::PlayerName;
use events::{EventsPlugin, GameStartEvent, InputEvent, PlayerAttackEvent};
use futures::stream::StreamExt;
use game_helpers::{EventDelay, Game, GameRenderMessage};
use localization::{Localization, Localizations};
use std::sync::mpsc::{self, Sender};

use crate::cli::Cli;
use crate::{command_parser::BYGONE_PARTS_FROM_EMOJI_NAME, systems::*};

use bevy::{app::ScheduleRunnerSettings, prelude::*};

use twilight_gateway::{cluster::Cluster, Event};
use twilight_http::{request::channel::reaction::RequestReactionType, Client as HttpClient};
use twilight_model::{
    channel::{Reaction, ReactionType},
    gateway::{payload::incoming::MessageCreate, Intents},
    id::{
        marker::{ChannelMarker, GuildMarker, MessageMarker, UserMarker},
        Id,
    },
    user::CurrentUser,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error + Send + Sync>> {
    let token = env::var("DISCORD_TOKEN")?;
    let (client, events) = DiscordClient::new(token).await?;
    client.startup();
    let input_receiver = client.listen_discord(events).await?;
    let output_sender = client.listen_game().await;

    let cli = Cli::parse();
    let games = read_json::<HashMap<Id<GuildMarker>, Game>>(&cli.games_path);
    // let scoreboard = read_json::<HashMap::<Id<GuildMarker>, HashMap<Id<UserMarker>, usize>>>(&cli.scoreboard_path);
    let (games_sender, games_receiver) = mpsc::channel::<HashMap<Id<GuildMarker>, Game>>();
    let games_path = cli.games_path.clone();

    tokio::spawn(async move {
        loop {
            write_json_from_channel(&games_receiver, &games_path);
        }
    });

    // let (scoreboard_sender, scoreboard_receiver) = mpsc::channel::<HashMap::<Id<GuildMarker>, HashMap<Id<UserMarker>, usize>>>();
    // let scoreboard_path = cli.scoreboard_path.clone();

    // tokio::spawn(async move {
    //     loop {
    //         write_json_from_channel(&scoreboard_receiver, &scoreboard_path);
    //     }
    // });

    let render_label = "render";

    App::new()
        .insert_resource(ScheduleRunnerSettings::run_loop(Duration::from_millis(100)))
        .insert_resource(EventDelay(Duration::from_millis(150)))
        .insert_resource(games)
        // .insert_resource(scoreboard)
        .insert_resource(HashMap::<Id<GuildMarker>, Vec<String>>::new())
        .add_plugins(MinimalPlugins)
        .add_plugin(RngPlugin::default())
        .add_plugin(EventsPlugin::default())
        .add_system(listen(Mutex::new(input_receiver)))
        .add_system(delay_events)
        .add_system(turn_timer)
        .add_system(spawn_bygones)
        .add_system(spawn_players)
        .add_system(damage_bygone)
        .add_system(damage_players)
        .add_system(process_bygone_part_death)
        .add_system(deactivate)
        .add_system(update_game_status)
        .add_system(log_battle.before(render_label))
        .add_system(render(Mutex::new(output_sender)).label(render_label))
        .add_system(ready_players)
        .add_system(cleanup)
        .add_system(save_games(Mutex::new(games_sender)))
        // .add_system(save_scoreboard(Mutex::new(scoreboard_sender)))
        .run();

    Ok(())
}
