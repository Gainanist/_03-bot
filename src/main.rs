// mod components;
// mod systems;
// mod bygone_03;
// mod localization;
// mod command_parser;
// mod language;
// mod player_command;
// mod dice;
// mod events;

use std::{env, error::Error, slice::SliceIndex, sync::Arc};
// use command_parser::parse_command;
use futures::stream::StreamExt;
// use systems::Game;

// use localization::{Localizations, RenderText};
use twilight_cache_inmemory::{InMemoryCache, ResourceType};
use twilight_embed_builder::{EmbedBuilder, EmbedFieldBuilder, ImageSource};
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
    //  localizations: Localizations,
}

impl EventHandler {
    pub fn new() -> Self {
        EventHandler {
            // localizations: Localizations::new(),
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
                if msg.author.bot {
                    ()
                } else {
                
                // if let Some((_command, language)) = parse_command(&msg.content) {
                    // let game = Game::new();
                    // let localization = self.localizations.get(language);
                    //self.send_message(&game.render_text(localization), msg.channel_id, http).await?;
                    self.send_message(
                        "a\ta\naa\ta",
                        msg.channel_id,
                        http,
                    ).await?;
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

    async fn send_message(&self, _msg: &str, channel_id: ChannelId, http: Arc<HttpClient>)
        -> Result<(), Box<dyn Error + Send + Sync>> 
    {
        let embeds = [
            EmbedBuilder::new()
                .description("**A wild _03 appeared!**")
                .image(ImageSource::url("http://www.uof7.com/wp-content/uploads/2016/09/15-Bygone-UPD.gif")?)
                .build()?,
            EmbedBuilder::new()
                .field(EmbedFieldBuilder::new("Status", "ATK 2, ACC 100%, Core: armored").build())
                .field(EmbedFieldBuilder::new(":regional_indicator_c:ore", "[▮▮] - 20%").inline())
                .field(EmbedFieldBuilder::new(":regional_indicator_s:ensor", "[▮▮] - 50%").inline())
                .field(EmbedFieldBuilder::new(":regional_indicator_l:eft wing", "[▮▮] - 70%").inline())
                .field(EmbedFieldBuilder::new(":regional_indicator_r:ight wing", "[▮▮] - 70%").inline())
                .field(EmbedFieldBuilder::new(":regional_indicator_g:un", "[▮▮] - 50%").inline())
                .build()?,
            EmbedBuilder::new()
                .field(EmbedFieldBuilder::new("Battle log", "> • _03 gently punches Rokari in the chest with a rubber bullet\n> • Rokari performs a drunken style attack\n> • Ultra_Scream has joined the fray").build())
                .build()?,
            EmbedBuilder::new()
                .field(EmbedFieldBuilder::new("Rokari", "[▮▮▮▮▯▯]").inline())
                .field(EmbedFieldBuilder::new("Ultra_Scream", "[▮▮▮▮▮]").inline())
                .build()?,
            // EmbedBuilder::new()
            //     .description("Here's a cool image of Twilight Sparkle")
            //     .image(ImageSource::url("http://www.uof7.com/wp-content/uploads/2016/09/15-Bygone-UPD.gif")?)
            //     .build()?,
        ];
        http.create_message(channel_id)
            // .content(msg)?
            .embeds(&embeds)?
            .exec()
            .await?;
        Ok(())
    }
}
