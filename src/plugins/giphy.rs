use std::time::Duration;

use rand::seq::SliceRandom;
use serde::Deserialize;

use crate::prelude::*;

const SEARCH_URL: &str = "https://api.giphy.com/v1/gifs/search";

pub struct GiphyPlugin {
    api_key: String,
    client: reqwest::Client,
}

#[derive(Deserialize)]
struct SearchResponse {
    data: Vec<Gif>,
}

#[derive(Deserialize)]
struct Gif {
    id: String,
}

impl GiphyPlugin {
    async fn search(&self, query: &str) -> reqwest::Result<Vec<Gif>> {
        let resp: SearchResponse = self
            .client
            .get(SEARCH_URL)
            .query(&[
                ("api_key", self.api_key.as_str()),
                ("q", query),
                ("limit", "25"),
                ("rating", "pg-13"),
            ])
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;

        Ok(resp.data)
    }

    async fn handle_giphy(&self, ctx: &Arc<Context>, query: &str) -> Result<()> {
        let gifs = self.search(query).await.map_err(|err| err.without_url())?;
        let gif = gifs.choose(&mut rand::thread_rng());

        match gif {
            Some(gif) => {
                ctx.mention_reply(&format!("https://i.giphy.com/{}.gif", gif.id))
                    .await?;
            }
            None => {
                ctx.mention_reply(&format!("no GIFs found for \"{}\"", query))
                    .await?;
            }
        }

        Ok(())
    }
}

#[async_trait]
impl Plugin for GiphyPlugin {
    fn new_from_env() -> Result<Self> {
        let api_key = dotenvy::var("GIPHY_API_KEY").map_err(|_| {
            anyhow::format_err!("Missing $GIPHY_API_KEY. Required by the \"giphy\" plugin.")
        })?;
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(5))
            .build()?;
        Ok(GiphyPlugin { api_key, client })
    }

    fn command_metadata(&self) -> Vec<CommandMetadata> {
        vec![CommandMetadata {
            name: "giphy".to_string(),
            short_help: "usage: giphy <query>. gives a random matching GIF from Giphy.".to_string(),
            full_help: "searches Giphy and gives a random GIF matching the query.".to_string(),
        }]
    }

    async fn run(self, bot: Arc<Client>) -> Result<()> {
        let mut stream = bot.subscribe();

        while let Ok(ctx) = stream.recv().await {
            let res = match ctx.as_event() {
                Ok(Event::Command("giphy", Some(query))) => self.handle_giphy(&ctx, query).await,
                Ok(Event::Command("giphy", None)) => {
                    ctx.mention_reply("usage: giphy <query>").await
                }
                _ => Ok(()),
            };

            crate::check_err(&ctx, res).await;
        }

        Err(format_err!("giphy plugin lagged"))
    }
}
