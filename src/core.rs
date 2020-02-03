use async_trait::async_trait;

use crate::{Command, Context, Plugin, Result};

pub struct Core {}

impl Core {
    pub fn new() -> Core {
        Core {}
    }
}

#[async_trait]
impl Plugin for Core {
    async fn handle_message(&self, ctx: &Context) -> Result<()> {
        match ctx.msg.as_command() {
            Command::Raw("PING", params) => {
                ctx.send("PONG", params).await?;
            }
            Command::Raw("001", _) => {
                ctx.send("JOIN", vec!["#encoded-test"]).await?;
                ctx.send("JOIN", vec!["#rust"]).await?;
            }
            _ => {}
        }

        Ok(())
    }
}
