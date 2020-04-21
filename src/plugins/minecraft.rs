use std::time::Duration;

use anyhow::Context as AnyhowContext;
use async_minecraft_ping::ConnectionConfig;
use futures::future::{select, Either};
use tokio::time::interval;

use crate::prelude::*;

const DEFAULT_PORT: &str = "25565";

enum TopicUpdateConfig {
    NoUpdate,
    Update {
        server_hostname: String,
        server_port: u16,
        channel: String,
    },
}

struct HostPort {
    host: String,
    port: u16,
}

pub struct MinecraftPlugin {
    update_config: TopicUpdateConfig,
    last_topic: Option<String>,
}

fn split_host_port(hostport: &str, default_port: &str) -> Result<HostPort> {
    let parts: Vec<&str> = hostport.splitn(2, ':').collect();
    let host = parts
        .get(0)
        .map(|s| (*s).to_string())
        .ok_or_else(|| format_err!("missing hostport string (this should be impossible)"))?;
    let port = parts
        .get(1)
        .unwrap_or_else(|| &default_port)
        .parse()
        .with_context(|| "hostport string has invalid port specifier")?;

    Ok(HostPort { host, port })
}

impl MinecraftPlugin {
    async fn handle_mc_players(&self, ctx: &Context, arg: &str) -> Result<()> {
        let host_port = split_host_port(arg, DEFAULT_PORT)?;

        let config = ConnectionConfig::build(host_port.host).with_port(host_port.port);
        let mut connection = config.connect().await?;

        let status = connection.status().await?;

        ctx.mention_reply(&format!(
            "{} of {} player(s) online",
            status.players.online, status.players.max
        ))
        .await?;

        Ok(())
    }

    async fn update_topic(&self, bot: &Arc<Client>) -> Result<()> {
        if let TopicUpdateConfig::Update {
            ref server_hostname,
            server_port,
            ref channel,
        } = self.update_config
        {
            let config =
                ConnectionConfig::build(server_hostname.to_string()).with_port(server_port);
            let mut connection = config.connect().await?;

            let status = connection.status().await?;

            let topic = format!(
                "{}:{} - {} of {} player(s) online. \"{}\"",
                server_hostname,
                server_port,
                status.players.online,
                status.players.max,
                status.description.text,
            );

            match self.last_topic.as_ref() {
                Some(last_topic) if &topic != last_topic => {
                    bot.send("TOPIC", vec![channel, &topic]).await?
                }
                None => bot.send("TOPIC", vec![channel, &topic]).await?,
                Some(_) => {}
            }
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
            let server_hostport = dotenv::var("MINECRAFT_TOPIC_UPDATE_SERVER_HOSTPORT").with_context(|| {
                "Missing $MINECRAFT_TOPIC_UPDATE_SERVER_HOSTPORT. Required by the \"minecraft\" plugin because $MINECRAFT_TOPIC_UPDATE_ENABLED was set to true.".to_string()
            })?;

            let hostport = split_host_port(&server_hostport, DEFAULT_PORT).with_context(|| {
                "$MINECRAFT_TOPIC_UPDATE_SERVER_HOSTPORT is invalid. Required by the \"minecraft\" plugin because $MINECRAFT_TOPIC_UPDATE_ENABLED was set to true.".to_string()
            })?;

            TopicUpdateConfig::Update {
                server_hostname: hostport.host,
                server_port: hostport.port,
                channel: dotenv::var("MINECRAFT_TOPIC_UPDATE_CHANNEL").with_context(|| {
                    "Missing $MINECRAFT_TOPIC_UPDATE_CHANNEL. Required by the \"minecraft\" plugin because MINECRAFT_TOPIC_UPDATE_ENABLED was set to true.".to_string()
                })?,
            }
        } else {
            TopicUpdateConfig::NoUpdate
        };

        Ok(MinecraftPlugin {
            update_config,
            last_topic: None,
        })
    }

    async fn run(mut self, bot: Arc<Client>, mut stream: Receiver<Arc<Context>>) -> Result<()> {
        let mut timer = interval(Duration::from_secs(10));

        loop {
            let next = select(stream.next(), timer.next()).await;

            match next {
                Either::Left((Some(ctx), _)) => {
                    let res = match ctx.as_event() {
                        Event::Command("mc_players", Some(arg)) => {
                            self.handle_mc_players(&ctx, arg).await
                        }
                        Event::RplTopic {
                            nick: _,
                            channel,
                            topic,
                        } => {
                            if let TopicUpdateConfig::Update {
                                server_hostname: _,
                                server_port: _,
                                channel: ref config_channel,
                            } = self.update_config
                            {
                                if channel == config_channel {
                                    self.last_topic = Some(topic.to_string());
                                }
                            }
                            Ok(())
                        }
                        _ => Ok(()),
                    };

                    crate::check_err(&ctx, res).await;
                }
                Either::Left((None, _)) => {}
                Either::Right(_) => {
                    self.update_topic(&bot).await?;
                }
            }
        }

        //Err(format_err!("minecraft plugin exited early"))
    }
}
