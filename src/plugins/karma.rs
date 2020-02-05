use async_trait::async_trait;
use diesel::prelude::*;
use diesel::result::OptionalExtension;
use diesel::Queryable;
use regex::Regex;

use crate::prelude::*;
use crate::schema::karma;

#[derive(Queryable)]
pub struct Karma {
    pub id: i32,
    pub name: String,
    pub score: i32,
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
    async fn handle_message(&self, ctx: &Context) -> Result<()> {
        let conn = ctx.db_pool.get()?;

        match ctx.as_event() {
            Event::Command("karma", Some(arg)) => {
                let arg = arg.to_string();

                let karma_result = tokio::task::spawn_blocking(move || {
                    karma::table
                        .filter(karma::columns::name.eq(arg))
                        .first::<Karma>(&conn)
                        .optional()
                })
                .await??;

                if let Some(k) = karma_result {
                    ctx.mention_reply(&format!("{}'s karma is {}", k.name, k.score))
                        .await?;
                }
            }
            Event::Privmsg(_, msg) => {
                let captures: Vec<_> = self.re.captures_iter(msg).collect();
                if !captures.is_empty() {
                    for capture in captures {
                        println!("{} {}", &capture[1], &capture[2]);
                    }
                }
            }
            _ => {}
        }

        Ok(())
    }
}
