use std::error::Error;
use std::fmt;

use crossbeam_channel::Sender;
use twilight_http::Client;

use twilight_model::http::interaction::{
    InteractionResponse, InteractionResponseData, InteractionResponseType,
};

use twilight_model::{
    application::interaction::{
        message_component::MessageComponentInteractionData, Interaction, InteractionData,
    },
    guild::PartialMember,
    id::{marker::MessageMarker, Id},
};

use crate::discord_renderer::{
    RenderedGame, RenderedGamePure, RenderedMessage, RenderedMessagePure,
};
use crate::game_helpers::{Difficulty, InteractionIds};
use crate::{
    command_parser::BYGONE_PARTS_FROM_EMOJI_NAME,
    components::PlayerName,
    events::{GameStartEvent, InputEvent, PlayerAttackEvent},
    localization::Localization,
    logging::format_time,
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
            flags: Some(msg.flags),
            title: None,
            tts: None,
        }),
    }
}

pub fn process_interaction(interaction: Interaction) -> Option<InputEvent> {
    let emoji_name = match interaction.data {
        Some(InteractionData::MessageComponent(MessageComponentInteractionData {
            custom_id,
            ..
        })) => custom_id,
        Some(_) => {
            println!(
                "{} - controller - ERROR processing interaction with id {}: interaction data is not MessageComponent",
                format_time(),
                interaction.id
            );
            return None;
        }
        None => {
            println!(
                "{} - controller - ERROR processing interaction with id {}: empty interaction data",
                format_time(),
                interaction.id
            );
            return None;
        }
    };
    let (user, user_nick) = match interaction.member {
        Some(PartialMember {
            user: Some(user),
            nick: user_nick,
            ..
        }) => (user, user_nick),
        Some(PartialMember { user: None, .. }) => {
            println!(
                "{} - controller - ERROR processing interaction with id {}: empty user",
                format_time(),
                interaction.id
            );
            return None;
        }
        None => {
            println!(
                "{} - controller - ERROR processing interaction with id {}: empty partial member",
                format_time(),
                interaction.id
            );
            return None;
        }
    };
    let guild_id = match interaction.guild_id {
        Some(guild_id) => guild_id,
        None => {
            println!(
                "{} - controller - ERROR processing interaction with id {}: empty guild id",
                format_time(),
                interaction.id
            );
            return None;
        }
    };

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
    } else {
        println!(
            "{} - controller - ERROR processing interaction with id {}: unknown bygone part emoji",
            format_time(),
            interaction.id
        );
    }
    return None;
}

pub fn start_game(
    sender: &Sender<InputEvent>,
    localization: Localization,
    difficulty: Difficulty,
    interaction: &Interaction,
) {
    if let (
        Some(PartialMember {
            user: Some(ref user),
            nick: ref user_nick,
            ..
        }),
        Some(guild_id),
    ) = (&interaction.member, interaction.guild_id)
    {
        let initial_player_name = PlayerName(
            match &user_nick {
                Some(nick) => nick,
                None => &user.name,
            }
            .to_string(),
        );
        if let Err(err) = sender.send(InputEvent::GameStart(GameStartEvent::new(
            user.id,
            initial_player_name,
            difficulty,
            guild_id,
            interaction.id,
            localization,
        ))) {
            println!(
                "{} - controller - FAILED to send game start event: {}",
                format_time(),
                err
            );
        }
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
            &make_message_interaction_response(oneshot),
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
    println!(
        "{} - controller - Creating game message with interaction id {}",
        format_time(),
        interaction.id
    );
    create_message(http, rendered_game.upper_message, &interaction).await?;
    let followup_id = http
        .interaction(interaction.app_id)
        .create_followup(&interaction.token)
        .embeds(&rendered_game.lower_message.embeds)?
        .components(&rendered_game.lower_message.components)?
        .flags(rendered_game.lower_message.flags)
        .exec()
        .await?
        .model()
        .await?
        .id;
    Ok(followup_id)
}

pub async fn update_game_message(
    http: &Client,
    interaction: &InteractionIds,
    followup_id: Id<MessageMarker>,
    rendered_game: &RenderedGame,
) -> Result<Option<Id<MessageMarker>>, Box<dyn Error + Send + Sync>> {
    println!(
        "{} - controller - Updating game message with interaction id {}",
        format_time(),
        interaction.id
    );
    let mut deleted = false;
    match &rendered_game.upper_message {
        RenderedMessage::Message(message) => {
            http.interaction(interaction.app_id)
                .update_response(&interaction.token)
                .embeds(Some(&message.embeds))?
                .components(Some(&message.components))? // Components are cleared with an empty slice, None does nothing for them
                .exec()
                .await?;
        }
        RenderedMessage::Delete => {
            http.interaction(interaction.app_id)
                .delete_response(&interaction.token)
                .exec()
                .await?;
        }
        RenderedMessage::Skip => {}
    }
    match &rendered_game.lower_message {
        RenderedMessage::Message(message) => {
            http.interaction(interaction.app_id)
                .update_followup(&interaction.token, followup_id)
                .embeds(Some(&message.embeds))?
                .components(Some(&message.components))? // Components are cleared with an empty slice, None does nothing for them
                .exec()
                .await?;
        }
        RenderedMessage::Delete => {
            http.interaction(interaction.app_id)
                .delete_followup(&interaction.token, followup_id)
                .exec()
                .await?;
            deleted = true;
        }
        RenderedMessage::Skip => {}
    }

    if deleted {
        Ok(None)
    } else {
        Ok(Some(followup_id))
    }
}

pub async fn update_game_message_pure(
    http: &Client,
    interaction: &InteractionIds,
    followup_id: Id<MessageMarker>,
    rendered_game: &RenderedGamePure,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    println!(
        "{} - controller - Updating pure game message with interaction id {}",
        format_time(),
        interaction.id
    );
    http.interaction(interaction.app_id)
        .update_response(&interaction.token)
        .embeds(Some(&rendered_game.upper_message.embeds))?
        .components(Some(&rendered_game.upper_message.components))? // Components are cleared with an empty slice, None does nothing for them
        .exec()
        .await?;
    http.interaction(interaction.app_id)
        .update_followup(&interaction.token, followup_id)
        .embeds(Some(&rendered_game.lower_message.embeds))?
        .components(Some(&rendered_game.lower_message.components))? // Components are cleared with an empty slice, None does nothing for them
        .exec()
        .await?;
    Ok(())
}
