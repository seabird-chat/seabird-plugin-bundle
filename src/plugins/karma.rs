use std::collections::BTreeMap;
use std::fmt::Write;

use regex::Regex;

use crate::prelude::*;

#[derive(sqlx::FromRow, Debug)]
pub struct Karma {
    pub name: String,
    pub score: i32,
}

impl Karma {
    fn sanitize_name(name: &str) -> String {
        name.to_lowercase()
    }

    async fn get_by_name(conn: &sqlx::PgPool, name: &str) -> Result<Self> {
        Ok(
            sqlx::query_as!(Karma, "SELECT name, score FROM karma WHERE name=$1;", name)
                .fetch_optional(conn)
                .await?
                .unwrap_or_else(|| Karma {
                    name: name.to_string(),
                    score: 0,
                }),
        )
    }

    async fn create_or_update(conn: &sqlx::PgPool, name: &str, score: i32) -> Result<Self> {
        sqlx::query!(
            "INSERT INTO karma (name, score) VALUES ($1, $2)
ON CONFLICT (name) DO UPDATE SET score=EXCLUDED.score+karma.score;",
            name,
            score
        )
        .execute(conn)
        .await?;

        Karma::get_by_name(conn, &name).await
    }
}

pub struct KarmaPlugin {
    re: Regex,
}

enum ParseState {
    Nothing,

    ChangeToAdd,
    Add,

    ChangeToSubtract,
    Subtract,
}

/// Parse an input karma change string and return the resulting delta
fn parse_karma_change(change_str: &str) -> Result<i32> {
    let mut state = ParseState::Nothing;
    let mut counter = 0;

    let missing_successive_plus = "you need at least two successive \"+\"s to increment, silly!";
    let missing_successive_minus = "you need at least two successive \"-\"s to decrement, silly!";

    for c in change_str.chars() {
        match (c, &state) {
            ('+', ParseState::Nothing) => {
                state = ParseState::ChangeToAdd;
            }
            ('+', ParseState::ChangeToAdd) => {
                state = ParseState::Add;
                counter += 1;
            }
            ('+', ParseState::Add) => {
                counter += 1;
            }
            ('+', ParseState::ChangeToSubtract) => {
                return Err(format_err!(missing_successive_plus));
            }
            ('+', ParseState::Subtract) => {
                state = ParseState::ChangeToAdd;
            }
            ('-', ParseState::Nothing) => {
                state = ParseState::ChangeToSubtract;
            }
            ('-', ParseState::ChangeToAdd) => {
                return Err(format_err!(missing_successive_minus));
            }
            ('-', ParseState::Add) => {
                state = ParseState::ChangeToSubtract;
            }
            ('-', ParseState::ChangeToSubtract) => {
                state = ParseState::Subtract;
                counter -= 1;
            }
            ('-', ParseState::Subtract) => {
                counter -= 1;
            }
            (unsupported_char, _) => {
                return Err(format_err!(
                    "character \"{}\" not supported by the Karma Adjustment Bureau",
                    unsupported_char
                ));
            }
        }
    }

    match state {
        ParseState::ChangeToAdd => Err(format_err!(missing_successive_plus)),
        ParseState::ChangeToSubtract => Err(format_err!(missing_successive_minus)),
        ParseState::Nothing => {
            // This state should be impossible given the regex we use, but
            // why not be safe?
            Err(format_err!(
                "you didn't even try to adjust karma on this one!"
            ))
        }
        ParseState::Add | ParseState::Subtract => Ok(counter),
    }
}

impl KarmaPlugin {
    pub fn new() -> Self {
        KarmaPlugin {
            re: Regex::new(r#"([\w]{2,}|".+?")([+-]+)(?:\s|$)"#).unwrap(),
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

        if captures.is_empty() {
            return Ok(());
        }

        let mut changes = BTreeMap::new();
        let mut change_errors = Vec::new();

        // Loop through all captures, adding them to the output.
        for capture in captures {
            let cleaned_name = Karma::sanitize_name(&capture[1].trim_matches('"'));

            match parse_karma_change(&capture[2]) {
                Ok(change) => {
                    *changes.entry(cleaned_name).or_insert(0) += change;
                }
                Err(e) => {
                    change_errors.push(format!(
                        "invalid karma change for \"{}\": {}",
                        cleaned_name, e
                    ));
                }
            }
        }

        let db = ctx.get_db();

        let mut lines = Vec::new();

        for (name, raw_change) in changes.into_iter() {
            let change = utils::clamp(raw_change, -5, 5);

            let karma = Karma::create_or_update(&db, &name, change).await?;

            let mut line = String::new();

            write!(line, "{}'s karma is now {}", name, karma.score)?;

            if raw_change != change {
                line.push_str(". Buzzkill Mode (tm) enforced a limit of 5");
            } else if raw_change == 0 {
                line.push_str(". Well done. Nothing happened.");
            }

            lines.push(line);
        }

        lines.append(&mut change_errors);

        // Send the bulk message
        ctx.reply(&lines.join(", ")).await?;

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

    async fn run(self, bot: Arc<Client>) -> Result<()> {
        let mut stream = bot.subscribe();

        while let Ok(ctx) = stream.recv().await {
            let res = match ctx.as_event() {
                Ok(Event::Command("karma", possible_arg)) => {
                    let nick = possible_arg.or_else(|| ctx.sender());

                    match nick {
                        Some(nick) => self.handle_karma(&ctx, nick).await,
                        None => Err(format_err!(
                            "no nick found to use for karma check (not provided in source message)"
                        )),
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_single_karma_increment() -> Result<()> {
        assert_eq!(parse_karma_change("++")?, 1);

        Ok(())
    }

    #[test]
    fn test_multiple_karma_increment() -> Result<()> {
        assert_eq!(parse_karma_change("+++")?, 2);
        assert_eq!(parse_karma_change("++++")?, 3);

        Ok(())
    }

    #[test]
    fn test_single_karma_decrement() -> Result<()> {
        assert_eq!(parse_karma_change("--")?, -1);

        Ok(())
    }

    #[test]
    fn test_multiple_karma_decrement() -> Result<()> {
        assert_eq!(parse_karma_change("---")?, -2);
        assert_eq!(parse_karma_change("----")?, -3);

        Ok(())
    }

    #[test]
    fn test_no_karma_change() -> Result<()> {
        assert_eq!(parse_karma_change("++--")?, 0);
        assert_eq!(parse_karma_change("--++")?, 0);

        assert_eq!(parse_karma_change("++++----")?, 0);
        assert_eq!(parse_karma_change("----++++")?, 0);

        assert_eq!(parse_karma_change("++--++--")?, 0);
        assert_eq!(parse_karma_change("--++--++")?, 0);

        Ok(())
    }

    #[test]
    fn test_malformed_karma_change() {
        assert!(parse_karma_change("").is_err());
        assert!(parse_karma_change("+").is_err());
        assert!(parse_karma_change("-").is_err());

        assert!(parse_karma_change("+-").is_err());
        assert!(parse_karma_change("-+").is_err());

        assert!(parse_karma_change("++-").is_err());
        assert!(parse_karma_change("--+").is_err());

        assert!(parse_karma_change("++-++").is_err());
        assert!(parse_karma_change("--+--").is_err());
    }
}
