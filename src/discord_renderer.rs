use std::collections::HashMap;

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

fn make_button(emoji: &str, health: &Health) -> Component {
    Component::Button(Button {
        custom_id: Some(emoji.to_owned()),
        disabled: !health.alive(),
        emoji: Some(ReactionType::Unicode { name: emoji.to_owned() }),
        label: None,
        style: get_button_style(health),
        url: None,
    })
}

#[derive(Clone, Debug)]
pub enum RenderedMessage {
    Message {
        embeds: Vec<Embed>,
        components: Vec<Component>,
    },
    Skip,
    Delete,
}

#[derive(Clone, Debug)]
pub struct RenderedGame {
    pub upper_message: RenderedMessage,
    pub lower_message: RenderedMessage,
}

#[derive(Clone, Debug, Default)]
pub struct DiscordRenderer {
    rendered_games_cache: HashMap<Id<GuildMarker>, RenderedGame>,
}

impl DiscordRenderer {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn render(&mut self, ev: GameRenderEvent) -> RenderedGame {
        let rendered_game = match ev.payload {
            GameRenderPayload::OngoingGame(payload) => {
                println!("Rendering ongoing game in guild {}", ev.id);
                self.render_ongoing_game(&ev.loc, &payload)
            },
            GameRenderPayload::FinishedGame(status) => {
                println!("Rendering finished game in guild {}", ev.id);
                self.render_finished_game(&ev.loc, status)
            },
            GameRenderPayload::TurnProgress(progress) => todo!(),
        };
        self.rendered_games_cache.insert(ev.id, rendered_game.clone());

        rendered_game
    }

    fn render_ongoing_game(&self, loc: &Localization, payload: &OngoingGamePayload) -> RenderedGame {
        let title = EmbedBuilder::new()
            .description(&loc.title)
            .image(
                ImageSource::url(
                    "http://www.uof7.com/wp-content/uploads/2016/09/15-Bygone-UPD.gif",
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
                    make_button("🇸", payload.bygone_parts[BygonePart::Sensor].health()),
                    make_button("🇨", payload.bygone_parts[BygonePart::Core].health()),
                    make_button("🇬", payload.bygone_parts[BygonePart::Gun].health()),
                ],
            }),
            Component::ActionRow(ActionRow {
                components: vec! [
                    make_button("🇷", payload.bygone_parts[BygonePart::RightWing].health()),
                    Component::Button(Button {
                        custom_id: Some("status".to_owned()),
                        disabled: true,
                        emoji: None, //Some(ReactionType::Custom { name: (*rng.sample(&AUXILIARY_EMOJIS).unwrap_or(&"")).to_owned(), id: None, animated: false }),
                        label: Some(" ".to_owned()),
                        style: ButtonStyle::Secondary,
                        url: None,
                    }),
                    make_button("🇱", payload.bygone_parts[BygonePart::LeftWing].health()),
                ],
            }),
        ];

        let upper_message = RenderedMessage::Message {
            embeds: vec! [ title, enemies ],
            components: controls,
        };


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

        let lower_message = RenderedMessage::Message {
            embeds: vec! [ log, players ],
            components: Vec::new(),
        };

        RenderedGame { upper_message, lower_message }
    }

    fn render_finished_game(&self, loc: &Localization, status: FinishedGameStatus) -> RenderedGame {
        let message = match status {
            FinishedGameStatus::Won => &loc.won.0,
            FinishedGameStatus::Lost => &loc.lost.0,
        };
        let embed = EmbedBuilder::new().description(message).build();

        RenderedGame {
            upper_message: RenderedMessage::Delete,
            lower_message: RenderedMessage::Message {
                embeds: vec! [ embed ],
                components: Vec::new(),
            },
        }
    }

    fn render_turn_progress(&self, id: Id<GuildMarker>, loc: &Localization, pprogress: &f32) -> RenderedGame {

    }
}
