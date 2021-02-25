// This bot is based off of bevbot https://github.com/kiedtl/bevbot which is
// released under the MIT license.
//
// Certain portions are copyright (c) 2020 Kiëd Llaentenn, lickthecheese

use rand::seq::SliceRandom;

use crate::prelude::*;

mod coffee;
mod tea;

const ACTIONS: &[&str] = &["hands", "gives", "passes", "serves"];

pub struct BaristaPlugin;

impl BaristaPlugin {
    async fn handle_coffee(&self, ctx: &Arc<Context>) -> Result<()> {
        let action = ACTIONS.choose(&mut rand::thread_rng()).unwrap();
        ctx.action_reply(&format!(
            "{} {} a {}!",
            action,
            ctx.sender().unwrap_or("someone"),
            coffee::prepare()
        ))
        .await?;
        Ok(())
    }

    async fn handle_tea(&self, ctx: &Arc<Context>) -> Result<()> {
        let action = ACTIONS.choose(&mut rand::thread_rng()).unwrap();
        ctx.action_reply(&format!(
            "{} {} a {}!",
            action,
            ctx.sender().unwrap_or("someone"),
            tea::prepare()
        ))
        .await?;
        Ok(())
    }
}

#[async_trait]
impl Plugin for BaristaPlugin {
    fn new_from_env() -> Result<Self> {
        Ok(BaristaPlugin {})
    }

    fn command_metadata(&self) -> Vec<CommandMetadata> {
        vec![
            CommandMetadata {
                name: "coffee".to_string(),
                short_help: "usage: coffee. Get some coffee from the bot.".to_string(),
                full_help: "a barista to give you coffee.".to_string(),
            },
            CommandMetadata {
                name: "tea".to_string(),
                short_help: "usage: tea. Get some tea from the bot.".to_string(),
                full_help: "a barista to give you tea.".to_string(),
            },
        ]
    }

    async fn run(self, bot: Arc<Client>) -> Result<()> {
        let mut stream = bot.subscribe();

        while let Ok(ctx) = stream.recv().await {
            let res = match ctx.as_event() {
                Ok(Event::Command("coffee", _)) => self.handle_coffee(&ctx).await,
                Ok(Event::Command("tea", _)) => self.handle_tea(&ctx).await,
                _ => Ok(()),
            };

            crate::check_err(&ctx, res).await;
        }

        Err(format_err!("barista plugin lagged"))
    }
}
