use regex::Regex;
use scryfall::{search::Search, Card};
use url::Url;

use crate::prelude::*;

pub struct ScryfallPlugin {
    re: Regex,
}

const SCRYFALL_SEARCH_URL: &str = "https://scryfall.com/search";

impl ScryfallPlugin {
    pub fn new() -> Self {
        ScryfallPlugin {
            re: Regex::new(r#"\[\[(.+?)\]\]"#).unwrap(),
        }
    }
}

impl ScryfallPlugin {
    async fn handle_scryfall(&self, ctx: &Arc<Context>, arg: &str) -> Result<()> {
        let card_iter = Card::search(arg).await?;

        let (n, _) = card_iter.size_hint();
        if n > 1 {
            let mut search_url = Url::parse(SCRYFALL_SEARCH_URL)?;
            arg.write_query(&mut search_url)?;

            ctx.mention_reply(&format!("Found {} results: {}", n, search_url))
                .await?;
        }

        let mut card_stream = card_iter.into_stream().take(3);

        while let Some(card) = card_stream.try_next().await? {
            ctx.mention_reply(&format!("{}: {}", card.name, card.scryfall_uri))
                .await?;
        }

        Ok(())
    }

    async fn handle_privmsg(&self, ctx: &Arc<Context>, msg: &str) -> Result<()> {
        let captures: Vec<_> = self.re.captures_iter(msg).collect();

        if captures.is_empty() {
            return Ok(());
        }

        let mut change_errors = Vec::new();

        // Loop through all captures, adding them to the output.
        for capture in captures {
            match Card::named(&capture[1]).await {
                Ok(card) => {
                    let card_uri = card.scryfall_uri;
                    let image_uri = card.image_uris.and_then(|uris| uris.png);
                    match image_uri {
                        Some(image_uri) => {
                            ctx.mention_reply(&format!("{} ({})", card_uri, image_uri.as_str()))
                                .await?;
                        }
                        None => {
                            ctx.mention_reply(&format!("{}", card_uri)).await?;
                        }
                    }
                }
                Err(e) => {
                    change_errors.push(format!("failed to look up \"{}\": {}", &capture[1], e));
                }
            };
        }

        if !change_errors.is_empty() {
            ctx.mention_reply(&change_errors.join(", ")).await?;
        }

        Ok(())
    }
}

#[async_trait]
impl Plugin for ScryfallPlugin {
    fn new_from_env() -> Result<Self> {
        Ok(ScryfallPlugin::new())
    }

    fn command_metadata(&self) -> Vec<CommandMetadata> {
        vec![CommandMetadata {
            name: "scryfall".to_string(),
            short_help: "usage: scryfall [card name]. gives a link to a magic card on Scryfall."
                .to_string(),
            full_help: "gives a link to a given magic card on Scryfall if it exists".to_string(),
        }]
    }

    async fn run(self, bot: Arc<Client>) -> Result<()> {
        let mut stream = bot.subscribe();

        while let Ok(ctx) = stream.recv().await {
            let res = match ctx.as_event() {
                Ok(Event::Command("scryfall", possible_arg)) => {
                    match possible_arg.or_else(|| ctx.sender()) {
                        Some(nick) => self.handle_scryfall(&ctx, nick).await,
                        None => Err(format_err!("no card name found")),
                    }
                }
                Ok(Event::Message(_, msg)) => self.handle_privmsg(&ctx, msg).await,
                _ => Ok(()),
            };

            crate::check_err(&ctx, res).await;
        }

        Err(format_err!("karma plugin lagged"))
    }
}
