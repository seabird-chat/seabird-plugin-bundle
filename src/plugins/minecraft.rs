use std::time::Duration;

use async_minecraft_ping::ConnectionConfig;
use log::info;
use tokio::time::timeout;

use crate::prelude::*;

enum TopicUpdateConfig {
    NoUpdate,
    Update {
        server_hostname: String,
        server_port: u16,
        channel: String,
    },
}

pub struct MinecraftPlugin {
    update_config: TopicUpdateConfig,
}

impl MinecraftPlugin {
    async fn handle_mc_players(&self, ctx: &Context, arg: &str) -> Result<()> {
        let parts: Vec<&str> = arg.splitn(2, ':').collect();
        let address = parts
            .get(0)
            .map(|s| (*s).to_string())
            .ok_or_else(|| format_err!("missing server argument"))?;
        let port = parts.get(1);

        let mut config = ConnectionConfig::build(address.to_string());
        if let Some(port) = port {
            config = config.with_port(port.parse()?);
        }

        let mut connection = config.connect().await?;

        let status = connection.status().await?;

        ctx.mention_reply(&format!(
            "{} of {} player(s) online on {}",
            status.players.online, status.players.max, address
        ))
        .await?;

        Ok(())
    }

    async fn update_topic(&self) -> Result<()> {
        if let TopicUpdateConfig::Update { ref server_hostname, server_port, ref channel } = self.update_config {
            let config = ConnectionConfig::build(server_hostname.to_string()).with_port(server_port);
            let mut connection = config.connect().await?;

            let status = connection.status().await?;

            let topic = format!("{}:{} - {} of {} player(s) online. \"{}\"",
                server_hostname, server_port, status.players.online, status.players.max,
                status.description.text,
            );

            // TODO(jsvana): set topic here: TOPIC #chan TOPIC
            info!("Would set topic on {} to \"{}\"", channel, topic);
        }

        Ok(())
    }
}

#[async_trait]
impl Plugin for MinecraftPlugin {
    fn new_from_env() -> Result<Self> {
        let updates_enabled = dotenv::var("MINECRAFT_TOPIC_UPDATE_ENABLED").unwrap_or_else(|_| "false".to_string()).parse().map_err(|_| {
            anyhow::format_err!("$MINECRAFT_TOPIC_UPDATE_ENABLED is not a valid boolean. Error from the \"minecraft\" plugin.")
        })?;
        let update_config = if updates_enabled {
            TopicUpdateConfig::Update {
                server_hostname: dotenv::var("MINECRAFT_TOPIC_UPDATE_SERVER_HOSTNAME").map_err(|_| {
                    anyhow::format_err!(
                        "Missing $MINECRAFT_TOPIC_UPDATE_SERVER_HOSTNAME. Required by the \"minecraft\" plugin because MINECRAFT_TOPIC_UPDATE_ENABLED was set to true."
                    )
                })?,
                server_port: dotenv::var("MINECRAFT_TOPIC_UPDATE_SERVER_PORT").unwrap_or_else(|_| "25565".to_string()).parse().map_err(|_| {
                    anyhow::format_err!(
                        "$MINECRAFT_TOPIC_UPDATE_PORT is an invalid u16. Required by the \"minecraft\" plugin because MINECRAFT_TOPIC_UPDATE_ENABLED was set to true."
                    )
                })?,
                channel: dotenv::var("MINECRAFT_TOPIC_UPDATE_CHANNEL").map_err(|_| {
                    anyhow::format_err!(
                        "Missing $MINECRAFT_TOPIC_UPDATE_CHANNEL. Required by the \"minecraft\" plugin because MINECRAFT_TOPIC_UPDATE_ENABLED was set to true."
                    )
                })?,
            }
        } else {
            TopicUpdateConfig::NoUpdate
        };

        Ok(MinecraftPlugin {
            update_config,
        })
    }

<<<<<<< HEAD
    async fn run(self, _bot: Arc<Client>, mut stream: Receiver<Arc<Context>>) -> Result<()> {
        // TODO(jsvana): make this update loop update the topic at regular intervals
        // instead of what it's doing now.
        loop {
            match timeout(Duration::from_secs(60), stream.next()).await {
                Ok(res) => match res {
                    Some(ctx) => {
                        let res = match ctx.as_event() {
                            Event::Command("mc_players", Some(arg)) => self.handle_mc_players(&ctx, arg).await,
                            _ => Ok(()),
                        };

                        crate::check_err(&ctx, res).await;
                    },
                    None => break,
                }
                Err(_) => self.update_topic().await?,
            }
        }

        Err(format_err!("minecraft plugin exited early"))
    }
}
