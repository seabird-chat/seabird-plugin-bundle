use std::time::Duration;

use serde::Deserialize;

use crate::prelude::*;

const API_BASE: &str = "https://v2.jokeapi.dev/joke";
const CATEGORIES: &[&str] = &["any", "misc", "programming", "pun", "spooky", "christmas"];

pub struct JokePlugin {
    client: reqwest::Client,
}

#[derive(Deserialize)]
struct JokeResponse {
    error: bool,
    #[serde(rename = "type")]
    joke_type: String,
    joke: Option<String>,
    setup: Option<String>,
    delivery: Option<String>,
    message: Option<String>,
}

impl JokePlugin {
    async fn fetch_joke(&self, category: &str) -> Result<JokeResponse> {
        let url = format!("{}/{}?safe-mode", API_BASE, category);

        let resp: JokeResponse = self
            .client
            .get(&url)
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;

        if resp.error {
            return Err(format_err!("{}", resp.message.unwrap_or_else(|| "Unknown API error".to_string())));
        }

        Ok(resp)
    }

    async fn handle_joke(&self, ctx: &Arc<Context>, arg: Option<&str>) -> Result<()> {
        let category = arg.unwrap_or("Any");

        let joke = self.fetch_joke(category).await?;

        match joke.joke_type.as_str() {
            "single" => {
                if let Some(text) = joke.joke {
                    ctx.reply(&text).await?;
                }
            }
            "twopart" => {
                if let (Some(setup), Some(delivery)) = (joke.setup, joke.delivery) {
                    ctx.reply(&setup).await?;
                    tokio::time::sleep(Duration::from_secs(3)).await;
                    ctx.reply(&delivery).await?;
                }
            }
            _ => {}
        }

        Ok(())
    }
}

#[async_trait]
impl Plugin for JokePlugin {
    fn new_from_env() -> Result<Self> {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(5))
            .build()?;
        Ok(JokePlugin { client })
    }

    fn command_metadata(&self) -> Vec<CommandMetadata> {
        vec![CommandMetadata {
            name: "joke".to_string(),
            short_help: format!("usage: joke [category]. Categories: {}", CATEGORIES.join(", ")),
            full_help: format!("Gets a random joke. Categories: {}", CATEGORIES.join(", ")),
        }]
    }

    async fn run(self, bot: Arc<Client>) -> Result<()> {
        let mut stream = bot.subscribe();

        while let Ok(ctx) = stream.recv().await {
            let res = match ctx.as_event() {
                Ok(Event::Command("joke", arg)) => self.handle_joke(&ctx, arg).await,
                _ => Ok(()),
            };

            crate::check_err(&ctx, res).await;
        }

        Err(format_err!("joke plugin lagged"))
    }
}
