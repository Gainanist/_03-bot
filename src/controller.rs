use std::fmt;
use std::{
    collections::HashSet,
    error::Error,
    sync::{mpsc::Sender, Mutex},
};

use twilight_http::{request::channel::reaction::RequestReactionType, Client};
use twilight_model::{
    channel::{Reaction, ReactionType},
    gateway::payload::incoming::MessageCreate,
    id::{
        marker::{ChannelMarker, MessageMarker},
        Id,
    },
    user::{CurrentUser, User}, application::{component::{ActionRow, Component, Button, button::ButtonStyle}, interaction::{Interaction, InteractionData, message_component::MessageComponentInteractionData}}, guild::PartialMember,
};

use crate::discord_renderer::{RenderedGamePure, RenderedGame, RenderedMessage};
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

pub fn start_game(sender: &Sender<InputEvent>, localization: Localization, msg: &MessageCreate) {
    if let Some(guild) = msg.guild_id {
        let initial_player_name = PlayerName(
            match &msg.member {
                Some(member) => match &member.nick {
                    Some(nick) => nick,
                    None => &msg.author.name,
                },
                None => &msg.author.name,
            }
            .to_string(),
        );
        sender.send(InputEvent::GameStart(GameStartEvent::new(
            msg.author.id,
            initial_player_name,
            guild,
            localization,
        )));
    }
}

pub async fn create_game_message(
    http: &Client,
    rendered_game: &RenderedGamePure,
    channel_id: Id<ChannelMarker>,
) -> Result<(Id<MessageMarker>, Id<MessageMarker>), Box<dyn Error + Send + Sync>> {
    let upper_message_id = http
        .create_message(channel_id)
        .embeds(&rendered_game.upper_message.embeds)?
        .components(&rendered_game.upper_message.components)?
        .exec()
        .await?
        .model()
        .await?
        .id;
    let lower_message_id = http
        .create_message(channel_id)
        .embeds(&rendered_game.lower_message.embeds)?
        .components(&rendered_game.lower_message.components)?
        .exec()
        .await?
        .model()
        .await?
        .id;
    Ok((upper_message_id, lower_message_id))
}

pub async fn update_game_message(
    http: &Client,
    upper_message_id: Id<MessageMarker>,
    lower_message_id: Id<MessageMarker>,
    rendered_game: &RenderedGame,
    channel_id: Id<ChannelMarker>,
) -> Result<Option<(Id<MessageMarker>, Id<MessageMarker>)>, Box<dyn Error + Send + Sync>> {
    let mut deleted = false;
    match &rendered_game.upper_message {
        RenderedMessage::Message(message) => {
            http.update_message(channel_id, upper_message_id)
                .embeds(Some(&message.embeds))?
                .components(Some(&message.components))?  // Components are cleared with an empty slice, None does nothing for them
                .exec()
                .await?;
        },
        RenderedMessage::Delete => {
            http.delete_message(channel_id, upper_message_id).exec().await?;
            deleted = true;
        },
        RenderedMessage::Skip => {},
    }
    match &rendered_game.lower_message {
        RenderedMessage::Message(message) => {
            http.update_message(channel_id, lower_message_id)
                .embeds(Some(&message.embeds))?
                .components(Some(&message.components))?  // Components are cleared with an empty slice, None does nothing for them
                .exec()
                .await?;
        },
        RenderedMessage::Delete => {
            http.delete_message(channel_id, lower_message_id).exec().await?;
            deleted = true;
        },
        RenderedMessage::Skip => {},
    }

    if deleted {
        Ok(None)
    } else {
        Ok(Some((upper_message_id, lower_message_id)))
    }
}
