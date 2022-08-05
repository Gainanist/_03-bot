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
    user::CurrentUser,
};

use crate::{
    command_parser::BYGONE_PARTS_FROM_EMOJI_NAME,
    components::PlayerName,
    events::{GameStartEvent, InputEvent, PlayerAttackEvent},
    game_helpers::GameRenderMessage,
    localization::Localization,
};

pub fn process_reaction(
    reaction: &Reaction,
    sender: &Sender<InputEvent>,
    current_user: &CurrentUser,
    game_message_ids: &Mutex<HashSet<Id<MessageMarker>>>,
) {
    if reaction.user_id == current_user.id {
        return;
    }
    if let Ok(game_message_ids_lock) = game_message_ids.lock() {
        if !game_message_ids_lock.contains(&reaction.message_id) {
            return;
        }
    } else {
        return;
    }

    if let ReactionType::Unicode { name } = &reaction.emoji {
        if let Some(bygone_part) = BYGONE_PARTS_FROM_EMOJI_NAME.get(name) {
            if let Some(guild) = reaction.guild_id {
                let user_name = PlayerName(
                    match &reaction.member {
                        Some(member) => match &member.nick {
                            Some(nick) => nick,
                            None => &member.user.name,
                        },
                        None => "Anon",
                    }
                    .to_string(),
                );

                sender.send(InputEvent::PlayerAttack(PlayerAttackEvent::new(
                    reaction.user_id,
                    user_name,
                    reaction.guild_id.unwrap(),
                    *bygone_part,
                )));
            }
        }
    }
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
    message_id: Option<&Id<MessageMarker>>,
    msg: GameRenderMessage,
    channel_id: Id<ChannelMarker>,
) -> Result<Id<MessageMarker>, Box<dyn Error + Send + Sync>> {
    match message_id {
        Some(message_id) => {
            http.update_message(channel_id, *message_id)
                .embeds(None)?
                .embeds(Some(&msg.embeds.render()))?
                .exec()
                .await?;
            Ok(*message_id)
        }
        None => {
            let message_id = http
                .create_message(channel_id)
                .embeds(&msg.embeds.render())?
                .exec()
                .await?
                .model()
                .await?
                .id;
            for emoji_name in BYGONE_PARTS_FROM_EMOJI_NAME.keys() {
                http.create_reaction(
                    channel_id,
                    message_id,
                    &RequestReactionType::Unicode { name: emoji_name },
                )
                .exec()
                .await?;
            }
            Ok(message_id)
        }
    }
}
