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
        update_interval: Duration,
    },
}

struct HostPort {
    host: String,
    port: u16,
}

pub struct MinecraftPlugin {
    update_config: TopicUpdateConfig,
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

    async fn query_topic(&self, bot: &Arc<Client>) -> Result<()> {
        if let TopicUpdateConfig::Update {
            ref channel, ..
        } = self.update_config {
            bot.send("TOPIC", vec![channel]).await?;
        }

        Ok(())
    }

    async fn update_topic(&self, bot: &Arc<Client>, incoming_channel: &str, last_topic: &str) -> Result<()> {
        if let TopicUpdateConfig::Update {
            ref server_hostname,
            server_port,
            ref channel,
            ..
        } = self.update_config
        {
            // Only update the topic for our configured channel
            if channel != incoming_channel {
                return Ok(());
            }

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

            if topic != last_topic {
                bot.send("TOPIC", vec![channel, &topic]).await?;
            }
        }

        Ok(())
    }

    async fn handle_message(&self, bot: &Arc<Client>, context: Arc<Context>) {
        let res = match context.as_event() {
            Event::Command("mc_players", Some(arg)) => {
                self.handle_mc_players(&context, arg).await
            }
            Event::RplTopic {
                nick: _,
                channel,
                topic,
            } => {
                self.update_topic(&bot, channel, topic).await
            }
            _ => Ok(()),
        };

        crate::check_err(&context, res).await;
    }

    async fn run_update_loop(&self, bot: Arc<Client>, mut stream: Receiver<Arc<Context>>) -> Result<()> {
        if let TopicUpdateConfig::Update { update_interval, .. } = self.update_config {
            let mut timer = interval(update_interval);

            loop {
                let next = select(stream.next(), timer.next()).await;

                match next {
                    Either::Left((Some(context), _)) => {
                        self.handle_message(&bot, context).await;
                    }
                    Either::Right(_) => {
                        self.query_topic(&bot).await?;
                    }
                    _ => {}
                }
            }
        }

        Ok(())
    }

    async fn run_only_read_commands(&self, bot: Arc<Client>, mut stream: Receiver<Arc<Context>>) -> Result<()> {
        while let Some(context) = stream.next().await {
            self.handle_message(&bot, context).await;
        }

        Err(format_err!("karma plugin exited early"))
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

            let channel = dotenv::var("MINECRAFT_TOPIC_UPDATE_CHANNEL").with_context(|| {
                "Missing $MINECRAFT_TOPIC_UPDATE_CHANNEL. Required by the \"minecraft\" plugin because MINECRAFT_TOPIC_UPDATE_ENABLED was set to true.".to_string()
            })?;

            let update_interval_seconds = dotenv::var("MINECRAFT_TOPIC_UPDATE_INTERVAL_SECONDS").unwrap_or_else(|_| "60".to_string()).parse::<u64>().with_context(|| {
                "$MINECRAFT_TOPIC_UPDATE_INTERVAL_SECONDS has invalid duration. Required by the \"minecraft\" plugin because MINECRAFT_TOPIC_UPDATE_ENABLED was set to true.".to_string()
            })?;
            let update_interval = Duration::from_secs(update_interval_seconds);

            TopicUpdateConfig::Update {
                server_hostname: hostport.host,
                server_port: hostport.port,
                channel,
                update_interval,
            }
        } else {
            TopicUpdateConfig::NoUpdate
        };

        Ok(MinecraftPlugin {
            update_config,
        })
    }

    async fn run(self, bot: Arc<Client>, stream: Receiver<Arc<Context>>) -> Result<()> {
        // Only run the update loop if we actually want to update the topic
        match self.update_config {
            TopicUpdateConfig::Update { .. } => self.run_update_loop(bot, stream).await,
            TopicUpdateConfig::NoUpdate => {
                self.run_only_read_commands(bot, stream).await
            }
        }
    }
}
