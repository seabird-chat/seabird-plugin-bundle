use std::convert::TryInto;

use async_trait::async_trait;
use diesel::prelude::*;
use diesel::result::OptionalExtension;
use diesel::Queryable;
use regex::Regex;

use crate::prelude::*;
use crate::schema::karma::dsl as karma;

#[derive(Queryable)]
pub struct Karma {
    pub id: i32,
    pub name: String,
    pub score: i32,
}

pub struct KarmaPlugin {
    re: Regex,
}

#[async_trait]
trait KarmaExt {
    async fn get_karma(&self, name: String) -> Result<i32>;
    async fn update_karma(&self, name: String, change: i32) -> Result<i32>;
}

#[async_trait]
impl KarmaExt for DbPool {
    async fn get_karma(&self, name: String) -> Result<i32> {
        let conn = self.get()?;

        Ok(tokio::task::spawn_blocking(move || {
            karma::karma
                .filter(karma::name.eq(name))
                .first::<Karma>(&conn)
                .optional()
        })
        .await??
        .map_or(0, |k| k.score))
    }

    async fn update_karma(&self, name: String, change: i32) -> Result<i32> {
        let conn = self.get()?;

        Ok(tokio::task::spawn_blocking(move || {
            diesel::insert_into(karma::karma)
                .values((karma::name.eq(&name), karma::score.eq(change)))
                .on_conflict(karma::name)
                .do_update()
                .set(karma::score.eq(karma::score + change))
                .execute(&conn)?;

            karma::karma
                .filter(karma::name.eq(&name))
                .first::<Karma>(&conn)
                .optional()
        })
        .await??
        .map_or(0, |k| k.score))
    }
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
        match ctx.as_event() {
            Event::Command("karma", Some(arg)) => {
                let arg = arg.to_string();

                let karma_result = ctx.db_pool.get_karma(arg.clone()).await?;
                ctx.mention_reply(&format!("{}'s karma is {}", arg, karma_result))
                    .await?;
            }
            Event::Privmsg(_, msg) => {
                let captures: Vec<_> = self.re.captures_iter(msg).collect();

                for capture in captures {
                    let mut change: i32 = (&capture[2].len() - 1).try_into().unwrap();
                    if capture[2].starts_with('-') {
                        change *= -1;
                    }

                    let karma_result = ctx
                        .db_pool
                        .update_karma(capture[1].to_string(), change)
                        .await?;

                    ctx.reply(&format!("{}'s karma is now {}", &capture[1], karma_result))
                        .await?;
                }
            }
            _ => {}
        }

        Ok(())
    }
}
