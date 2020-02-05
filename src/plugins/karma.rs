use std::convert::TryInto;

use async_trait::async_trait;
use diesel::prelude::*;
use diesel::Queryable;
use regex::Regex;

use crate::prelude::*;
use crate::schema::karma;

#[derive(Queryable)]
pub struct Karma {
    pub name: String,
    pub score: i32,
}

type AllColumns = (karma::name, karma::score);
pub const ALL_COLUMNS: AllColumns = (karma::name, karma::score);

impl Karma {
    fn get_by_name(conn: &DbConn, name: &str) -> Result<Option<Self>> {
        Ok(karma::table
            .select(ALL_COLUMNS)
            .filter(karma::name.eq(name))
            .first::<Karma>(conn)
            .optional()?)
    }

    fn create_or_update(conn: &DbConn, name: &str, score: i32) -> Result<Self> {
        Ok(diesel::insert_into(karma::table)
            .values((karma::name.eq(name), karma::score.eq(score)))
            .on_conflict(karma::name)
            .do_update()
            .set(karma::score.eq(karma::score + score))
            .returning(ALL_COLUMNS)
            .get_result(conn)?)
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
    async fn handle_message(&self, ctx: &Context) -> Result<()> {
        match ctx.as_event() {
            Event::Command("karma", Some(arg)) => {
                let inner_arg = arg.to_string();

                let conn = ctx.db_pool.get()?;
                let karma_result =
                    tokio::task::spawn_blocking(move || Karma::get_by_name(&conn, &inner_arg[..]))
                        .await??
                        .map_or(0, |k| k.score);

                ctx.mention_reply(&format!("{}'s karma is {}", arg, karma_result))
                    .await?;
            }
            Event::Privmsg(_, msg) => {
                let captures: Vec<_> = self.re.captures_iter(msg).collect();

                if !captures.is_empty() {
                    for capture in captures {
                        let name = capture[1].to_string();
                        let mut change: i32 = (&capture[2].len() - 1).try_into().unwrap();
                        if capture[2].starts_with('-') {
                            change *= -1;
                        }

                        let conn = ctx.db_pool.get()?;
                        let karma_result = tokio::task::spawn_blocking(move || {
                            Karma::create_or_update(&conn, &name[..], change)
                        })
                        .await??;

                        ctx.reply(&format!(
                            "{}'s karma is now {}",
                            karma_result.name, karma_result.score
                        ))
                        .await?;
                    }
                }
            }
            _ => {}
        }

        Ok(())
    }
}
