use crate::prelude::*;

pub struct MentionPlugin {}

impl MentionPlugin {
    pub fn new() -> Self {
        MentionPlugin {}
    }
}

#[async_trait]
impl Plugin for MentionPlugin {
    async fn handle_message(&self, ctx: &Arc<Context>) -> Result<()> {
        if let Event::Mention(msg) = ctx.as_event() {
            match msg {
                "ping" => ctx.mention_reply("pong").await?,
                "scoobysnack" | "scooby snack" => ctx.reply("Scooby Dooby Doo!").await?,
                "botsnack" | "bot snack" => ctx.reply(":)").await?,
                "pizzahousesnack" => {
                    ctx.reply("HECK YEAHHHHHHHHHHHH OMG I LOVE U THE WORLD IS GREAT")
                        .await?
                }
                _ => (),
            };
        }

        Ok(())
    }
}
