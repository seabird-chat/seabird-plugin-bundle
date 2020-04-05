use async_minecraft_ping::Server;

use crate::prelude::*;

pub struct MinecraftPlugin;

impl MinecraftPlugin {
    pub fn new() -> Self {
        MinecraftPlugin {}
    }
}


#[async_trait]
impl Plugin for MinecraftPlugin {
    async fn handle_message(&self, ctx: &Arc<Context>) -> Result<()> {
        match ctx.as_event() {
            Event::Command("mc_players", Some(arg)) => {
                let parts: Vec<&str> = arg.splitn(2, ':').into_iter().collect();
                let address = parts.get(0).map(|s| s.to_string()).ok_or_else(|| format_err!("Missing server argument"))?;
                let port = parts.get(1);

                let mut server = Server::build(address.to_string());
                if let Some(port) = port {
                    server = server.with_port(port.parse()?);
                }

                let status = server.status().await?;

                ctx.mention_reply(&format!("{} of {} player(s) online on {}", status.players.online, status.players.max, address)).await?;
            }
            _ => {}
        }

        Ok(())
    }
}
