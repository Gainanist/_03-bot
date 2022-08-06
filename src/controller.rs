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
            let mut action_row = ActionRow{ components: Vec::with_capacity(5) };
            for emoji_name in BYGONE_PARTS_FROM_EMOJI_NAME.keys() {
                action_row.components.push(Component::Button(Button {
                    custom_id: Some((*emoji_name).to_owned()),
                    disabled: false,
                    emoji: Some(ReactionType::Unicode { name: (*emoji_name).to_owned() }),
                    label: None,
                    style: ButtonStyle::Secondary,
                    url: None,
                }));
            }
            let message_id = http
                .create_message(channel_id)
                .embeds(&msg.embeds.render())?
                .components(&[Component::ActionRow(action_row)])?
                .exec()
                .await?
                .model()
                .await?
                .id;
            Ok(message_id)
        }
    }
}
