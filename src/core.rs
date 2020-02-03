use async_trait::async_trait;

use crate::{Context, Plugin, Result};

pub struct Core {}

impl Core {
    pub fn new() -> Core {
        Core {}
    }
}

#[async_trait]
impl Plugin for Core {
    async fn handle_message(&self, ctx: &Context) -> Result<()> {
        match &ctx.msg.command[..] {
            "PING" => {
                ctx.send_msg(&irc::Message::new(
                    "PONG".to_string(),
                    ctx.msg.params.clone(),
                ))
                .await?;
            }
            "001" => {
                ctx.send("JOIN", vec!["#encoded-test"]).await?;
                ctx.send("JOIN", vec!["#rust"]).await?;
            }
            _ => {},
        }

        Ok(())
    }
}
