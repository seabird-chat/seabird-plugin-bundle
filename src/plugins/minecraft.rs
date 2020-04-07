use async_minecraft_ping::ConnectionConfig;

use crate::prelude::*;

pub struct MinecraftPlugin;

#[async_trait]
impl Plugin for MinecraftPlugin {
    fn new_from_env() -> Result<Self> {
        Ok(MinecraftPlugin {})
    }

    async fn handle_message(&self, ctx: &Arc<Context>) -> Result<()> {
        match ctx.as_event() {
            Event::Command("mc_players", Some(arg)) => {
                let parts: Vec<&str> = arg.splitn(2, ':').into_iter().collect();
                let address = parts
                    .get(0)
                    .map(|s| s.to_string())
                    .ok_or_else(|| format_err!("Missing server argument"))?;
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
            }
            _ => {}
        }

        Ok(())
    }
}
