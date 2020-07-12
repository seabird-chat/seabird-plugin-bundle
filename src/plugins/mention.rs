use crate::prelude::*;

pub struct MentionPlugin;

#[async_trait]
impl Plugin for MentionPlugin {
    fn new_from_env() -> Result<Self> {
        Ok(MentionPlugin {})
    }

    async fn run(self, bot: Arc<Client>) -> Result<()> {
        let mut stream = bot.subscribe();

        while let Ok(ctx) = stream.recv().await {
            if let Ok(Event::Mention(msg)) = ctx.as_event() {
                let res = match msg {
                    "ping" => ctx.mention_reply("pong").await,
                    "scoobysnack" | "scooby snack" => ctx.reply("Scooby Dooby Doo!").await,
                    "botsnack" | "bot snack" => ctx.reply(":)").await,
                    "pizzahousesnack" => {
                        ctx.reply("HECK YEAHHHHHHHHHHHH OMG I LOVE U THE WORLD IS GREAT")
                            .await
                    }
                    _ => Ok(()),
                };

                crate::check_err(&ctx, res).await;
            }
        }

        Err(format_err!("mention plugin lagged"))
    }
}
