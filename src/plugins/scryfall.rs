use regex::Regex;
use scryfall::Card;

use crate::prelude::*;

pub struct ScryfallPlugin {
    re: Regex,
}

impl ScryfallPlugin {
    pub fn new() -> Self {
        ScryfallPlugin {
            re: Regex::new(r#"\[\[(.+?)\]\]"#).unwrap(),
        }
    }
}

impl ScryfallPlugin {
    async fn handle_scryfall(&self, ctx: &Arc<Context>, arg: &str) -> Result<()> {
        let card = Card::named_fuzzy(arg).await?;

        ctx.mention_reply(card.scryfall_uri.as_str()).await?;

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
            println!("{:?}", capture);
            match self.handle_scryfall(ctx, &capture[1]).await {
                Ok(_) => {}
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
            short_help: "usage: scryfall [card name]. gives a link to a card on Scryfall."
                .to_string(),
            full_help: "gives a link to a fiven card on Scryfall if it exists".to_string(),
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
