use std::{collections::HashMap, fmt::Display, error::Error};

use derive_new::new;
use twilight_model::{channel::{embed::Embed, ReactionType}, application::component::{Component, ActionRow, button::ButtonStyle, Button}, id::{Id, marker::GuildMarker}};
use twilight_util::builder::embed::{EmbedBuilder, EmbedFieldBuilder, ImageSource};

use crate::{events::{GameRenderEvent, GameRenderPayload, OngoingGamePayload}, game_helpers::FinishedGameStatus, localization::{Localization, RenderText}, components::{BygonePart, Health}};

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
        emoji: Some(ReactionType::Unicode { name: emoji.to_owned() }),
        label: None,
        style: get_button_style(health),
        url: None,
    })
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
pub struct  RenderedMessagePure {
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
    pub fn render(ev: GameRenderEvent) -> Result<RenderedGamePure, GameRenderError> {
        match ev.payload {
            GameRenderPayload::OngoingGame(payload) => {
                println!("Rendering ongoing game in guild {}", ev.guild_id);
                Ok(Self::render_ongoing_game(&ev.loc, &payload))
            },
            GameRenderPayload::FinishedGame(_) => {
                Err(GameRenderError::new(ev.guild_id, "can't render finished game without previous rendered game".to_owned()))
            },
            GameRenderPayload::TurnProgress(_) => {
                Err(GameRenderError::new(ev.guild_id, "can't render turn progress without previous rendered game".to_owned()))
            },
        }
    }

    pub fn render_with_previous(ev: GameRenderEvent, previous: &RenderedGame) -> Result<RenderedGame, GameRenderError> {
        match ev.payload {
            GameRenderPayload::OngoingGame(payload) => {
                println!("Rendering ongoing game in guild {}", ev.guild_id);
                Ok(Self::render_ongoing_game(&ev.loc, &payload).into())
            },
            GameRenderPayload::FinishedGame(status) => {
                println!("Rendering finished game in guild {}", ev.guild_id);
                Ok(Self::render_finished_game(&ev.loc, status))
            },
            GameRenderPayload::TurnProgress(progress) => {
                println!("Rendering turn progress in guild {}", ev.guild_id);
                Self::render_turn_progress(ev.guild_id, previous, &ev.loc, progress)
            },
        }
    }

    fn render_ongoing_game(loc: &Localization, payload: &OngoingGamePayload) -> RenderedGamePure {
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
            .field(
                EmbedFieldBuilder::new(&loc.right_wing_title, right_wing)
                    .inline(),
            )
            .field(
                EmbedFieldBuilder::new(&loc.left_wing_title, left_wing)
                    .inline(),
            )
            .build();

        let controls = vec! [
            Component::ActionRow(ActionRow {
                components: vec! [
                    make_controls_button("🇸", payload.bygone_parts[BygonePart::Sensor].health()),
                    make_controls_button("🇨", payload.bygone_parts[BygonePart::Core].health()),
                    make_controls_button("🇬", payload.bygone_parts[BygonePart::Gun].health()),
                ],
            }),
            Component::ActionRow(ActionRow {
                components: vec! [
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
            embeds: vec! [ title, enemies ],
            components: controls,
        };

        let turn_progress = EmbedBuilder::new()
            .field(EmbedFieldBuilder::new(&loc.turn_progress_title, "[▯▯▯▯▯▯▯▯]"))
            .build();

        let battle_log_contents = " • ".to_string() + &payload.battle_log_lines.join("\n • ");
        let log = EmbedBuilder::new()
            .field(EmbedFieldBuilder::new(&loc.log_title, battle_log_contents))
            .build();

        let mut players_embed_builder = EmbedBuilder::new();
        for (name, vitality) in payload.players.iter() {
            players_embed_builder = players_embed_builder.field(
                EmbedFieldBuilder::new(&name.0, vitality.health().render_text(loc)),
            );
        }
        let players = players_embed_builder.build();

        let lower_message = RenderedMessagePure {
            embeds: vec! [ turn_progress, log, players ],
            components: Vec::new(),
        };

        RenderedGamePure { upper_message, lower_message }
    }

    fn render_finished_game(loc: &Localization, status: FinishedGameStatus) -> RenderedGame {
        let message = match status {
            FinishedGameStatus::Won => &loc.won.0,
            FinishedGameStatus::Lost => &loc.lost.0,
        };
        let embed = EmbedBuilder::new().description(message).build();

        RenderedGame {
            upper_message: RenderedMessage::Delete,
            lower_message: RenderedMessagePure {
                embeds: vec! [ embed ],
                components: Vec::new(),
            }.into(),
        }
    }

    fn render_turn_progress(id: Id<GuildMarker>, previous: &RenderedGame, loc: &Localization, progress: f32) -> Result<RenderedGame, GameRenderError> {
        if let RenderedMessage::Message(mut lower_message) = previous.lower_message.clone() {
            let filled_count = ((progress / 0.1).round() as isize).max(0).min(8) as usize;
            let progress_bar = match filled_count {
                0 => "[        ]".to_owned(),
                1..=8 => format!("[{}>{}]", "▮".to_owned().repeat(filled_count - 1), "▯".to_owned().repeat(8 - filled_count)),
                _ => "[▮▮▮▮▮▮▮>]".to_owned(),
            };
            let progress_bar_embed = EmbedBuilder::new()
                .field(EmbedFieldBuilder::new(&loc.turn_progress_title, progress_bar))
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
            Err(GameRenderError::new(id, "can't write progress bar for deleted or skipped lower message".to_owned()))
        }
    }
}
