use rand::seq::SliceRandom;

use crate::prelude::*;

pub struct ChancePlugin;

const CHOICES: &[&str] = &[
    // Yes
    "It is certain.",
    "It is decidedly so.",
    "Without a doubt.",
    "Yes - definitely.",
    "You may rely on it.",
    "As I see it, yes.",
    "Most likely.",
    "Outlook good.",
    "Yes.",
    "Signs point to yes.",
    // Maybe
    "Reply hazy, try again.",
    "Ask again later.",
    "Better not tell you now.",
    "Cannot predict now.",
    "Concentrate and ask again.",
    // No
    "Don't count on it.",
    "My reply is no.",
    "My sources say no.",
    "Outlook not so good.",
    "Very doubtful.",
];

impl ChancePlugin {
    async fn handle_8ball(&self, ctx: &Arc<Context>) -> Result<()> {
        let msg = CHOICES
            .choose(&mut rand::thread_rng())
            .ok_or_else(|| format_err!("failed to get valid choice"))?;

        ctx.mention_reply(msg).await?;

        Ok(())
    }
}

#[async_trait]
impl Plugin for ChancePlugin {
    fn new_from_env() -> Result<Self> {
        Ok(ChancePlugin {})
    }

    fn command_metadata(&self) -> Vec<CommandMetadata> {
        vec![CommandMetadata {
            name: "8ball".to_string(),
            short_help: "usage: 8ball [question]. Answers yes/no questions.".to_string(),
            full_help: "an oracle to answer yes/no questions.".to_string(),
        }]
    }

    async fn run(self, bot: Arc<Client>) -> Result<()> {
        let mut stream = bot.subscribe();

        while let Ok(ctx) = stream.recv().await {
            let res = match ctx.as_event() {
                Ok(Event::Command("8ball", _)) => self.handle_8ball(&ctx).await,
                _ => Ok(()),
            };

            crate::check_err(&ctx, res).await;
        }

        Err(format_err!("mention plugin lagged"))
    }
}
