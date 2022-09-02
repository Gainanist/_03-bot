use std::fmt;
use std::{
    collections::HashSet,
    error::Error,
    sync::{Mutex},
};

use crossbeam_channel::Sender;
use twilight_http::{request::channel::reaction::RequestReactionType, Client};
use twilight_model::channel::embed::Embed;
use twilight_model::http::interaction::{InteractionResponse, InteractionResponseType, InteractionResponseData};
use twilight_model::id::marker::InteractionMarker;
use twilight_model::{
    channel::{Reaction, ReactionType},
    gateway::payload::incoming::MessageCreate,
    id::{
        marker::{ChannelMarker, MessageMarker},
        Id,
    },
    user::{CurrentUser, User}, application::{component::{ActionRow, Component, Button, button::ButtonStyle}, interaction::{Interaction, InteractionData, message_component::MessageComponentInteractionData}}, guild::PartialMember,
};

use crate::discord_renderer::{RenderedGamePure, RenderedGame, RenderedMessage, RenderedMessagePure};
use crate::game_helpers::{InteractionIds, Difficulty};
use crate::{
    command_parser::BYGONE_PARTS_FROM_EMOJI_NAME,
    components::PlayerName,
    events::{GameStartEvent, InputEvent, PlayerAttackEvent},
    localization::Localization,
};

#[derive(Clone, Copy, Debug)]
pub struct GameCreateError;

impl fmt::Display for GameCreateError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "need two messages to create a game")
    }
}

impl Error for GameCreateError {}

fn make_message_interaction_response(msg: RenderedMessagePure) -> InteractionResponse {
    InteractionResponse {
        kind: InteractionResponseType::ChannelMessageWithSource,
        data: Some(InteractionResponseData {
            allowed_mentions: None,
            attachments: None,
            choices: None,
            components: Some(msg.components),
            content: None,
            custom_id: None,
            embeds: Some(msg.embeds),
            flags: None,
            title: None,
            tts: None,
        }),
    }
}

pub fn process_interaction(interaction: Interaction) -> Option<InputEvent> {
    if let (
        Some(InteractionData::MessageComponent(MessageComponentInteractionData { custom_id: emoji_name, .. })),
        Some(PartialMember { user: Some(user), nick: user_nick, .. }),
        Some(guild_id),
    ) = (interaction.data, interaction.member, interaction.guild_id) {
        if let Some(bygone_part) = BYGONE_PARTS_FROM_EMOJI_NAME.get(&emoji_name) {
            let user_name = PlayerName(
                match &user_nick {
                    Some(nick) => nick,
                    None => &user.name,
                }
                .to_string(),
            );

            return Some(InputEvent::PlayerAttack(PlayerAttackEvent::new(
                user.id,
                user_name,
                guild_id,
                *bygone_part,
            )));
        }
    }
    return None;
}

pub fn start_game(sender: &Sender<InputEvent>, localization: Localization, difficulty: Difficulty, interaction: &Interaction) {
    if let (
        Some(PartialMember { user: Some(ref user), nick: ref user_nick, .. }),
        Some(guild_id),
    ) = (&interaction.member, interaction.guild_id) {
        let initial_player_name = PlayerName(
            match &user_nick {
                Some(nick) => nick,
                None => &user.name,
            }
            .to_string(),
        );
        sender.send(InputEvent::GameStart(GameStartEvent::new(
            user.id,
            initial_player_name,
            difficulty,
            guild_id,
            interaction.id,
            localization,
        )));
    }
}

pub async fn create_message(
    http: &Client,
    oneshot: RenderedMessagePure,
    interaction: &InteractionIds,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    http.interaction(interaction.app_id)
        .create_response(
            interaction.id,
            &interaction.token,
            &make_message_interaction_response(oneshot.into()),
        )
        .exec()
        .await?;
    Ok(())
}

pub async fn create_game_message(
    http: &Client,
    rendered_game: RenderedGamePure,
    interaction: &InteractionIds,
) -> Result<Id<MessageMarker>, Box<dyn Error + Send + Sync>> {
    println!("Creating game message with interaction id {}", interaction.id);
    create_message(http, rendered_game.upper_message, &interaction).await?;
    let lower_message_id = http.interaction(interaction.app_id)
        .create_followup(&interaction.token)
        .embeds(&rendered_game.lower_message.embeds)?
        .components(&rendered_game.lower_message.components)?
        .exec()
        .await?
        .model()
        .await?
        .id;
    Ok(lower_message_id)
}

pub async fn update_game_message(
    http: &Client,
    interaction: &InteractionIds,
    lower_message_id: Id<MessageMarker>,
    rendered_game: &RenderedGame,
) -> Result<Option<Id<MessageMarker>>, Box<dyn Error + Send + Sync>> {
    println!("Updating game message with interaction id {}", interaction.id);
    let mut deleted = false;
    match &rendered_game.upper_message {
        RenderedMessage::Message(message) => {
            http.interaction(interaction.app_id)
                .update_response(&interaction.token)
                .embeds(Some(&message.embeds))?
                .components(Some(&message.components))?  // Components are cleared with an empty slice, None does nothing for them
                .exec()
                .await?;
        },
        RenderedMessage::Delete => {
            http.interaction(interaction.app_id)
                .delete_response(&interaction.token)
                .exec()
                .await?;
        },
        RenderedMessage::Skip => {},
    }
    match &rendered_game.lower_message {
        RenderedMessage::Message(message) => {
            http.interaction(interaction.app_id)
                .update_followup(&interaction.token, lower_message_id)
                .embeds(Some(&message.embeds))?
                .components(Some(&message.components))?  // Components are cleared with an empty slice, None does nothing for them
                .exec()
                .await?;
        },
        RenderedMessage::Delete => {
            http.interaction(interaction.app_id)
                .delete_followup(&interaction.token, lower_message_id)
                .exec()
                .await?;
            deleted = true;
        },
        RenderedMessage::Skip => {},
    }

    if deleted {
        Ok(None)
    } else {
        Ok(Some(lower_message_id))
    }
}