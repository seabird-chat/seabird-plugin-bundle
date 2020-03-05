use std::sync::Arc;

use async_trait::async_trait;
use regex::Regex;

use crate::prelude::*;

/*
// These are roughly in the order that they appear in xkcd-Bucket
literalRegexp  = regexp.MustCompile(`(?i)^literal(?:\[(\d+)\])? (.*)$`)
undoRegexp     = regexp.MustCompile(`(?i)^undo(?: last)?$`)
mergeRegexp    = regexp.MustCompile(`(?i)^merge (.*) [-=]> (.*)$`)
aliasRegexp    = regexp.MustCompile(`(?i)^alias (.*) [-=]> (.*)$`)
lookupRegexp   = regexp.MustCompile(`(?i)^lookup (.*)$`)
forgetIsRegexp = regexp.MustCompile(`(?i)^forget (.+?) (is|is also|are|<\w+>) (.+)$`) // Custom feature
forgetRegexp   = regexp.MustCompile(`(?i)^forget (.*)$`)
whatRegexp     = regexp.MustCompile(`(?i)^what was that\??$`)

renderRegexp = regexp.MustCompile(`(?i)^render (.*)$`) // Custom feature
isRegexp     = regexp.MustCompile(`(?i)^(.+?) (is|is also|are|<\w+>) (.+)$`)
*/

#[derive(Debug)]
pub struct BucketFact {
    pub id: i32,
    pub fact: String,
    pub verb: String,
    pub tidbit: String,
}

impl BucketFact {
    fn insert(conn: &DbConn, fact: &str, verb: &str, tidbit: &str) -> Result<usize> {
        Ok(diesel::insert_into(bucket_facts::table)
            .values((
                bucket_facts::fact.eq(fact),
                bucket_facts::verb.eq(verb),
                bucket_facts::tidbit.eq(tidbit),
            ))
            .execute(conn)?)
    }

    fn get_by_name(conn: &DbConn, name: &str) -> Result<Option<Self>> {
        Ok(bucket_facts::table
            .select(ALL_COLUMNS)
            .filter(bucket_facts::fact.eq(name))
            .first::<BucketFact>(conn)
            .optional()?)
    }
}

pub struct BucketPlugin {
    re_is: Regex,
}

impl BucketPlugin {
    pub fn new() -> Self {
        BucketPlugin {
            re_is: Regex::new(r#"(?i)^(.+?) (is|is also|are|<\w+>) (.+)$"#).unwrap(),
        }
    }
}

#[async_trait]
impl Plugin for BucketPlugin {
    async fn handle_message(&self, ctx: &Arc<Context>) -> Result<()> {
        // For now all of these commands are mention only
        if let Event::Mention(arg) = ctx.as_event() {
            let arg = arg.to_string();

            if let Some(captures) = self.re_is.captures(&arg[..]) {
                let conn = ctx.get_db()?;

                let bucket_fact_result = tokio::task::block_in_place(move || {
                    BucketFact::insert(&conn, &captures[1], &captures[2], &captures[3])
                });

                ctx.mention_reply(&format!("{:?}", bucket_fact_result)[..])
                    .await?;
            } else {
                let conn = ctx.get_db()?;

                let bucket_fact_result =
                    tokio::task::spawn_blocking(move || BucketFact::get_by_name(&conn, &arg[..]))
                        .await?;

                ctx.mention_reply(&format!("{:?}", bucket_fact_result)[..])
                    .await?;
            }
        }

        Ok(())
    }
}
