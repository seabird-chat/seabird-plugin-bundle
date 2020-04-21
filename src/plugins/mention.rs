use crate::prelude::*;

pub struct MentionPlugin;

#[async_trait]
impl Plugin for MentionPlugin {
    fn new_from_env() -> Result<Self> {
        Ok(MentionPlugin {})
    }

    async fn run(self, bot: Arc<Client>, mut stream: Receiver<Arc<Context>>) -> Result<()> {
        // Drop the bot reference because we don't need it.
        drop(bot);

        while let Some(ctx) = stream.next().await {
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
        }

        Err(format_err!("mention plugin exited early"))
    }
}
