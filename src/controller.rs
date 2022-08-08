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
    let embeds = msg.embeds;
    match message_id {
        Some((upper_message_id, lower_message_id)) => {
            let mut deleted = false;
            if embeds.upper_embeds.len() > 0 {
                let mut update_message = http.update_message(channel_id, *upper_message_id)
                    .embeds(None)?
                    .embeds(Some(&embeds.upper_embeds))?
                    .components(None)?;
                if embeds.controls.len() > 0 {
                    update_message = update_message.components(Some(&embeds.controls))?;
                }
                update_message.components(None)?.exec().await?;
            } else {
                http.delete_message(channel_id, *upper_message_id).exec().await?;
                deleted = true;
            }

            if embeds.lower_embeds.len() > 0 {
                http.update_message(channel_id, *lower_message_id)
                    .embeds(None)?
                    .embeds(Some(&embeds.lower_embeds))?
                    .exec()
                    .await?;
            } else {
                http.delete_message(channel_id, *lower_message_id).exec().await?;
                deleted = true;
            }

            if deleted {
                Ok(None)
            } else {
                Ok(Some((*upper_message_id, *lower_message_id)))
            }
        }
        None => {
            let upper_message_id = None;
            if embeds.upper_embeds.len() > 0 {
                let mut upper_message = http
                    .create_message(channel_id)
                    .embeds(&embeds.upper_embeds)?;
                if embeds.controls.len() > 0 {
                    upper_message = upper_message.components(&embeds.controls)?;
                }
                upper_message_id = Some(upper_message
                    .exec()
                    .await?
                    .model()
                    .await?
                    .id
                );
            }
            let lower_message_id = None;
            if embeds.lower_embeds.len() > 0 {
                lower_message_id = Some(http
                    .create_message(channel_id)
                    .embeds(&embeds.lower_embeds)?
                    .exec()
                    .await?
                    .model()
                    .await?
                    .id
                );
            }
            if (Some(upper_message_id), Some(lower_message_id)) - (upper_message_id, lower_message_id) {
                Ok((upper_message_id, lower_message_id))
            } else {
                Ok(None)
            }
        }
    }
}
