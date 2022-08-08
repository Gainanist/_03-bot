use std::{
    collections::{HashMap, HashSet},
    error::Error,
    sync::{
        mpsc::{self, Receiver, Sender},
        Arc, Mutex,
    },
    time::{Duration, Instant},
};

use futures::stream::StreamExt;

use twilight_gateway::{
    cluster::{ClusterStartError, Events},
    Cluster, Event, Intents,
};
use twilight_http::Client as HttpClient;
use twilight_model::{id::{
    marker::{ChannelMarker, GuildMarker, MessageMarker},
    Id,
}, application::{interaction::InteractionType, component::{button::ButtonStyle, Component, Button, ActionRow}}, channel::ReactionType, http::interaction::{InteractionResponse, InteractionResponseType}};

use crate::{
    command_parser::{is_game_starting, BYGONE_PARTS_FROM_EMOJI_NAME},
    controller::{send_game_message, start_game, process_interaction},
    events::InputEvent,
    game_helpers::GameRenderMessage,
    localization::Localizations,
};

pub struct DiscordClient {
    cluster: Arc<Cluster>,
    http: Arc<HttpClient>,
    http_clone: Arc<HttpClient>,
    game_channel_ids: Arc<Mutex<HashMap<Id<GuildMarker>, Id<ChannelMarker>>>>,
}

impl DiscordClient {
    pub async fn new(token: String) -> Result<(Self, Events), ClusterStartError> {
        let (cluster, events) = Cluster::new(
            token.to_owned(),
            Intents::GUILD_MESSAGES | Intents::MESSAGE_CONTENT,
        )
        .await?;
        let cluster = Arc::new(cluster);

        let http_clone = Arc::new(HttpClient::new(token.to_owned()));
        let http = Arc::new(HttpClient::new(token));

        let game_channel_ids = Arc::new(Mutex::new(
            HashMap::<Id<GuildMarker>, Id<ChannelMarker>>::new(),
        ));

        Ok((
            Self {
                cluster,
                http,
                http_clone,
                game_channel_ids,
            },
            events,
        ))
    }

    pub fn startup(&self) {
        let cluster_spawn = Arc::clone(&self.cluster);
        tokio::spawn(async move {
            cluster_spawn.up().await;
        });
    }

    pub async fn listen_discord(
        &self,
        mut events: Events,
    ) -> Result<Receiver<InputEvent>, Box<dyn Error + Send + Sync>> {
        let game_channel_ids_input = Arc::clone(&self.game_channel_ids);
        let http = Arc::clone(&self.http_clone);
        let me = self.http.current_user().exec().await?.model().await?;
        let app_id = self.http.current_user_application().exec().await?.model().await?.id;
        let (input_sender, input_receiver) = mpsc::channel();

        tokio::spawn(async move {
            let localizations = Localizations::new();
            // Process each event as they come in.
            while let Some((shard_id, event)) = events.next().await {
                match event {
                    Event::MessageCreate(msg) => {
                        println!(
                            "Received MessageCreate event from channel {}",
                            msg.channel_id
                        );
                        if let Some(guild_id) = msg.guild_id {
                            if msg.author.id != me.id {
                                if let Some(language) = is_game_starting(&msg.content) {
                                    println!("Starting game in guild {}", guild_id);
                                    let localization = localizations.get(language).clone();
                                    start_game(&input_sender, localization, &msg);
                                    if let Ok(mut game_channel_ids_input_lock) =
                                        game_channel_ids_input.lock()
                                    {
                                        game_channel_ids_input_lock
                                            .insert(guild_id, msg.channel_id);
                                    }
                                } else {
                                    println!("Failed to start game: wrong user intent, channel: {}", msg.channel_id);
                                }
                            } else {
                                println!("Failed to start game: message author is self, channel: {}", msg.channel_id);
                            }
                        } else {
                            println!("Failed to start game: empty guild_id, channel: {}", msg.channel_id);
                        }
                    }
                    Event::ShardConnected(_) => {
                        println!("Connected on shard {}", shard_id);
                    }
                    Event::InteractionCreate(interaction) => {
                        println!("Received InteractionCreate event");
                        let http = Arc::clone(&http);
                        let interaction_clone = interaction.clone();
                        let response = InteractionResponse {
                            kind: InteractionResponseType::DeferredUpdateMessage,
                            data: None,
                        };
                        tokio::spawn(
                             http.interaction(app_id)
                                .create_response(
                                    interaction_clone.id,
                                    &interaction_clone.token,
                                    &response,
                                )
                                .exec()
                        );
                        if let Some(ev) = process_interaction(interaction.0) {
                            input_sender.send(ev);
                        }
                    }
                    _ => {}
                }
            }
        });

        Ok(input_receiver)
    }

    pub async fn listen_game(&self) -> Sender<GameRenderMessage> {
        let game_channel_ids_output = Arc::clone(&self.game_channel_ids);
        let http_write = Arc::clone(&self.http);
        let (output_sender, output_receiver) = mpsc::channel::<GameRenderMessage>();

        tokio::spawn(async move {
            let mut message_ids = HashMap::new();
            loop {
                let msg = output_receiver.recv_timeout(Duration::from_secs(1));
                if let Ok(msg) = msg {
                    let mut channel_id = None;
                    if let Ok(game_channel_ids_output_lock) = game_channel_ids_output.lock() {
                        if let Some(_channel_id) = game_channel_ids_output_lock.get(&msg.guild_id) {
                            channel_id = Some(*_channel_id);
                        }
                    }

                    if let Some(channel_id) = channel_id {
                        let game_id = msg.game_id;
                        let message_id = message_ids.get(&game_id);
                        match send_game_message(&http_write, message_id, msg, channel_id).await {
                            Ok(message_id) => {
                                println!(
                                    "Successfully sent/updated a game message in channel {}",
                                    channel_id
                                );
                                if let Some(message_id) = message_id {
                                    message_ids.insert(game_id, message_id);
                                } else {
                                    message_ids.remove(&game_id);
                                }
                            }
                            Err(err) => {
                                println!(
                                    "Error sending/updating a game message in channel {}: {}",
                                    channel_id, err
                                );
                            }
                        }
                    }
                }
            }
        });

        output_sender
    }
}
