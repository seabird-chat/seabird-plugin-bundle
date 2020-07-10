use std::collections::BTreeMap;
use std::convert::TryInto;
use std::fmt::Write;

use regex::Regex;

use crate::prelude::*;

#[derive(Debug)]
pub struct Karma {
    pub name: String,
    pub score: i32,
}

impl Karma {
    fn sanitize_name(name: &str) -> String {
        name.to_lowercase()
    }

    async fn get_by_name(conn: &tokio_postgres::Client, name: &str) -> Result<Self> {
        let res = conn
            .query_opt("SELECT name, score FROM karma WHERE name=$1;", &[&name])
            .await?;

        Ok(if let Some(row) = res {
            Karma {
                name: row.get(0),
                score: row.get(1),
            }
        } else {
            Karma {
                name: name.to_string(),
                score: 0,
            }
        })
    }

    async fn create_or_update(
        conn: &tokio_postgres::Client,
        name: &str,
        score: i32,
    ) -> Result<Self> {
        conn.execute(
            "INSERT INTO karma (name, score) VALUES ($1, $2)
ON CONFLICT (name) DO UPDATE SET score=EXCLUDED.score+karma.score;",
            &[&name, &score],
        )
        .await?;

        Karma::get_by_name(conn, &name).await
    }
}

pub struct KarmaPlugin {
    re: Regex,
}

impl KarmaPlugin {
    pub fn new() -> Self {
        KarmaPlugin {
            re: Regex::new(r#"([\w]{2,}|".+?")(\+\++|--+)(?:\s|$)"#).unwrap(),
        }
    }
}

impl KarmaPlugin {
    async fn handle_karma(&self, ctx: &Arc<Context>, arg: &str) -> Result<()> {
        let name = Karma::sanitize_name(arg);
        let karma = Karma::get_by_name(&ctx.get_db(), &name).await?;

        ctx.mention_reply(&format!("{}'s karma is {}", arg, karma.score))
            .await?;

        Ok(())
    }

    async fn handle_privmsg(&self, ctx: &Arc<Context>, msg: &str) -> Result<()> {
        let captures: Vec<_> = self.re.captures_iter(msg).collect();

        if !captures.is_empty() {
            let mut changes = BTreeMap::new();

            // Loop through all captures, adding them to the output.
            for capture in captures {
                let mut name = &capture[1];

                // TODO: switch to strip_prefix and strip_suffix when they're available.
                if name.starts_with('"') && name.ends_with('"') {
                    name = &name[1..name.len() - 1];
                }

                // Len returns a usize which won't fit in an i64, so we need to try and
                // convert it.
                let mut change: i32 = (&capture[2].len() - 1).try_into()?;
                if capture[2].starts_with('-') {
                    change *= -1;
                }

                let cleaned_name = Karma::sanitize_name(name);
                *changes.entry(cleaned_name).or_insert(0) += change;
            }

            let mut first = true;
            let mut out = String::new();

            let db = ctx.get_db();

            for (name, raw_change) in changes.into_iter() {
                let change = utils::clamp(raw_change, -5, 5);

                let karma = Karma::create_or_update(&db, &name, change).await?;
                if first {
                    first = false;
                } else {
                    out.push_str(", ");
                }

                write!(out, "{}'s karma is now {}", name, karma.score)?;

                if raw_change != change {
                    out.push_str(" Buzzkill Mode (tm) enforced a limit of 5");
                }
            }

            // Send the bulk message
            ctx.reply(&out).await?;
        }

        Ok(())
    }
}

#[async_trait]
impl Plugin for KarmaPlugin {
    fn new_from_env() -> Result<Self> {
        Ok(KarmaPlugin::new())
    }

    fn command_metadata(&self) -> Vec<CommandMetadata> {
        vec![CommandMetadata {
            name: "karma".to_string(),
            short_help: "".to_string(),
            full_help: "".to_string(),
        }]
    }

    async fn run(self, _bot: Arc<Client>, mut stream: EventStream) -> Result<()> {
        while let Some(ctx) = stream.next().await {
            let res = match ctx.as_event() {
                Ok(Event::Command("karma", Some(arg))) => self.handle_karma(&ctx, arg).await,
                Ok(Event::Message(_, msg)) => self.handle_privmsg(&ctx, msg).await,
                _ => Ok(()),
            };

            crate::check_err(&ctx, res).await;
        }

        Err(format_err!("karma plugin exited early"))
    }
}
