mod components;
mod systems;
mod bygone_03;
mod localization;
mod command_parser;
mod language;
mod player_command;
mod dice;
mod events;

use std::{env, error::Error, sync::{Arc, mpsc::Receiver, Mutex}, task::Poll, pin::Pin, time::Duration, collections::HashMap, num::NonZeroU64};
use command_parser::is_game_starting;
use components::{Player, Active};
use events::*;
use futures::{stream::{Stream, StreamExt}, Future};
use localization::{Localizations, Localization};
use std::sync::mpsc::{self, Sender};
// use systems::Game;

use crate::{bygone_03::*, command_parser::BYGONE_PARTS_FROM_EMOJI_NAME};

use bevy::{prelude::*, app::ScheduleRunnerSettings};
use bevy_rng::*;
use twilight_cache_inmemory::{InMemoryCache, ResourceType};
use twilight_embed_builder::{EmbedBuilder, EmbedFieldBuilder, ImageSource};
use twilight_gateway::{cluster::{Cluster, ShardScheme, Events}, Event};
use twilight_http::{Client as HttpClient, response::ResponseFuture};
use twilight_model::{gateway::{Intents, payload::incoming::{ReactionAdd, MessageCreate}}, id::{ChannelId, MessageId, UserId}, channel::{Reaction, ReactionType, embed::Embed, Message}};

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error + Send + Sync>> {
    let token = env::var("DISCORD_TOKEN")?;

    // This is the default scheme. It will automatically create as many
    // shards as is suggested by Discord.
    let scheme = ShardScheme::Auto;

    // Use intents to only receive guild message events.
    let (cluster, mut events) = Cluster::builder(
        token.to_owned(),
        Intents::GUILD_MESSAGES | Intents::GUILD_MESSAGE_REACTIONS
    )
        .shard_scheme(scheme)
        .build()
        .await?;
    let cluster = Arc::new(cluster);

    // Start up the cluster.
    let cluster_spawn = Arc::clone(&cluster);

    // Start all shards in the cluster in the background.
    tokio::spawn(async move {
        cluster_spawn.up().await;
    });

    // HTTP is separate from the gateway, so create a new client.
    let http = Arc::new(HttpClient::new(token));

    // Since we only care about new messages, make the cache only
    // cache new messages.
    // let cache = InMemoryCache::builder()
    //     .resource_types(ResourceType::MESSAGE | ResourceType::REACTION)
    //     .build();
    // let cache = Arc::new(cache);

    // let cache_write = Arc::clone(&cache);

    // let mut message_ids: Arc<Mutex<HashMap<GameId, MessageId>>> = Arc::new(Mutex::new(HashMap::new()));
    // let message_ids_input = Arc::clone(&message_ids);
    // let mut message_ids_output = Arc::clone(&message_ids);

    let http_input = Arc::clone(&http);
    let (input_sender, input_receiver) = mpsc::channel();

    tokio::spawn(async move {
        let localizations = Localizations::new();
        // Process each event as they come in.
        while let Some((shard_id, event)) = events.next().await {
            match event {
                Event::MessageCreate(msg) => {
                    if !msg.author.bot {
                        if let Some(language) = is_game_starting(&msg.content) {
                            let localization = localizations.get(language).clone();
                            let game_start_sender = input_sender.clone();
                            let game_start_http = Arc::clone(&http_input);

                            tokio::spawn(async move {
                                start_game(game_start_sender, game_start_http, localization, msg).await;
                            });
                        }
                    }
                },
                Event::ReactionAdd(reaction) => process_reaction(&reaction.0, &input_sender),
                Event::ReactionRemove(reaction) => process_reaction(&reaction.0, &input_sender),
                Event::ShardConnected(_) => {
                    println!("Connected on shard {}", shard_id);
                }
                _ => {}
            }
        }
    });

    let http_write = Arc::clone(&http);
    let (output_sender, output_receiver) = mpsc::channel::<GameRenderMessage>();

    tokio::spawn(async move {
        loop {
            let msg = output_receiver.recv_timeout(Duration::from_secs(1));
            if let Ok(msg) = msg {
                send_game_message(&http_write, msg).await;
            }
        }
    });

    // let mut event_handler = EventHandler::new();
    // // Process each event as they come in.
    // while let Some((shard_id, event)) = events.next().await {
    //     // Update the cache with the event.
    //     cache.update(&event);

    //     event_handler.handle(
    //         shard_id,
    //         event,
    //         Arc::clone(&http),
    //     ).await?;
    // }

    App::build()
        .insert_resource(ScheduleRunnerSettings::run_loop(Duration::from_secs(5)))
        .add_plugins(MinimalPlugins)
        .add_plugin(RngPlugin::default())
        .add_system(listen.system())
        .add_system(spawn_bygones.system())
        .add_system(spawn_players.system())
        .add_system(damage_bygone.system())
        .add_system(damage_players.system())
        .add_system(deativate.system())
        .add_system(cleanup.system())
        .run();

    Ok(())
}

#[derive(Clone, Debug)]
struct GameRenderMessage {
    embeds: Vec<Embed>,
    channel_id: ChannelId,
    message_id: MessageId,
}

impl GameRenderMessage {
    fn new(embeds: Vec<Embed>, channel_id: ChannelId, message_id: MessageId) -> Self {
        Self {
            embeds,
            channel_id,
            message_id,
        }
    }
}

fn process_reaction(reaction: &Reaction, sender: &Sender<InputEvent>) {
    if let ReactionType::Unicode { name } = &reaction.emoji {
        if let Some(bygone_part) = BYGONE_PARTS_FROM_EMOJI_NAME.get(name) {
            sender.send(InputEvent::PlayerAttack(
                PlayerAttackEvent::new(
                    reaction.user_id,
                    reaction.message_id,
                    reaction.channel_id,
                    *bygone_part,
                )
            )
        );
        }
    }
}

async fn start_game(sender: Sender<InputEvent>, http: Arc<HttpClient>, localization: Localization, msg: Box<MessageCreate>)
    -> Result<(), Box<dyn Error + Sync + Send>>
{
    let emdeds = [
        EmbedBuilder::new()
            .description(&localization.intro)
            .build()
            .unwrap(),
    ];
    let message_id = http
        .create_message(msg.channel_id)
        .embeds(&emdeds)?
        .exec()
        .await?
        .model()
        .await?
        .id;
    sender.send(
        InputEvent::GameStart(GameStartEvent::new(
            msg.author.id,
            message_id,
            msg.channel_id,
            localization,
        ))
    )?;
    Ok(())
}

async fn send_game_message(
    http: &HttpClient,
    msg: GameRenderMessage
) -> Result<(), Box<dyn Error + Send + Sync>> {
    http.update_message(msg.channel_id, msg.message_id)
        .embeds(&[])?
        .embeds(&msg.embeds)?
        .exec()
        .await?;
    Ok(())
}

// struct DiscordListener {
//     events: Events,
//     http: Arc<HttpClient>,
//     cache: InMemoryCache,
// }

// impl Iterator for DiscordListener {
//     type Item = Event;

//     fn next(&mut self) -> Option<Self::Item> {
//         if let Some((shard_id, event)) = self.events.next() {
//             // Update the cache with the event.
//             self.cache.update(&event);
//             Some(event)1
//         }
//         None
//     }
// }

fn listen(
    mut input_receiver: Local<Option<Mutex<Receiver<InputEvent>>>>,
    mut games: ResMut<HashMap<ChannelId, Game>>,
    mut ev_game_start: EventWriter<GameStartEvent>,
    mut ev_player_attack: EventWriter<PlayerAttackEvent>,
    mut ev_player_join: EventWriter<PlayerJoinEvent>,
    players: Query<(&UserId, Option<&Active>), (With<Player>,)>,
) {
    let mut events = Vec::new();
    if let Some(input_receiver) = &mut *input_receiver {
        if let Ok(ref mut receiver_lock) = input_receiver.try_lock() {
            while let Ok(event) = receiver_lock.recv() {
                events.push(event);
            }
        }
    }
    let events = events;

    let players: HashMap<_, _> = players.iter().collect();

    for event in events.into_iter() {
        match event {
            InputEvent::GameStart(ev) => {
                ev_game_start.send(ev.clone());
                games.insert(ev.channel, Game::new(ev.message, ev.localization));
                ev_player_join.send(PlayerJoinEvent::new(ev.initial_player, ev.channel));
            }
            InputEvent::PlayerAttack(ev) => {
                if games.contains_key(&ev.channel) {
                    match players.get(&ev.player) {
                        Some(maybe_active) => {
                            if let Some(_active) = maybe_active {
                                ev_player_attack.send(ev);
                            }
                        },
                        None => {
                            ev_player_join.send(PlayerJoinEvent::new(
                                ev.player.clone(),
                                ev.channel.clone(),
                            ));
                            ev_player_attack.send(ev);
                        }
                    }
                }
            },
        }
    }
}

#[derive(Clone, Debug)]
pub struct Game {
    pub message_id: MessageId,
    pub localization: Localization,
}

impl Game {
    pub fn new(message_id: MessageId, localization: Localization) -> Self {
        Self {
            message_id,
            localization,
        }
    }
}

struct EventHandler {
    //  localizations: Localizations,
}

impl EventHandler {
    pub fn new() -> Self {
        EventHandler {
            // localizations: Localizations::new(),
        }
    }

    pub async fn handle(
        &mut self,
        shard_id: u64,
        event: Event,
        http: Arc<HttpClient>,
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        match event {
            Event::MessageCreate(msg) => {
                if msg.author.bot {
                    ()
                } else {
                
                // if let Some((_command, language)) = parse_command(&msg.content) {
                    // let game = Game::new();
                    // let localization = self.localizations.get(language);
                    //self.send_message(&game.render_text(localization), msg.channel_id, http).await?;
                    self.send_message(
                        "a\ta\naa\ta",
                        msg.channel_id,
                        http,
                    ).await?;
                }
            }
            Event::ShardConnected(_) => {
                println!("Connected on shard {}", shard_id);
            }
            // Other events here...
            _ => {}
        }

        Ok(())
    }

    async fn send_message(&self, _msg: &str, channel_id: ChannelId, http: Arc<HttpClient>)
        -> Result<(), Box<dyn Error + Send + Sync>> 
    {
        let embeds = [
            EmbedBuilder::new()
                .description("**A wild _03 appeared!**")
                .image(ImageSource::url("http://www.uof7.com/wp-content/uploads/2016/09/15-Bygone-UPD.gif")?)
                .build()?,
            EmbedBuilder::new()
                .field(EmbedFieldBuilder::new("Status", "ATK 2, ACC 100%, Core: armored").build())
                .field(EmbedFieldBuilder::new(":regional_indicator_c:ore", "[▮▮] - 20%").inline())
                .field(EmbedFieldBuilder::new(":regional_indicator_s:ensor", "[▮▮] - 50%").inline())
                .field(EmbedFieldBuilder::new(":regional_indicator_l:eft wing", "[▮▮] - 70%").inline())
                .field(EmbedFieldBuilder::new(":regional_indicator_r:ight wing", "[▮▮] - 70%").inline())
                .field(EmbedFieldBuilder::new(":regional_indicator_g:un", "[▮▮] - 50%").inline())
                .build()?,
            EmbedBuilder::new()
                .field(EmbedFieldBuilder::new("Battle log", "> • _03 gently punches Rokari in the chest with a rubber bullet\n> • Rokari performs a drunken style attack\n> • Ultra_Scream has joined the fray").build())
                .build()?,
            EmbedBuilder::new()
                .field(EmbedFieldBuilder::new("Rokari", "[▮▮▮▮▯▯]").inline())
                .field(EmbedFieldBuilder::new("Ultra_Scream", "[▮▮▮▮▮]").inline())
                .build()?,
            // EmbedBuilder::new()
            //     .description("Here's a cool image of Twilight Sparkle")
            //     .image(ImageSource::url("http://www.uof7.com/wp-content/uploads/2016/09/15-Bygone-UPD.gif")?)
            //     .build()?,
        ];
        http.create_message(channel_id)
            // .content(msg)?
            .embeds(&embeds)?
            .exec()
            .await?;
        Ok(())
    }
}
