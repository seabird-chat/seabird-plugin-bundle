use std::convert::TryInto;
use std::sync::Arc;

use async_trait::async_trait;
use regex::Regex;

use crate::prelude::*;

#[derive(Debug)]
pub struct Karma {
    pub name: String,
    pub score: i32,
}

impl Karma {
    async fn get_by_name(conn: Arc<tokio_postgres::Client>, name: &str) -> Result<Self> {
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
        conn: Arc<tokio_postgres::Client>,
        name: &str,
        score: i32,
    ) -> Result<Self> {
        conn.execute(
            "INSERT INTO karma (name, score) VALUES ($1, $2)
ON CONFLICT (name) DO UPDATE SET score=EXCLUDED.score;",
            &[&name, &score],
        )
        .await?;

        Karma::get_by_name(conn, name).await
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

#[async_trait]
impl Plugin for KarmaPlugin {
    async fn handle_message(&self, ctx: &Arc<Context>) -> Result<()> {
        match ctx.as_event() {
            Event::Command("karma", Some(arg)) => {
                let karma = Karma::get_by_name(ctx.get_db(), arg).await?;

                ctx.mention_reply(&format!("{}'s karma is {}", arg, karma.score))
                    .await?;
            }
            Event::Privmsg(_, msg) => {
                let captures: Vec<_> = self.re.captures_iter(msg).collect();

                if !captures.is_empty() {
                    for capture in captures {
                        let name = &capture[1];
                        let mut change: i32 = (&capture[2].len() - 1).try_into().unwrap();
                        if capture[2].starts_with('-') {
                            change *= -1;
                        }

                        let karma = Karma::create_or_update(ctx.get_db(), name, change).await?;

                        ctx.reply(&format!("{}'s karma is now {}", karma.name, karma.score))
                            .await?;
                    }
                }
            }
            _ => {}
        }

        Ok(())
    }
}
