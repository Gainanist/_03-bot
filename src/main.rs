mod state;

use std::{borrow::BorrowMut, env, error::Error, sync::Arc};
use futures::stream::StreamExt;
use rand::{Rng, prelude::ThreadRng};
use state::Battle;
use twilight_cache_inmemory::{InMemoryCache, ResourceType};
use twilight_gateway::{cluster::{Cluster, ShardScheme}, Event};
use twilight_http::Client as HttpClient;
use twilight_model::{gateway::{Intents, payload::incoming::MessageCreate}, id::ChannelId};

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
    let mut rng = rand::thread_rng();

    // Process each event as they come in.
    while let Some((shard_id, event)) = events.next().await {
        // Update the cache with the event.
        cache.update(&event);

        event_handler.handle(
            shard_id,
            event,
            Arc::clone(&http),
            rng.gen_range(0..100),
            rng.gen_range(0..100),
        ).await?;
    }

    Ok(())
}

struct EventHandler {
    battle: Battle,
}

impl EventHandler {
    pub fn new() -> Self {
        EventHandler {
            battle: Battle::new()
        }
    }
    pub async fn handle(
        &mut self,
        shard_id: u64,
        event: Event,
        http: Arc<HttpClient>,
        hero_hit_roll: isize,
        bygone_hit_roll: isize,
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        match event {
            Event::MessageCreate(msg) => {
                if let Some(app_id) = msg.application_id {
                    if app_id.0.to_string() == env::var("APP_ID")? {
                        ()
                    }
                }
                
                let bygone_hit_roll = bygone_hit_roll + self.battle.bygone.accuracy_penalty();
                if msg.content.contains("_03") && self.battle.finished() {
                    self.battle = Battle::start();
                    self.render_battle(msg.channel_id, http).await?;
                } else if msg.content.contains("сенсор") && !self.battle.finished() {
                    self.battle.bygone.damage_sensor(
                        self.battle.hero.shoot(),
                        hero_hit_roll,
                    );
                    if self.battle.bygone.alive() {
                        self.battle.hero.damage(
                            self.battle.bygone.shoot(),
                            bygone_hit_roll,

                        );
                    }
                    self.render_battle(msg.channel_id, http).await?;
                } else if msg.content.contains("ядро") && !self.battle.finished() {
                    self.battle.bygone.damage_core(
                        self.battle.hero.shoot(),
                        hero_hit_roll,
                    );
                    if self.battle.bygone.alive() {
                        self.battle.hero.damage(
                            self.battle.bygone.shoot(),
                            bygone_hit_roll,

                        );
                    }
                    self.render_battle(msg.channel_id, http).await?;
                } else if msg.content.contains("левое крыло") && !self.battle.finished() {
                    self.battle.bygone.damage_left_wing(
                        self.battle.hero.shoot(),
                        hero_hit_roll,
                    );
                    if self.battle.bygone.alive() {
                        self.battle.hero.damage(
                            self.battle.bygone.shoot(),
                            bygone_hit_roll,

                        );
                    }
                    self.render_battle(msg.channel_id, http).await?;
                } else if msg.content.contains("правое крыло") && !self.battle.finished() {
                    self.battle.bygone.damage_right_wing(
                        self.battle.hero.shoot(),
                        hero_hit_roll,
                    );
                    if self.battle.bygone.alive() {
                        self.battle.hero.damage(
                            self.battle.bygone.shoot(),
                            bygone_hit_roll,

                        );
                    }
                    self.render_battle(msg.channel_id, http).await?;
                } else if msg.content.contains("орудие") && !self.battle.finished() {
                    self.battle.bygone.damage_gun(
                        self.battle.hero.shoot(),
                        hero_hit_roll,
                    );
                    if self.battle.bygone.alive() {
                        self.battle.hero.damage(
                            self.battle.bygone.shoot(),
                            bygone_hit_roll,

                        );
                    }
                    self.render_battle(msg.channel_id, http).await?;
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

    async fn render_battle(&self, channel_id: ChannelId, http: Arc<HttpClient>) -> Result<(), Box<dyn Error + Send + Sync>> {
        http.create_message(channel_id)
            .content(&format!("{}", self.battle))?
            .exec()
            .await?;
        Ok(())
    }

    // async fn tick<F: FnMut(isize, isize) -> () + ?Sized>(&mut self, damage_bygone: &mut F) {
    //     let (damage, hit_chance_roll) = self.battle.hero.shoot(&mut self.rng);
    //     damage_bygone(damage, hit_chance_roll);
    //     if self.battle.bygone.alive() {
    //         let (damage, hit_chance_roll) = self.battle.bygone.shoot(&mut self.rng);
    //         self.battle.hero.damage(
    //             damage, hit_chance_roll
    //         );
    //     }
    // }
}
