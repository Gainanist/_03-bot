use std::{
    collections::HashMap,
    error::Error,
    sync::{Arc, Mutex},
    time::Duration, future::IntoFuture,
};

use crossbeam_channel::{unbounded, Receiver, Sender};

use twilight_gateway::{Intents, Shard, ShardId, Event};
use twilight_http::Client as HttpClient;
use twilight_model::{
    application::{
        command::{CommandOption, CommandOptionChoice, CommandOptionType, CommandOptionChoiceValue},
        interaction::{InteractionData, InteractionType},
    },
    http::interaction::{InteractionResponse, InteractionResponseType},
};

use tracing;

use crate::{
    command_parser::{
        is_game_starting, BATTLE_COMMAND, DIFFICULTY_COMMAND_OPTION, LANGUAGE_COMMAND_OPTION,
    },
    controller::{
        create_game_message, create_message, process_interaction, start_game, update_game_message,
        update_game_message_pure,
    },
    discord_renderer::{DiscordRenderer, RenderedGame, RenderedMessage},
    events::{GameRenderEvent, GameRenderPayload, InputEvent},
    game_helpers::{Difficulty, InteractionIds},
    localization::{Language, Localizations},
    logging::format_time,
};

fn merge_with_cached(rendered_game: RenderedGame, cached: &mut RenderedGame) {
    match rendered_game.upper_message {
        RenderedMessage::Message(message) => {
            cached.upper_message = message.into();
        }
        RenderedMessage::Skip => {}
        RenderedMessage::Delete => {
            cached.upper_message = RenderedMessage::Delete;
        }
    }
    match rendered_game.lower_message {
        RenderedMessage::Message(message) => {
            cached.lower_message = message.into();
        }
        RenderedMessage::Skip => {}
        RenderedMessage::Delete => {
            cached.lower_message = RenderedMessage::Delete;
        }
    }
}

pub struct DiscordClient {
    http_write: Arc<HttpClient>,
    http_read: Arc<HttpClient>,
}

impl DiscordClient {
    pub fn new(token: String) -> (Self, Shard) {
        let shard = Shard::new(ShardId::ONE, token.to_owned(), Intents::empty());

        let http_read = Arc::new(HttpClient::new(token.to_owned()));
        let http_write = Arc::new(HttpClient::new(token));

        (
            Self {
                http_write,
                http_read,
            },
            shard
        )
    }

    pub fn register_commands(&self) {
        async fn inner(http: Arc<HttpClient>) -> Result<(), Box<dyn Error + Sync + Send>> {
            let app_id = http
                .current_user_application()
                .await?
                .model()
                .await?
                .id;
            println!(
                "{} - discord_client - Registering commands for app id {}",
                format_time(),
                app_id
            );
            http.interaction(app_id)
                .create_global_command()
                .chat_input(BATTLE_COMMAND, "Fight _03")?
                .description_localizations(&HashMap::from([(
                    Language::Ru.to_string(),
                    "Сразиться с _03".to_owned(),
                )]))?
                .command_options(&[
                    CommandOption {
                        autocomplete: Some(false),
                        channel_types: None,
                        choices: Some(vec![
                            CommandOptionChoice {
                                name: "Easy - Just like in the game".to_owned(),
                                name_localizations: Some(HashMap::from([(
                                    Language::Ru.to_string(),
                                    "Легко - Совсем как в игре".to_owned(),
                                )])),
                                value: CommandOptionChoiceValue::String(Difficulty::Easy.to_string()),
                            },
                            CommandOptionChoice {
                                name: "Medium - Take a buddy with you".to_owned(),
                                name_localizations: Some(HashMap::from([(
                                    Language::Ru.to_string(),
                                    "Средне - Позови друга".to_owned(),
                                )])),
                                value: CommandOptionChoiceValue::String(Difficulty::Medium.to_string()),
                            },
                            CommandOptionChoice {
                                name: "Hard - You shall not pass!".to_owned(),
                                name_localizations: Some(HashMap::from([(
                                    Language::Ru.to_string(),
                                    "Сложно - Ты не пройдёшь!".to_owned(),
                                )])),
                                value: CommandOptionChoiceValue::String(Difficulty::Hard.to_string()),
                            },
                            CommandOptionChoice {
                                name: "Real bullets - Forgive me, Mister Pikes...".to_owned(),
                                name_localizations: Some(HashMap::from([(
                                    Language::Ru.to_string(),
                                    "Боевые патроны - Простите меня, мистер Пайкс...".to_owned(),
                                )])),
                                value: CommandOptionChoiceValue::String(Difficulty::RealBullets.to_string()),
                            },
                        ]),
                        description: "Battle difficulty".to_owned(),
                        description_localizations: Some(HashMap::from([(
                            Language::Ru.to_string(),
                            "Сложность битвы".to_owned(),
                        )])),
                        kind: CommandOptionType::String,
                        max_length: None,
                        min_length: None,
                        min_value: None,
                        max_value: None,
                        name: DIFFICULTY_COMMAND_OPTION.to_owned(),
                        name_localizations: Some(HashMap::from([(
                            Language::Ru.to_string(),
                            "сложность".to_owned(),
                        )])),
                        options: None,
                        required: Some(false),
                    },
                    CommandOption {
                        autocomplete: Some(false),
                        channel_types: None,
                        choices: Some(vec![
                            CommandOptionChoice {
                                name: "English".to_owned(),
                                name_localizations: Some(HashMap::from([(
                                    Language::Ru.to_string(),
                                    "Английский".to_owned(),
                                )])),
                                value: CommandOptionChoiceValue::String(Language::En.to_string()),
                            },
                            CommandOptionChoice {
                                name: "Russian".to_owned(),
                                name_localizations: Some(HashMap::from([(
                                    Language::Ru.to_string(),
                                    "Русский".to_owned(),
                                )])),
                                value: CommandOptionChoiceValue::String(Language::Ru.to_string()),
                            },
                        ]),
                        description: "Interface language".to_owned(),
                        description_localizations: Some(HashMap::from([(
                            Language::Ru.to_string(),
                            "Язык интерфейса".to_owned(),
                        )])),
                        kind: CommandOptionType::String,
                        max_length: None,
                        min_length: None,
                        min_value: None,
                        max_value: None,
                        name: LANGUAGE_COMMAND_OPTION.to_owned(),
                        name_localizations: Some(HashMap::from([(
                            Language::Ru.to_string(),
                            "язык".to_owned(),
                        )])),
                        options: None,
                        required: Some(false),
                    },
                ])?
                .await?;
            println!(
                "{} - discord_client - Commands register success",
                format_time()
            );
            Ok(())
        }

        let http = Arc::clone(&self.http_write);
        tokio::spawn(async move {
            inner(http).await.unwrap();
        });
    }

    pub async fn listen_discord(
        &self,
        mut shard: Shard
    ) -> Result<(Receiver<InputEvent>, Receiver<InteractionIds>), Box<dyn Error + Send + Sync>>
    {
        let http = Arc::clone(&self.http_read);
        let (input_sender, input_receiver) = unbounded();
        let (interaction_sender, interaction_receiver) = unbounded();

        tokio::spawn(async move {
            let localizations = Localizations::new();
            // Process each event as they come in.
            loop {
                let event = match shard.next_event().await {
                    Ok(event) => event,
                    Err(source) => {
                        tracing::warn!(?source, "error receiving event");
        
                        if source.is_fatal() {
                            break;
                        }

                        continue;
                    }
                };
                match event {
                    Event::InteractionCreate(interaction)
                        if interaction.kind == InteractionType::MessageComponent =>
                    {
                        println!(
                            "{} - discord_client - Received InteractionCreate event of type MessageComponent, id {}",
                            format_time(),
                            interaction.id
                        );
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
                                .into_future()
                        );
                        match process_interaction(interaction.0) {
                            Some(ev) => {
                                if let Err(err) = input_sender.send(ev) {
                                    println!(
                                        "{} - discord_client - FAILED to send input event: {}",
                                        format_time(),
                                        err
                                    );
                                }
                            }
                            None => {
                                println!(
                                    "{} - discord_client - FAILED to process MessageComponent interaction",
                                    format_time()
                                );
                            }
                        }
                    }
                    Event::InteractionCreate(interaction)
                        if interaction.kind == InteractionType::ApplicationCommand =>
                    {
                        println!(
                            "{} - discord_client - Received InteractionCreate event of type ApplicationCommand, id {}",
                            format_time(),
                            interaction.id
                        );
                        if let (
                            Some(guild_id),
                            Some(InteractionData::ApplicationCommand(ref command)),
                        ) = (interaction.guild_id, &interaction.data)
                        {
                            if let Some((language, difficulty)) = is_game_starting(&command) {
                                println!(
                                    "{} - discord_client - Attempting to start game in guild {} with lang {} and difficulty {}",
                                    format_time(),
                                    guild_id,
                                    language.to_string(),
                                    difficulty.to_string(),
                                );
                                let localization = localizations.get(language).clone();
                                start_game(&input_sender, localization, difficulty, &interaction);
                                if let Err(err) = interaction_sender.send(InteractionIds {
                                    id: interaction.id,
                                    app_id: interaction.application_id,
                                    token: interaction.token.clone(),
                                }) {
                                    println!(
                                        "{} - discord_client - FAILED to send interaction ids: {}",
                                        format_time(),
                                        err
                                    );
                                }
                            }
                        } else {
                            println!(
                                "{} - discord_client - Failed to start game: empty guild_id or wrong interaction command", format_time()
                            );
                        }
                    }
                    _ => {}
                }
            }
        });

        Ok((input_receiver, interaction_receiver))
    }

    pub async fn listen_game(
        &self,
        interactions_receiver: Receiver<InteractionIds>,
    ) -> Sender<GameRenderEvent> {
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
                        if let Some(_interaction_ids) =
                            cached_interactions_lock.get(&ev.interaction_id)
                        {
                            interaction_ids = Some(_interaction_ids.clone());
                        }
                    }

                    if let Some(interaction_ids) = interaction_ids {
                        match ev.payload {
                            GameRenderPayload::OngoingGame(payload) => {
                                let rendered_game =
                                    DiscordRenderer::render_ongoing_game(&ev.loc, &payload);
                                match messages.get_mut(&guild_id) {
                                    Some((cached_interaction_id, followup_id, cached))
                                        if interaction_ids.id == *cached_interaction_id =>
                                    {
                                        match update_game_message_pure(
                                            &http_write,
                                            &interaction_ids,
                                            *followup_id,
                                            &rendered_game,
                                        )
                                        .await
                                        {
                                            Ok(()) => {
                                                println!("{} - discord_client - Updated game message with interaction id {}", format_time(), interaction_ids.id);
                                                merge_with_cached(rendered_game.into(), cached);
                                            }
                                            Err(err) => {
                                                println!("{} - discord_client - ERROR updating game message with interaction id {}: {}", format_time(), interaction_ids.id, err);
                                            }
                                        }
                                    }
                                    _ => {
                                        match create_game_message(
                                            &http_write,
                                            rendered_game.clone(),
                                            &interaction_ids,
                                        )
                                        .await
                                        {
                                            Ok(followup_id) => {
                                                println!("{} - discord_client - Created game message with interaction id {}", format_time(), interaction_ids.id);
                                                messages.insert(
                                                    guild_id,
                                                    (
                                                        interaction_ids.id,
                                                        followup_id,
                                                        rendered_game.into(),
                                                    ),
                                                );
                                            }
                                            Err(err) => {
                                                println!("{} - discord_client - ERROR creating game message with interaction id {}: {}", format_time(), interaction_ids.id, err);
                                            }
                                        }
                                    }
                                }
                            }
                            GameRenderPayload::FinishedGame(status) => {
                                let rendered_game =
                                    DiscordRenderer::render_finished_game(&ev.loc, status);
                                let mut remove = false;
                                match messages.get(&guild_id) {
                                    Some((cached_interaction_id, followup_id, _cached))
                                        if interaction_ids.id == *cached_interaction_id =>
                                    {
                                        remove = true;
                                        match update_game_message(
                                            &http_write,
                                            &interaction_ids,
                                            *followup_id,
                                            &rendered_game,
                                        )
                                        .await
                                        {
                                            Ok(_) => {
                                                println!("{} - discord_client - Cleanup rendered game cache with interaction id {}", format_time(), interaction_ids.id);
                                            }
                                            Err(err) => {
                                                println!("{} - discord_client - ERROR updating finished game with interaction id {}: {}", format_time(), interaction_ids.id, err);
                                            }
                                        }
                                    }
                                    _ => {
                                        println!("{} - discord_client - ERROR updating finished game with interaction id {}: cache not found", format_time(), interaction_ids.id);
                                    }
                                }
                                if remove {
                                    messages.remove(&guild_id);
                                }
                            }
                            GameRenderPayload::TurnProgress(progress) => {
                                match messages.get_mut(&guild_id) {
                                    Some((cached_interaction_id, followup_id, cached))
                                        if interaction_ids.id == *cached_interaction_id =>
                                    {
                                        match DiscordRenderer::render_turn_progress(
                                            guild_id, cached, &ev.loc, progress,
                                        ) {
                                            Ok(rendered_game) => {
                                                match update_game_message(
                                                    &http_write,
                                                    &interaction_ids,
                                                    *followup_id,
                                                    &rendered_game,
                                                )
                                                .await
                                                {
                                                    Ok(Some(_)) => {
                                                        println!("{} - discord_client - Updated progress bar with interaction id {}", format_time(), interaction_ids.id);
                                                        merge_with_cached(rendered_game, cached);
                                                    }
                                                    Ok(None) => {}
                                                    Err(err) => {
                                                        println!("{} - discord_client - ERROR updating progress bar with interaction id {}: {}", format_time(), interaction_ids.id, err);
                                                    }
                                                }
                                            }
                                            Err(err) => {
                                                println!("{} - discord_client - ERROR updating progress bar with interaction id {}: {}", format_time(), interaction_ids.id, err);
                                            }
                                        }
                                    }
                                    _ => {
                                        println!("{} - discord_client - ERROR updating progress bar with interaction id {}: cache not found", format_time(), interaction_ids.id);
                                    }
                                }
                            }
                            GameRenderPayload::OneshotMessage(oneshot_type) => {
                                let oneshot_message =
                                    DiscordRenderer::render_oneshot(oneshot_type, &ev.loc);
                                match create_message(&http_write, oneshot_message, &interaction_ids).await {
                                    Ok(()) => println!("{} - discord_client - Created oneshot message with interaction id {}", format_time(), interaction_ids.id),
                                    Err(err) =>
                                        println!("{} - discord_client - ERROR creating oneshot message with interaction id {}: {}", format_time(), interaction_ids.id, err),
                                }
                            }
                        }
                    }
                }
            }
        });

        output_sender
    }
}
