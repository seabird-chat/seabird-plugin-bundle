use std::time::Duration;

use crate::prelude::*;

pub struct RemindPlugin {
    db_pool: sqlx::SqlitePool,
}

#[derive(sqlx::FromRow)]
struct Reminder {
    id: i64,
    channel_id: String,
    target_user: String,
    message: String,
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
        _ => return Err(format_err!("Unknown duration unit '{}'. Use s/m/h/d/w", unit)),
    };

    Ok(Duration::from_secs(seconds))
}

fn looks_like_duration(s: &str) -> bool {
    if s.len() < 2 {
        return false;
    }
    let (num_str, unit) = s.split_at(s.len() - 1);
    num_str.parse::<u64>().is_ok() && matches!(unit, "s" | "m" | "h" | "d" | "w")
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
    async fn handle_remind(&self, ctx: &Arc<Context>, arg: Option<&str>) -> Result<()> {
        let arg = match arg {
            Some(a) => a,
            None => {
                ctx.mention_reply("Usage: remind [user] <time> <message>").await?;
                return Ok(());
            }
        };

        let parts: Vec<&str> = arg.splitn(3, ' ').collect();
        if parts.len() < 2 {
            ctx.mention_reply("Usage: remind [user] <time> <message>").await?;
            return Ok(());
        }

        let sender = ctx
            .sender()
            .ok_or_else(|| format_err!("Could not determine sender"))?;

        let (target_user, duration_str, message) = if looks_like_duration(parts[0]) {
            let msg = if parts.len() > 1 { parts[1..].join(" ") } else { String::new() };
            (sender.to_string(), parts[0], msg)
        } else {
            if parts.len() < 3 {
                ctx.mention_reply("Usage: remind [user] <time> <message>").await?;
                return Ok(());
            }
            (parts[0].to_string(), parts[1], parts[2].to_string())
        };

        if message.is_empty() {
            ctx.mention_reply("Please provide a reminder message").await?;
            return Ok(());
        }

        let duration = match parse_duration(duration_str) {
            Ok(d) => d,
            Err(e) => {
                ctx.mention_reply(&format!("{}", e)).await?;
                return Ok(());
            }
        };

        let channel_id = ctx
            .target_channel_id()
            .ok_or_else(|| format_err!("Could not determine channel"))?;

        let remind_at = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_secs() as i64
            + duration.as_secs() as i64;

        sqlx::query!(
            "INSERT INTO reminders (channel_id, target_user, message, remind_at, created_by) VALUES ($1, $2, $3, $4, $5)",
            channel_id,
            target_user,
            message,
            remind_at,
            sender
        )
        .execute(&ctx.get_db())
        .await?;

        let duration_text = format_duration(duration.as_secs() as i64);
        ctx.mention_reply(&format!(
            "I'll remind {} in {}",
            if target_user == sender { "you".to_string() } else { target_user },
            duration_text
        ))
        .await?;

        Ok(())
    }

    async fn check_due_reminders(&self, bot: &Arc<Client>) -> Result<()> {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_secs() as i64;

        let reminders: Vec<Reminder> = sqlx::query_as!(
            Reminder,
            "SELECT id, channel_id, target_user, message, created_by FROM reminders WHERE remind_at <= $1",
            now
        )
        .fetch_all(&self.db_pool)
        .await?;

        for reminder in reminders {
            let msg = if reminder.created_by == reminder.target_user {
                format!("{}: Reminder: {}", reminder.target_user, reminder.message)
            } else {
                format!(
                    "{}: Reminder from {}: {}",
                    reminder.target_user, reminder.created_by, reminder.message
                )
            };

            if let Err(e) = bot.send_message(&reminder.channel_id, &msg).await {
                error!("Failed to send reminder: {}", e);
                continue;
            }

            sqlx::query!("DELETE FROM reminders WHERE id = $1", reminder.id)
                .execute(&self.db_pool)
                .await?;
        }

        Ok(())
    }
}

#[async_trait]
impl Plugin for RemindPlugin {
    fn new_from_env() -> Result<Self> {
        let db_url = dotenvy::var("DATABASE_URL")
            .map_err(|_| format_err!("Missing $DATABASE_URL"))?;
        let db_pool = sqlx::sqlite::SqlitePoolOptions::new()
            .max_connections(2)
            .connect_lazy(&db_url)?;
        Ok(RemindPlugin { db_pool })
    }

    fn command_metadata(&self) -> Vec<CommandMetadata> {
        vec![CommandMetadata {
            name: "remind".to_string(),
            short_help: "usage: remind [user] <time> <message>".to_string(),
            full_help: "Set a reminder. Time format: 30s, 5m, 2h, 1d, 1w".to_string(),
        }]
    }

    async fn run(self, bot: Arc<Client>) -> Result<()> {
        let mut stream = bot.subscribe();
        let mut interval = tokio::time::interval(Duration::from_secs(30));

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
