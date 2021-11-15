mod entities;
mod game;
mod bygone_03;
mod localization;
mod command_parser;
mod language;
mod command;
mod player_action;

use std::{env, error::Error, slice::SliceIndex, sync::Arc};
use command_parser::parse_command;
use futures::stream::StreamExt;
use game::Game;

use localization::{Localizations, Localize};
use twilight_cache_inmemory::{InMemoryCache, ResourceType};
use twilight_gateway::{cluster::{Cluster, ShardScheme}, Event};
use twilight_http::Client as HttpClient;
use twilight_model::{gateway::{Intents}, id::ChannelId};

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error + Send + Sync>> {
    let token = env::var("DISCORD_TOKEN")?;

    // This is the default scheme. It will automatically create as many
    // shards as is suggested by Discord.
    let scheme = ShardScheme::Auto;

    // Use intents to only receive guild message events.
    let (cluster, mut events) = Cluster::builder(token.to_owned(), Intents::GUILD_MESSAGES)
        .shard_scheme(scheme)
        .build()
        .await?;
    let cluster = Arc::new(cluster);

    // Start up the cluster.
    let cluster_spawn = Arc::clone(&cluster);

    // Start all shards in the cluster in the background.
    tokio::spawn(async move {
        cluster_spawn.up().await;
    });

    // HTTP is separate from the gateway, so create a new client.
    let http = Arc::new(HttpClient::new(token));

    // Since we only care about new messages, make the cache only
    // cache new messages.
    let cache = InMemoryCache::builder()
        .resource_types(ResourceType::MESSAGE)
        .build();

    let mut event_handler = EventHandler::new();
    // Process each event as they come in.
    while let Some((shard_id, event)) = events.next().await {
        // Update the cache with the event.
        cache.update(&event);

        event_handler.handle(
            shard_id,
            event,
            Arc::clone(&http),
        ).await?;
    }

    Ok(())
}

struct EventHandler {
    localizations: Localizations,
}

impl EventHandler {
    pub fn new() -> Self {
        EventHandler {
            localizations: Localizations::new(),
        }
    }

    pub async fn handle(
        &mut self,
        shard_id: u64,
        event: Event,
        http: Arc<HttpClient>,
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        match event {
            Event::MessageCreate(msg) => {
                if let Some(app_id) = msg.application_id {
                    if app_id.0.to_string() == env::var("APP_ID")? {
                        ()
                    }
                }
                
                if let Some((_command, language)) = parse_command(&msg.content) {
                    let game = Game::new();
                    let localization = self.localizations.get(language);
                    self.send_message(&game.localize(localization), msg.channel_id, http).await?;
                }
            }
            Event::ShardConnected(_) => {
                println!("Connected on shard {}", shard_id);
            }
            // Other events here...
            _ => {}
        }

        Ok(())
    }

    async fn send_message(&self, msg: &str, channel_id: ChannelId, http: Arc<HttpClient>)
        -> Result<(), Box<dyn Error + Send + Sync>>
    {
        http.create_message(channel_id)
            .content(msg)?
            .exec()
            .await?;
        Ok(())
    }
}
