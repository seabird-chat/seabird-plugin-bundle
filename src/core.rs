use async_trait::async_trait;

use crate::{Context, Plugin, Result};

pub struct Ping {}

impl Ping {
    pub fn new() -> Ping {
        Ping {}
    }
}

#[async_trait]
impl Plugin for Ping {
    async fn handle_message(&self, ctx: &Context) -> Result<()> {
        match &ctx.msg.command[..] {
            "PING" => {
                ctx.send_msg(&irc::Message::new(
                    "PONG".to_string(),
                    ctx.msg.params.clone(),
                ))
                .await?
            }
            _ => {}
        }

        Ok(())
    }
}

pub struct Welcome {}

impl Welcome {
    pub fn new() -> Self {
        Welcome {}
    }
}

#[async_trait]
impl Plugin for Welcome {
    async fn handle_message(&self, ctx: &Context) -> Result<()> {
        match &ctx.msg.command[..] {
            "001" => {
                ctx.send("JOIN", vec!["#encoded-test"]).await?;
                ctx.send("JOIN", vec!["#rust"]).await?;
            }
            _ => {}
        }

        Ok(())
    }
}
