use std::{
    collections::{HashMap, HashSet},
    error::Error,
    sync::{
        Arc, Mutex,
    },
    time::{Duration, Instant}, num::NonZeroU64,
};

use crossbeam_channel::{unbounded, Receiver, Sender};

use derive_new::new;
use futures::stream::StreamExt;

use twilight_gateway::{
    cluster::{ClusterStartError, Events},
    Cluster, Event, Intents,
};
use twilight_http::Client as HttpClient;
use twilight_model::{id::{
    marker::{ChannelMarker, GuildMarker, MessageMarker, InteractionMarker, ApplicationMarker},
    Id,
}, application::{interaction::{InteractionType, InteractionData}, component::{button::ButtonStyle, Component, Button, ActionRow}, command::{CommandOption, NumberCommandOptionData, CommandOptionValue, ChoiceCommandOptionData, CommandOptionChoice}}, channel::ReactionType, http::interaction::{InteractionResponse, InteractionResponseType}};

use crate::{
    command_parser::{is_game_starting, BYGONE_PARTS_FROM_EMOJI_NAME},
    controller::{start_game, process_interaction, update_game_message, create_game_message, create_message},
    events::{InputEvent, GameRenderEvent},
    localization::Localizations, discord_renderer::{DiscordRenderer, RenderedGame, RenderedMessage, DiscordRendererPureResult, DiscordRendererResult}, game_helpers::InteractionIds,
};

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
    interaction: &InteractionIds,
    lower_message_id: Id<MessageMarker>,
    guild_id: Id<GuildMarker>,
    messages: &mut HashMap<Id<GuildMarker>, (Id<MessageMarker>, RenderedGame)>,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    let interaction_id = interaction.id;
    match DiscordRenderer::render_with_previous(ev, &cached)? {
        DiscordRendererResult::Game(rendered_game) => {
            if let Some(followup_id) = update_game_message(http, interaction, lower_message_id, &rendered_game).await? {
                println!("discord_client: Updating game message with interaction id {}, new interaction id: {}", interaction_id, interaction.id);
                messages.insert(guild_id, (followup_id, merge_with_cached(rendered_game, cached)));
                println!("discord_client: Deleting game message with interaction id {}", interaction_id);
            }
        }
        DiscordRendererResult::Oneshot(oneshot_message) => {
            println!("discord_client: Creating oneshot message with interaction id {}", interaction_id);
            messages.insert(guild_id, (lower_message_id, cached));
            create_message(http, oneshot_message, &interaction).await?;
        }
    }
    Ok(())
}

async fn try_create_message(
    ev: GameRenderEvent,
    http: &HttpClient,
    interaction: &InteractionIds,
    guild_id: Id<GuildMarker>,
    messages: &mut HashMap<Id<GuildMarker>, (Id<MessageMarker>, RenderedGame)>,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    match DiscordRenderer::render(ev)? {
        DiscordRendererPureResult::Game(rendered_game) => {
            let followup_id = create_game_message(http, rendered_game.clone(), interaction).await?;
            messages.insert(guild_id, (followup_id, rendered_game.into())); 
        }
        DiscordRendererPureResult::Oneshot(oneshot_message) => {
            create_message(http, oneshot_message, &interaction).await?;
        }
    }    
    Ok(())
}

pub struct DiscordClient {
    cluster: Arc<Cluster>,
    http_write: Arc<HttpClient>,
    http_read: Arc<HttpClient>,
    game_channel_ids: Arc<Mutex<HashMap<Id<GuildMarker>, Id<ChannelMarker>>>>,
}

impl DiscordClient {
    pub async fn new(token: String) -> Result<(Self, Events), Box<dyn Error + Sync + Send>> {
        let (cluster, events) = Cluster::new(
            token.to_owned(),
            Intents::empty(),
        )
        .await?;
        let cluster = Arc::new(cluster);

        let http_read = Arc::new(HttpClient::new(token.to_owned()));
        let http_write = Arc::new(HttpClient::new(token));

        let game_channel_ids = Arc::new(Mutex::new(
            HashMap::<Id<GuildMarker>, Id<ChannelMarker>>::new(),
        ));

        Ok((
            Self {
                cluster,
                http_write,
                http_read,
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

    pub fn register_commands(&self) {
        async fn inner(http: Arc<HttpClient>) -> Result<(), Box<dyn Error + Sync + Send>> {
            let app_id = http.current_user_application().exec().await?.model().await?.id;
            println!("Registering commands for app id {}", app_id);
            http.interaction(app_id)
                .create_global_command()
                .chat_input("battle", "Fight the _03")?
                .description_localizations(&HashMap::from([
                    ("ru".to_owned(), "Сразиться с _03".to_owned())
                ]))?
                .command_options(&[
                    CommandOption::String(ChoiceCommandOptionData {
                        autocomplete: false,
                        choices: vec! [
                            CommandOptionChoice::String {
                                name: "Easy - Just like in the game".to_owned(),
                                name_localizations: Some(HashMap::from([
                                    ("ru".to_owned(), "Легко - Совсем как в игре".to_owned())
                                ])),
                                value: "easy".to_owned(),
                            },
                            CommandOptionChoice::String {
                                name: "Medium - Take a buddy with you".to_owned(),
                                name_localizations: Some(HashMap::from([
                                    ("ru".to_owned(), "Средне - Позови друга".to_owned())
                                ])),
                                value: "medium".to_owned(),
                            },
                            CommandOptionChoice::String {
                                name: "Hard - You shall not pass!".to_owned(),
                                name_localizations: Some(HashMap::from([
                                    ("ru".to_owned(), "Сложно - Ты не пройдёшь!".to_owned())
                                ])),
                                value: "hard".to_owned(),
                            },
                            CommandOptionChoice::String {
                                name: "Real bullets - Forgive me, Mister Pikes...".to_owned(),
                                name_localizations: Some(HashMap::from([
                                    ("ru".to_owned(), "Боевые патроны - Простите меня, мистер Пайкс...".to_owned())
                                ])),
                                value: "real_bullets".to_owned(),
                            },
                        ],
                        description: "Battle difficulty".to_owned(),
                        description_localizations: Some(HashMap::from([
                            ("ru".to_owned(), "Сложность битвы".to_owned())
                        ])),
                        max_length: None,
                        min_length: None,
                        name: "difficulty".to_owned(),
                        name_localizations: Some(HashMap::from([
                            ("ru".to_owned(), "сложность".to_owned())
                        ])),
                        required: false,
                    }),
                    CommandOption::String(ChoiceCommandOptionData {
                        autocomplete: false,
                        choices: vec! [
                            CommandOptionChoice::String {
                                name: "English".to_owned(),
                                name_localizations: Some(HashMap::from([
                                    ("ru".to_owned(), "английский".to_owned())
                                ])),
                                value: "en".to_owned(),
                            },
                            CommandOptionChoice::String {
                                name: "Russian".to_owned(),
                                name_localizations: Some(HashMap::from([
                                    ("ru".to_owned(), "русский".to_owned())
                                ])),
                                value: "ru".to_owned(),
                            },
                        ],
                        description: "Interface language".to_owned(),
                        description_localizations: Some(HashMap::from([
                            ("ru".to_owned(), "Язык интерфейса".to_owned())
                        ])),
                        max_length: None,
                        min_length: None,
                        name: "language".to_owned(),
                        name_localizations: Some(HashMap::from([
                            ("ru".to_owned(), "язык".to_owned())
                        ])),
                        required: false,
                    })
                ])?
                .exec()
                .await?;
            println!("Commands register success");
            Ok(())
        }

        let http = Arc::clone(&self.http_write);
        tokio::spawn(async move {
            inner(http).await.unwrap();
        });
    }

    pub async fn listen_discord(
        &self,
        mut events: Events,
    ) -> Result<(Receiver<InputEvent>, Receiver<InteractionIds>), Box<dyn Error + Send + Sync>> {
        let game_channel_ids_input = Arc::clone(&self.game_channel_ids);
        let http = Arc::clone(&self.http_read);
        let me = self.http_write.current_user().exec().await?.model().await?;
        let (input_sender, input_receiver) = unbounded();
        let (interaction_sender, interaction_receiver) = unbounded();

        tokio::spawn(async move {
            let localizations = Localizations::new();
            // Process each event as they come in.
            while let Some((shard_id, event)) = events.next().await {
                match event {
                    Event::ShardConnected(_) => {
                        println!("Connected on shard {}", shard_id);
                    }
                    Event::InteractionCreate(interaction) if interaction.kind == InteractionType::MessageComponent => {
                        println!("Received InteractionCreate event of type MessageComponent");
                        let http = Arc::clone(&http);
                        let interaction_clone = interaction.clone();
                        let response = InteractionResponse {
                            kind: InteractionResponseType::DeferredUpdateMessage,
                            data: None,
                        };
                        tokio::spawn(
                             http.interaction(interaction.application_id)
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
                    Event::InteractionCreate(interaction) if interaction.kind == InteractionType::ApplicationCommand => {
                        println!("Received InteractionCreate event of type ApplicationCommand");
                        if let (
                            Some(guild_id),
                            Some(InteractionData::ApplicationCommand(ref command))
                         ) = (interaction.guild_id, &interaction.data) {
                            if let Some((language, difficulty)) = is_game_starting(&command) {
                                println!("Starting game in guild {}", guild_id);
                                let localization = localizations.get(language).clone();
                                start_game(&input_sender, localization, difficulty, &interaction);
                                interaction_sender.send(InteractionIds {
                                    id: interaction.id,
                                    app_id: interaction.application_id,
                                    token: interaction.token.clone(),
                                });
                            }
                        } else {
                            println!("Failed to start game: empty guild_id or wrong interaction command");
                        }
                    }
                    _ => {}
                }
            }
        });

        Ok((input_receiver, interaction_receiver))
    }

    pub async fn listen_game(&self, interactions_receiver: Receiver<InteractionIds>) -> Sender<GameRenderEvent> {
        let cached_interactions = Arc::new(Mutex::new(HashMap::new()));
        let cached_interactions_input = Arc::clone(&cached_interactions);
        let http_write = Arc::clone(&self.http_write);
        let (output_sender, output_receiver) = unbounded::<GameRenderEvent>();

        tokio::spawn(async move {
            loop {
                let ev = interactions_receiver.recv_timeout(Duration::from_secs(1));
                if let Ok(ev) = ev {
                    if let Ok(mut cached_interactions_lock) = cached_interactions_input.lock() {
                        cached_interactions_lock.insert(ev.id, ev);
                    }
                }
            }
        });

        tokio::spawn(async move {
            let mut messages = HashMap::new();
            loop {
                let ev = output_receiver.recv_timeout(Duration::from_secs(1));
                if let Ok(ev) = ev {
                    let guild_id = ev.guild_id;

                    let mut interaction_ids = None;
                    if let Ok(cached_interactions_lock) = cached_interactions.lock() {
                        if let Some(_interaction_ids) = cached_interactions_lock.get(&ev.interaction_id) {
                            interaction_ids = Some(_interaction_ids.clone());
                        }
                    }

                    if let Some(interaction_ids) = interaction_ids {
                        if let Some((lower_message_id, cached)) = messages.remove(&guild_id) {
                            match try_update_message(ev, cached, &http_write, &interaction_ids, lower_message_id, guild_id, &mut messages).await {
                                Ok(()) => println!(
                                    "Successfully updated a message in guild {}",
                                    guild_id
                                ),
                                Err(err) => println!(
                                    "Error updating a message in guild {}: {}",
                                    guild_id, err
                                ),
                            }
                        } else {
                            match try_create_message(ev, &http_write, &interaction_ids, guild_id, &mut messages).await {
                                Ok(()) => println!(
                                    "Successfully created a message in guild {}",
                                    guild_id
                                ),
                                Err(err) => println!(
                                    "Error creating a message in guild {}: {}",
                                    guild_id, err
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
