use std::{error::Error, fmt::Display};

use derive_new::new;
use rand::{thread_rng, Rng, seq::SliceRandom};
use twilight_model::{
    application::component::{button::ButtonStyle, ActionRow, Button, Component},
    channel::{embed::Embed, ReactionType},
    id::{marker::GuildMarker, Id},
};
use twilight_util::builder::embed::{EmbedBuilder, EmbedFieldBuilder, ImageSource};

use crate::{
    components::{BygonePart, Health},
    events::{OneshotType, OngoingGamePayload},
    game_helpers::FinishedGameStatus,
    localization::{Localization, RenderText},
};

const PROGRESS_BAR_SIZE: usize = 4;
const PROGRESS_BAR_SCALE: f32 = PROGRESS_BAR_SIZE as f32 + 1.0;

fn get_button_style(health: &Health) -> ButtonStyle {
    if health.current() == 0 {
        ButtonStyle::Secondary
    } else if health.current() > health.max() / 2 {
        ButtonStyle::Success
    } else {
        ButtonStyle::Danger
    }
}

fn make_controls_button(emoji: &str, health: &Health) -> Component {
    Component::Button(Button {
        custom_id: Some(emoji.to_owned()),
        disabled: !health.alive(),
        emoji: Some(ReactionType::Unicode {
            name: emoji.to_owned(),
        }),
        label: None,
        style: get_button_style(health),
        url: None,
    })
}

fn render_turn_timer(cur: usize, max: usize) -> String {
    let cur = cur.min(max);
    format!(
        "{}{}",
        ":red_square:".to_owned().repeat(cur),
        ":white_large_square:".to_owned().repeat(max - cur),
    )
}
#[derive(Clone, Debug, new)]
pub struct GameRenderError {
    id: Id<GuildMarker>,
    msg: String,
}

impl Display for GameRenderError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}, guild id: {}", self.msg, self.id)
    }
}

impl Error for GameRenderError {}

#[derive(Clone, Debug)]
pub struct RenderedMessagePure {
    pub embeds: Vec<Embed>,
    pub components: Vec<Component>,
}

#[derive(Clone, Debug)]
pub struct RenderedGamePure {
    pub upper_message: RenderedMessagePure,
    pub lower_message: RenderedMessagePure,
}

#[derive(Clone, Debug)]
pub enum RenderedMessage {
    Message(RenderedMessagePure),
    Skip,
    Delete,
}

impl From<RenderedMessagePure> for RenderedMessage {
    fn from(message: RenderedMessagePure) -> Self {
        Self::Message(message)
    }
}

#[derive(Clone, Debug)]
pub struct RenderedGame {
    pub upper_message: RenderedMessage,
    pub lower_message: RenderedMessage,
}

impl From<RenderedGamePure> for RenderedGame {
    fn from(game: RenderedGamePure) -> Self {
        Self {
            upper_message: game.upper_message.into(),
            lower_message: game.lower_message.into(),
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct DiscordRenderer;

impl DiscordRenderer {
    pub fn render_ongoing_game(
        loc: &Localization,
        payload: &OngoingGamePayload,
    ) -> RenderedGamePure {
        let title = EmbedBuilder::new()
            .description(&loc.title)
            .image(
                ImageSource::url(
                    "https://cdn.discordapp.com/attachments/924690917108121661/1015027218264641556/bygone_03_cropped.gif"
                )
                .unwrap(),
            )
            .build();

        let bygone_status = format!(
            " • {}\n • {}: {}",
            payload.bygone_attack.render_text(loc),
            &loc.core,
            payload.bygone_stage.render_text(loc)
        );
        let sensor = payload.bygone_parts[BygonePart::Sensor].render_text(loc);
        let core = payload.bygone_parts[BygonePart::Core].render_text(loc);
        let gun = payload.bygone_parts[BygonePart::Gun].render_text(loc);
        let right_wing = payload.bygone_parts[BygonePart::RightWing].render_text(loc);
        let left_wing = payload.bygone_parts[BygonePart::LeftWing].render_text(loc);

        let enemies = EmbedBuilder::new()
            .field(EmbedFieldBuilder::new(&loc.status_title, bygone_status).build())
            .field(EmbedFieldBuilder::new(&loc.sensor_title, sensor).inline())
            .field(EmbedFieldBuilder::new(&loc.core_title, core).inline())
            .field(EmbedFieldBuilder::new(&loc.gun_title, gun).inline())
            .field(EmbedFieldBuilder::new(&loc.right_wing_title, right_wing).inline())
            .field(EmbedFieldBuilder::new(&loc.left_wing_title, left_wing).inline())
            .build();

        let controls = vec![
            Component::ActionRow(ActionRow {
                components: vec![
                    make_controls_button("🇸", payload.bygone_parts[BygonePart::Sensor].health()),
                    make_controls_button("🇨", payload.bygone_parts[BygonePart::Core].health()),
                    make_controls_button("🇬", payload.bygone_parts[BygonePart::Gun].health()),
                ],
            }),
            Component::ActionRow(ActionRow {
                components: vec![
                    make_controls_button("🇷", payload.bygone_parts[BygonePart::RightWing].health()),
                    Component::Button(Button {
                        custom_id: Some("status".to_owned()),
                        disabled: true,
                        emoji: None, //Some(ReactionType::Custom { name: (*rng.sample(&AUXILIARY_EMOJIS).unwrap_or(&"")).to_owned(), id: None, animated: false }),
                        label: Some(" ".to_owned()),
                        style: ButtonStyle::Secondary,
                        url: None,
                    }),
                    make_controls_button("🇱", payload.bygone_parts[BygonePart::LeftWing].health()),
                ],
            }),
        ];

        let upper_message = RenderedMessagePure {
            embeds: vec![title, enemies],
            components: controls,
        };

        let turn_progress = EmbedBuilder::new()
            .field(EmbedFieldBuilder::new(
                &loc.turn_progress_title,
                render_turn_timer(0, PROGRESS_BAR_SIZE),
            ))
            .build();

        let battle_log_contents = " • ".to_string() + &payload.battle_log_lines.join("\n • ");
        let log = EmbedBuilder::new()
            .field(EmbedFieldBuilder::new(&loc.log_title, battle_log_contents))
            .build();

        let mut players_embed_builder = EmbedBuilder::new();
        for (name, vitality) in payload.players.iter() {
            players_embed_builder = players_embed_builder.field(EmbedFieldBuilder::new(
                &name.0,
                vitality.health().render_text(loc),
            ));
        }
        let players = players_embed_builder.build();

        let lower_message = RenderedMessagePure {
            embeds: vec![turn_progress, log, players],
            components: Vec::new(),
        };

        RenderedGamePure {
            upper_message,
            lower_message,
        }
    }

    pub fn render_finished_game(loc: &Localization, status: FinishedGameStatus) -> RenderedGame {
        let message = match status {
            FinishedGameStatus::Won => &loc.won.0,
            FinishedGameStatus::Lost => &loc.lost.0,
            FinishedGameStatus::Expired => &loc.expired.choose(&mut rand::thread_rng()).unwrap().0,
        };
        let embed = EmbedBuilder::new().description(message).build();

        RenderedGame {
            upper_message: RenderedMessagePure {
                embeds: vec![embed],
                components: Vec::new(),
            }
            .into(),
            lower_message: RenderedMessage::Delete,
        }
    }

    pub fn render_turn_progress(
        id: Id<GuildMarker>,
        previous: &RenderedGame,
        loc: &Localization,
        progress: f32,
    ) -> Result<RenderedGame, GameRenderError> {
        if let RenderedMessage::Message(mut lower_message) = previous.lower_message.clone() {
            let filled_count =
                ((progress * PROGRESS_BAR_SCALE).round().max(0.0) as usize).min(PROGRESS_BAR_SIZE);
            let progress_bar = render_turn_timer(filled_count, PROGRESS_BAR_SIZE);
            let progress_bar_embed = EmbedBuilder::new()
                .field(EmbedFieldBuilder::new(
                    &loc.turn_progress_title,
                    progress_bar,
                ))
                .build();

            if lower_message.embeds.is_empty() {
                lower_message.embeds.push(progress_bar_embed);
            } else {
                lower_message.embeds[0] = progress_bar_embed;
            }

            Ok(RenderedGame {
                upper_message: RenderedMessage::Skip,
                lower_message: lower_message.into(),
            })
        } else {
            Err(GameRenderError::new(
                id,
                "can't write progress bar for deleted or skipped lower message".to_owned(),
            ))
        }
    }

    pub fn render_oneshot(oneshot_type: OneshotType, loc: &Localization) -> RenderedMessagePure {
        let oneshot_message = match oneshot_type {
            OneshotType::Cooldown(duration_left) => {
                loc.battle_cooldown.insert_duration(&duration_left)
            }
            OneshotType::OtherGameInProgress => loc.other_battle_ongoing.clone(),
        };
        let oneshot_embed = EmbedBuilder::new().description(&oneshot_message.0).build();
        RenderedMessagePure {
            embeds: vec![oneshot_embed],
            components: Vec::new(),
        }
    }
}
