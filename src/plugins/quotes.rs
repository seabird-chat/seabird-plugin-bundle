use crate::prelude::*;

#[derive(Debug, Hash, Eq, PartialEq, PartialOrd, Ord)]
struct CacheKey {
    channel: String,
    nick: String,
}

#[derive(Default)]
pub struct QuotesPlugin {
    message_cache: HashMap<CacheKey, Quote>,
}

impl QuotesPlugin {
    pub fn new() -> Self {
        QuotesPlugin {
            ..Default::default()
        }
    }
}

impl QuotesPlugin {
    async fn handle_grab(&mut self, ctx: &Context, arg: Option<&str>) -> Result<()> {
        let nick = match arg {
            None => {
                ctx.mention_reply(&format!("missing argument")).await?;
                return Ok(());
            }
            Some(nick) => nick,
        };

        let cache_key = CacheKey {
            channel: ctx.target_channel_id().unwrap_or("unknown").to_string(),
            nick: nick.to_string(),
        };

        let quote = match self.message_cache.remove(&cache_key) {
            None => {
                ctx.mention_reply(&format!(
                    "latest message from {} in channel {} not found",
                    cache_key.nick, cache_key.channel
                ))
                .await?;
                return Ok(());
            }
            Some(quote) => quote,
        };

        let conn = ctx.get_db();
        sqlx::query!(
            "INSERT INTO quotes (nick, quote) VALUES ($1, $2)",
            quote.nick,
            quote.quote,
        )
        .execute(&conn)
        .await?;

        ctx.mention_reply(&format!("saved {}'s message: {}", quote.nick, quote.quote))
            .await?;

        Ok(())
    }

    async fn handle_quote(&self, ctx: &Context, arg: Option<&str>) -> Result<()> {
        let conn = ctx.get_db();

        let nick = match arg {
            None => {
                ctx.mention_reply(&format!("missing argument")).await?;
                return Ok(());
            }
            Some(nick) => nick,
        };

        let quote = match sqlx::query_as!(
            Quote,
            "SELECT nick, quote FROM quotes WHERE nick=$1 ORDER BY random() LIMIT 1;",
            nick
        )
        .fetch_optional(&conn)
        .await?
        {
            None => {
                ctx.mention_reply(&format!("no quotes from {} found", nick))
                    .await?;
                return Ok(());
            }
            Some(quote) => quote,
        };

        ctx.mention_reply(&format!("quote from {}: {}", quote.nick, quote.quote))
            .await?;

        Ok(())
    }

    async fn handle_message(&mut self, ctx: &Context, _sender: &str, msg: &str) -> Result<()> {
        let nick = ctx.sender().unwrap_or("unknown").to_string();

        let cache_key = CacheKey {
            channel: ctx.target_channel_id().unwrap_or("unknown").to_string(),
            nick: nick.clone(),
        };

        let quote = Quote {
            nick,
            quote: msg.to_string(),
        };

        log::debug!(
            "Latest quotes cache key: {:?}, quote: {:?}",
            cache_key,
            quote
        );

        self.message_cache.insert(cache_key, quote);

        Ok(())
    }
}

#[derive(sqlx::FromRow, Hash, Debug)]
pub struct Quote {
    pub nick: String,
    pub quote: String,
}

#[async_trait]
impl Plugin for QuotesPlugin {
    fn new_from_env() -> Result<Self> {
        Ok(QuotesPlugin::new())
    }

    fn command_metadata(&self) -> Vec<CommandMetadata> {
        vec![
            CommandMetadata {
                name: "grab".to_string(),
                short_help: "usage: grab [nick]. Saves the latest message by nick in this channel as a quote.".to_string(),
                ..Default::default()
            },
            CommandMetadata {
                name: "quote".to_string(),
                short_help: "usage: quote [nick]. Returns a random quote by a user in a channel.".to_string(),
                ..Default::default()
            },
        ]
    }

    async fn run(mut self, bot: Arc<Client>) -> Result<()> {
        let mut stream = bot.subscribe();

        while let Ok(ctx) = stream.recv().await {
            let res = match ctx.as_event() {
                Ok(Event::Command("grab", arg)) => self.handle_grab(&ctx, arg).await,
                Ok(Event::Command("quote", arg)) => self.handle_quote(&ctx, arg).await,
                Ok(Event::Message(sender, message)) => {
                    self.handle_message(&ctx, sender, message).await
                }
                _ => Ok(()),
            };

            crate::check_err(&ctx, res).await;
        }

        Err(format_err!("quotes plugin lagged"))
    }
}
