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
    controller::{start_game, process_interaction, update_game_message, create_game_message},
    events::{InputEvent, GameRenderEvent},
    localization::Localizations, discord_renderer::{DiscordRenderer, RenderedGame, RenderedMessage},
};

pub struct DiscordClient {
    cluster: Arc<Cluster>,
    http: Arc<HttpClient>,
    http_clone: Arc<HttpClient>,
    game_channel_ids: Arc<Mutex<HashMap<Id<GuildMarker>, Id<ChannelMarker>>>>,
}

fn merge_with_cached(rendered_game: RenderedGame, mut cached: RenderedGame) -> RenderedGame {
    match rendered_game.upper_message {
        RenderedMessage::Message(message) => {
            cached.upper_message = message.into();
        },
        RenderedMessage::Skip => {},
        RenderedMessage::Delete => {
            cached.upper_message = RenderedMessage::Delete;
        },
    }
    match rendered_game.lower_message {
        RenderedMessage::Message(message) => {
            cached.lower_message = message.into();
        },
        RenderedMessage::Skip => {},
        RenderedMessage::Delete => {
            cached.lower_message = RenderedMessage::Delete;
        },
    }

    cached
}

async fn try_update_message(
    ev: GameRenderEvent,
    cached: RenderedGame,
    http: &HttpClient,
    message_id: (Id<MessageMarker>, Id<MessageMarker>),
    channel_id: Id<ChannelMarker>,
    guild_id: Id<GuildMarker>,
    messages: &mut HashMap<Id<GuildMarker>, (Id<MessageMarker>, Id<MessageMarker>, RenderedGame)>,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    let rendered_game = DiscordRenderer::render_with_previous(ev, &cached)?;
    let message_id = update_game_message(http, message_id.0, message_id.1, &rendered_game, channel_id).await?;
    if let Some(message_id) = message_id {
        messages.insert(guild_id, (message_id.0, message_id.1, merge_with_cached(rendered_game, cached)));
    } else {
        messages.remove(&guild_id);
    }
    Ok(())
}

async fn try_create_message(
    ev: GameRenderEvent,
    http: &HttpClient,
    channel_id: Id<ChannelMarker>,
    guild_id: Id<GuildMarker>,
    messages: &mut HashMap<Id<GuildMarker>, (Id<MessageMarker>, Id<MessageMarker>, RenderedGame)>,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    let rendered_game = DiscordRenderer::render(ev)?;
    let message_id = create_game_message(http, &rendered_game, channel_id).await?;
    messages.insert(guild_id, (message_id.0, message_id.1, rendered_game.into()));
    Ok(())
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

    pub async fn listen_game(&self) -> Sender<GameRenderEvent> {
        let game_channel_ids_output = Arc::clone(&self.game_channel_ids);
        let http_write = Arc::clone(&self.http);
        let (output_sender, output_receiver) = mpsc::channel::<GameRenderEvent>();

        tokio::spawn(async move {
            let mut messages = HashMap::new();
            loop {
                let ev = output_receiver.recv_timeout(Duration::from_secs(1));
                if let Ok(ev) = ev {
                    let guild_id = ev.guild_id;
                    let mut channel_id = None;
                    if let Ok(game_channel_ids_output_lock) = game_channel_ids_output.lock() {
                        if let Some(_channel_id) = game_channel_ids_output_lock.get(&guild_id) {
                            channel_id = Some(*_channel_id);
                        }
                    }

                    if let Some(channel_id) = channel_id {
                        if let Some((upper_message_id, lower_message_id, cached)) = messages.remove(&guild_id) {
                            match try_update_message(ev, cached, &http_write, (upper_message_id, lower_message_id), channel_id, guild_id, &mut messages).await {
                                Ok(()) => println!(
                                    "Successfully updated a game message in channel {}",
                                    channel_id
                                ),
                                Err(err) => println!(
                                    "Error updating a game message in channel {}: {}",
                                    channel_id, err
                                ),
                            }
                        } else {
                            match try_create_message(ev, &http_write, channel_id, guild_id, &mut messages).await {
                                Ok(()) => println!(
                                    "Successfully created a game message in channel {}",
                                    channel_id
                                ),
                                Err(err) => println!(
                                    "Error creating a game message in channel {}: {}",
                                    channel_id, err
                                ),

                            }
                        }
                    }
                }
            }
        });

        output_sender
    }
}
