use std::time::Duration;

use crate::prelude::*;

pub struct RemindPlugin {}

#[derive(sqlx::FromRow)]
struct Reminder {
    id: i64,
    channel_id: String,
    target_user: String,
    message: String,
    remind_at: i64,
    created_at: i64,
    created_by: String,
}

fn parse_duration(s: &str) -> Result<Duration> {
    let s = s.trim().to_lowercase();
    if s.len() < 2 {
        return Err(format_err!("Invalid duration format"));
    }

    let (num_str, unit) = s.split_at(s.len() - 1);
    let num: u64 = num_str
        .parse()
        .map_err(|_| format_err!("Invalid number in duration"))?;

    let seconds = match unit {
        "s" => num,
        "m" => num * 60,
        "h" => num * 60 * 60,
        "d" => num * 60 * 60 * 24,
        "w" => num * 60 * 60 * 24 * 7,
        _ => {
            return Err(format_err!(
                "Unknown duration unit '{}'. Use s/m/h/d/w",
                unit
            ))
        }
    };

    Ok(Duration::from_secs(seconds))
}

fn format_duration(secs: i64) -> String {
    let secs = secs.unsigned_abs();
    if secs < 60 {
        format!("{} second{}", secs, if secs == 1 { "" } else { "s" })
    } else if secs < 3600 {
        let mins = secs / 60;
        format!("{} minute{}", mins, if mins == 1 { "" } else { "s" })
    } else if secs < 86400 {
        let hours = secs / 3600;
        format!("{} hour{}", hours, if hours == 1 { "" } else { "s" })
    } else {
        let days = secs / 86400;
        format!("{} day{}", days, if days == 1 { "" } else { "s" })
    }
}

impl RemindPlugin {
    fn new() -> Self {
        RemindPlugin {}
    }

    async fn handle_list(&self, ctx: &Arc<Context>) -> Result<()> {
        let sender = ctx
            .sender()
            .ok_or_else(|| format_err!("Could not determine sender"))?;

        let db = ctx.get_db();

        let reminders: Vec<Reminder> = sqlx::query_as!(
            Reminder,
            r#"SELECT id as "id!", channel_id, target_user, message, remind_at, created_at, created_by
               FROM reminders
               WHERE created_by = $1 OR target_user = $1
               ORDER BY remind_at ASC
               LIMIT 10"#,
            sender
        )
        .fetch_all(&db)
        .await?;

        if reminders.is_empty() {
            ctx.mention_reply("You have no pending reminders.").await?;
            return Ok(());
        }

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_secs() as i64;

        let lines: Vec<String> = reminders
            .iter()
            .map(|r| {
                let time_left = format_duration(r.remind_at - now);
                let target = if r.target_user == sender {
                    "you".to_string()
                } else {
                    r.target_user.clone()
                };
                format!(
                    "[{}] in {} for {}: \"{}\"",
                    r.id, time_left, target, r.message
                )
            })
            .collect();

        ctx.mention_reply(&lines.join(" | ")).await?;

        Ok(())
    }

    async fn handle_cancel(&self, ctx: &Arc<Context>, id_str: &str) -> Result<()> {
        let sender = ctx
            .sender()
            .ok_or_else(|| format_err!("Could not determine sender"))?;

        let id: i64 = match id_str.trim().parse() {
            Ok(id) => id,
            Err(_) => {
                ctx.mention_reply("Invalid reminder ID. Use 'remind list' to see your reminders.")
                    .await?;
                return Ok(());
            }
        };

        let db = ctx.get_db();

        let result = sqlx::query!(
            "DELETE FROM reminders WHERE id = $1 AND (created_by = $2 OR target_user = $2)",
            id,
            sender
        )
        .execute(&db)
        .await?;

        if result.rows_affected() == 0 {
            ctx.mention_reply("Reminder not found or you don't have permission to cancel it.")
                .await?;
        } else {
            ctx.mention_reply("Reminder cancelled.").await?;
        }

        Ok(())
    }

    async fn handle_add(
        &self,
        ctx: &Arc<Context>,
        target: &str,
        duration: std::time::Duration,
        message: &str,
    ) -> Result<()> {
        let sender = ctx.sender().unwrap_or("unknown sender");

        let channel_id = ctx
            .target_channel_id()
            .ok_or_else(|| format_err!("Could not determine channel"))?;

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_secs() as i64;
        let remind_at = now + duration.as_secs() as i64;

        let db = ctx.get_db();

        sqlx::query!(
            "INSERT INTO reminders (channel_id, target_user, message, remind_at, created_at, created_by) VALUES ($1, $2, $3, $4, $5, $6)",
            channel_id,
            target,
            message,
            remind_at,
            now,
            sender
        )
        .execute(&db)
        .await?;

        let duration_text = format_duration(duration.as_secs() as i64);
        ctx.mention_reply(&format!(
            "I'll remind {} in {}: \"{}\"",
            if target == sender { "you" } else { target },
            duration_text,
            message
        ))
        .await?;

        Ok(())
    }

    async fn handle_remind(&self, ctx: &Arc<Context>, arg: Option<&str>) -> Result<()> {
        let arg = match arg {
            Some(a) => a,
            None => {
                ctx.mention_reply(
                    "Usage: remind <user|me> <time> <message> | remind list | remind cancel <id>",
                )
                .await?;
                return Ok(());
            }
        };

        let mut parts = arg.splitn(2, ' ');
        let first_arg = parts.next().unwrap_or("");
        let rest = parts.next();

        match first_arg {
            "list" => {
                self.handle_list(ctx).await?;
                return Ok(());
            }
            "cancel" => {
                let id_str = match rest {
                    None => {
                        ctx.mention_reply("Usage: remind cancel <id>").await?;
                        return Ok(());
                    }
                    Some(id_str) => id_str,
                };

                self.handle_cancel(ctx, id_str).await?;
                return Ok(());
            }
            arg => {
                let (duration, message) = match rest.unwrap_or("").split_once(' ') {
                    None => {
                        ctx.mention_reply("Usage: remind <user|me> <time> <message>")
                            .await?;
                        return Ok(());
                    }
                    Some((time, message)) => (time, message),
                };

                let target = if arg.eq_ignore_ascii_case("me") {
                    ctx.sender().unwrap_or("unknown sender")
                } else {
                    arg
                };

                let duration = match parse_duration(duration) {
                    Err(e) => {
                        ctx.mention_reply(&format!("{}", e)).await?;
                        return Ok(());
                    }
                    Ok(d) => d,
                };

                self.handle_add(ctx, target, duration, message).await?;

                return Ok(());
            }
        }
    }

    async fn check_due_reminders(&self, bot: &Client) -> Result<()> {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_secs() as i64;

        let db = bot.get_db();

        let reminders: Vec<Reminder> = sqlx::query_as!(
            Reminder,
            r#"SELECT id as "id!", channel_id, target_user, message, remind_at, created_at, created_by FROM reminders WHERE remind_at <= $1"#,
            now
        )
        .fetch_all(&db)
        .await?;

        for reminder in reminders {
            let age = format_duration(now - reminder.created_at);
            let msg = if reminder.created_by == reminder.target_user {
                format!(
                    "{}: Reminder ({} ago): {}",
                    reminder.target_user, age, reminder.message
                )
            } else {
                format!(
                    "{}: Reminder from {} ({} ago): {}",
                    reminder.target_user, reminder.created_by, age, reminder.message
                )
            };

            if let Err(e) = bot.send_message(&reminder.channel_id, &msg).await {
                error!("Failed to send reminder: {}", e);
                continue;
            }

            sqlx::query!("DELETE FROM reminders WHERE id = $1", reminder.id)
                .execute(&db)
                .await?;
        }

        Ok(())
    }
}

#[async_trait]
impl Plugin for RemindPlugin {
    fn new_from_env() -> Result<Self> {
        Ok(RemindPlugin::new())
    }

    fn command_metadata(&self) -> Vec<CommandMetadata> {
        vec![CommandMetadata {
            name: "remind".to_string(),
            short_help: "usage: remind <user|me> <time> <message> | list | cancel <id>".to_string(),
            full_help: "Set a reminder. Time format: 30s, 5m, 2h, 1d, 1w. Use 'remind list' to see pending reminders, 'remind cancel <id>' to cancel one.".to_string(),
        }]
    }

    async fn run(self, bot: Arc<Client>) -> Result<()> {
        let mut stream = bot.subscribe();
        let mut interval = tokio::time::interval(Duration::from_secs(1));

        loop {
            tokio::select! {
                result = stream.recv() => {
                    let ctx = result?;
                    let res = match ctx.as_event() {
                        Ok(Event::Command("remind", arg)) => self.handle_remind(&ctx, arg).await,
                        _ => Ok(()),
                    };
                    crate::check_err(&ctx, res).await;
                }
                _ = interval.tick() => {
                    if let Err(e) = self.check_due_reminders(&bot).await {
                        error!("Error checking reminders: {}", e);
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_duration() {
        // Valid cases: (input, expected_seconds)
        let valid_cases = [
            ("5s", 5),
            ("90s", 90),  // More than 1 minute
            ("1m", 60),
            ("5m", 300),
            ("90m", 5400),  // More than 1 hour
            ("2h", 7200),
            ("25h", 90000),  // More than 1 day
            ("1d", 86400),
            ("1w", 604800),
            // Case insensitive and whitespace
            ("5M", 300),
            (" 2h ", 7200),
        ];

        for (input, expected) in valid_cases {
            assert_eq!(
                parse_duration(input).unwrap(),
                Duration::from_secs(expected),
                "Failed parsing '{}'",
                input
            );
        }

        // Invalid cases
        let invalid_cases = ["", "x", "5", "xm", "5x", "-5m"];
        for input in invalid_cases {
            assert!(
                parse_duration(input).is_err(),
                "Expected '{}' to fail parsing",
                input
            );
        }
    }

    #[test]
    fn test_format_duration() {
        let cases = [
            (1, "1 second"),
            (5, "5 seconds"),
            (60, "1 minute"),
            (300, "5 minutes"),
            (3600, "1 hour"),
            (7200, "2 hours"),
            (86400, "1 day"),
            (172800, "2 days"),
            (-60, "1 minute"), // negative values use unsigned_abs
        ];

        for (input, expected) in cases {
            assert_eq!(format_duration(input), expected, "Failed formatting {}", input);
        }
    }
}
