use std::error::Error;
use std::fmt::Write;

use lazy_static::lazy_static;
use regex::Regex;
use scryfall::{search::Search, Card};
use url::Url;

use crate::prelude::*;

lazy_static! {
    static ref SCRYFALL_RE: Regex = Regex::new(r#"\[\[(.+?)\]\]"#)
        .expect("invalid scryfall regex");
}

pub struct ScryfallPlugin;

const SCRYFALL_SEARCH_URL: &str = "https://scryfall.com/search";

impl ScryfallPlugin {
    pub fn new() -> Self {
        ScryfallPlugin
    }
}

fn reqwest_error_string(err: &reqwest::Error) -> String {
    let mut err: &dyn std::error::Error = &err;
    let mut s = format!("{}", err);
    while let Some(src) = err.source() {
        let _ = write!(s, "\n\nCaused by: {}", src);
        err = src;
    }
    s
}

fn scryfall_api_error_string(err: &scryfall::error::ScryfallError) -> String {
    format!("Error code: {}, Details: {}", err.code, err.details)
}

// Unfortunately, some of the errors Scryfall provides aren't very helpful, so
// we provide an alternative to scryfall's fmt::Display implementation.
fn scryfall_error_string(err: &scryfall::Error) -> String {
    match err {
        scryfall::Error::JsonError(inner_err) => format!("JSON error: {}", inner_err),
        scryfall::Error::UrlEncodedError(inner_err) => format!("URL encoding error: {}", inner_err),
        scryfall::Error::UrlParseError(inner_err) => format!("URL parse error: {}", inner_err),
        scryfall::Error::ReqwestError {
            error: inner_err,
            url: _,
        } => format!("Request error: {}", reqwest_error_string(inner_err)),
        scryfall::Error::ScryfallError(inner_err) => format!(
            "Scryfall API error: {}",
            scryfall_api_error_string(inner_err)
        ),
        scryfall::Error::HttpError(inner_err) => format!("HTTP error: {}", inner_err),
        scryfall::Error::IoError(inner_err) => format!("IO error: {}", inner_err),
        scryfall::Error::Other(inner_err) => format!("Other error: {}", inner_err),
    }
}

impl ScryfallPlugin {
    async fn handle_scryfall(&self, ctx: &Arc<Context>, arg: &str) -> Result<()> {
        let card_iter = Card::search(arg).await.map_err(|err| {
            println!("{}", scryfall_error_string(&err));
            err
        })?;

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
        let captures: Vec<_> = SCRYFALL_RE.captures_iter(msg).collect();

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

        Err(format_err!("scryfall plugin lagged"))
    }
}
