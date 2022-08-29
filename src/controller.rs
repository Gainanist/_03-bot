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

use crate::{
    command_parser::BYGONE_PARTS_FROM_EMOJI_NAME,
    components::PlayerName,
    events::{GameStartEvent, InputEvent, PlayerAttackEvent},
    game_helpers::GameRenderMessage,
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

pub async fn send_game_message(
    http: &Client,
    message_id: Option<&(Id<MessageMarker>, Id<MessageMarker>)>,
    msg: GameRenderMessage,
    channel_id: Id<ChannelMarker>,
) -> Result<Option<(Id<MessageMarker>, Id<MessageMarker>)>, Box<dyn Error + Send + Sync>> {
    match message_id {
        Some((upper_message_id, lower_message_id)) =>
            update_game_message(http, *upper_message_id, *lower_message_id, msg, channel_id).await,
        None => create_game_message(http, msg, channel_id).await,
    }
}

async fn create_game_message(
    http: &Client,
    msg: GameRenderMessage,
    channel_id: Id<ChannelMarker>,
) -> Result<Option<(Id<MessageMarker>, Id<MessageMarker>)>, Box<dyn Error + Send + Sync>> {
    let embeds = msg.embeds;
    let upper_message_id;
    let lower_message_id;
    if let Some(upper_message) = embeds.upper_message {
        upper_message_id = if upper_message.embeds.len() > 0 {
            Some(http
                .create_message(channel_id)
                .embeds(&upper_message.embeds)?
                .components(&upper_message.controls)?
                .exec()
                .await?
                .model()
                .await?
                .id
            )
        } else {
            None
        };
    } else {
        return Err(Box::new(GameCreateError));
    }
    if let Some(lower_message) = embeds.lower_message {
        lower_message_id = if lower_message.embeds.len() > 0 {
            Some(http
                .create_message(channel_id)
                .embeds(&lower_message.embeds)?
                .components(&lower_message.controls)?
                .exec()
                .await?
                .model()
                .await?
                .id
            )
        } else {
            None
        };
    } else {
        return Err(Box::new(GameCreateError));
    }
    if let (Some(upper_message_id), Some(lower_message_id)) = (upper_message_id, lower_message_id) {
        Ok(Some((upper_message_id, lower_message_id)))
    } else {
        Ok(None)
    }
}

async fn update_game_message(
    http: &Client,
    upper_message_id: Id<MessageMarker>,
    lower_message_id: Id<MessageMarker>,
    msg: GameRenderMessage,
    channel_id: Id<ChannelMarker>,
) -> Result<Option<(Id<MessageMarker>, Id<MessageMarker>)>, Box<dyn Error + Send + Sync>> {
    let embeds = msg.embeds;
    let mut deleted = false;
    if let Some(upper_message) = embeds.upper_message {
        http.update_message(channel_id, upper_message_id)
            .embeds(Some(&upper_message.embeds))?
            .components(Some(&upper_message.controls))?  // Components are cleared with an empty slice, None does nothing for them
            .exec()
            .await?;
    } else {
        http.delete_message(channel_id, upper_message_id).exec().await?;
        deleted = true;
    }

    if let Some(lower_message) = embeds.lower_message {
        http.update_message(channel_id, lower_message_id)
            .embeds(None)?
            .embeds(Some(&lower_message.embeds))?
            .components(Some(&lower_message.controls))?
            .exec()
            .await?;
    } else {
        http.delete_message(channel_id, lower_message_id).exec().await?;
        deleted = true;
    }

    if deleted {
        Ok(None)
    } else {
        Ok(Some((upper_message_id, lower_message_id)))
    }
}
