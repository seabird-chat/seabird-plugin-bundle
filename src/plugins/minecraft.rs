use async_minecraft_ping::ConnectionConfig;

use crate::prelude::*;

pub struct MinecraftPlugin;

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
}

#[async_trait]
impl Plugin for MinecraftPlugin {
    fn new_from_env() -> Result<Self> {
        Ok(MinecraftPlugin {})
    }

    async fn run(self, _bot: Arc<Client>, mut stream: Receiver<Arc<Context>>) -> Result<()> {
        while let Some(ctx) = stream.next().await {
            let res = match ctx.as_event() {
                Event::Command("mc_players", Some(arg)) => self.handle_mc_players(&ctx, arg).await,
                _ => Ok(()),
            };

            crate::check_err(&ctx, res).await;
        }

        Err(format_err!("minecraft plugin exited early"))
    }
}
