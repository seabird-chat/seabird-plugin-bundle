use std::fmt::Write;
use std::time::SystemTime;

use git_version::git_version;
use time::Instant;

use crate::prelude::*;

pub struct IntrospectionPlugin {
    started: Instant,
}

impl IntrospectionPlugin {
    pub fn new() -> Self {
        IntrospectionPlugin {
            started: Instant::now(),
        }
    }

    async fn reply_with_uptime(&self, ctx: &Arc<Context>) -> Result<()> {
        let elapsed = self.started.elapsed();

        let days = elapsed.whole_days();
        let hours = elapsed.whole_hours() - (days * 24);
        let minutes = elapsed.whole_minutes() - (days * 24 * 60) - (hours * 60);

        let mut ret = String::new();

        if days > 0 {
            write!(ret, "{} days ", days).unwrap();
        }

        write!(ret, "{:02}:{:02}", hours, minutes).unwrap();

        ctx.mention_reply(&ret[..]).await
    }

    async fn reply_with_core_uptime(&self, ctx: &Arc<Context>) -> Result<()> {
        let info = ctx.get_core_info().await?;

        let current_time = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)?
            .as_secs();

        if current_time < info.startup_timestamp {
            ctx.mention_reply("server somehow started in the future")
                .await?;

            return Ok(());
        }

        let elapsed_seconds = current_time - info.startup_timestamp;

        let days = elapsed_seconds / (60 * 60 * 24);
        let hours = elapsed_seconds / (60 * 60) - days * 24;
        let minutes = elapsed_seconds / 60 - days * 24 * 60 - hours * 60;

        let mut ret = String::new();

        if days > 0 {
            write!(ret, "{} day(s) ", days).unwrap();
        }

        write!(ret, "{:02}:{:02}", hours, minutes).unwrap();

        ctx.mention_reply(&ret[..]).await
    }

    async fn reply_with_version(&self, ctx: &Arc<Context>) -> Result<()> {
        ctx.mention_reply(&format!(
            "seabird-plugin-bundle {}-{}",
            SEABIRD_VERSION, GIT_VERSION
        ))
        .await
    }

    async fn handle_say(&self, ctx: &Arc<Context>, field: &str) -> Result<()> {
        match field {
            "uptime" => self.reply_with_uptime(ctx).await,
            "core_uptime" => self.reply_with_core_uptime(ctx).await,
            "version" => self.reply_with_version(ctx).await,
            _ => ctx.mention_reply(&format!("unknown field {}", field)).await,
        }
    }

    async fn handle_mention(&self, ctx: &Arc<Context>, message: &str) -> Result<()> {
        let parts: Vec<&str> = message.split(" ").collect();

        match parts.get(0) {
            Some(&"say") => match parts.get(1) {
                Some(field) => self.handle_say(ctx, field).await,
                None => ctx.mention_reply("must provide a field").await,
            },
            _ => Ok(()),
        }
    }
}

const SEABIRD_VERSION: &str = env!("CARGO_PKG_VERSION");

// Pull the git hash, but allow us to fall back to the SOURCE_COMMIT variable
// which is used in automated Docker Hub builds. For some reason Docker Hub
// doesn't give us access to the full repository, so we have to use this as a
// fallback. Thankfully, this will fail if the SOURCE_COMMIT variable also isn't
// defined.
const GIT_VERSION: &str = git_version!();

#[async_trait]
impl Plugin for IntrospectionPlugin {
    fn new_from_env() -> Result<Self> {
        Ok(IntrospectionPlugin::new())
    }

    fn command_metadata(&self) -> Vec<CommandMetadata> {
        vec![
            CommandMetadata {
                name: "uptime".to_string(),
                short_help: "plugin uptime".to_string(),
                full_help: "plugin uptime".to_string(),
            },
            CommandMetadata {
                name: "core_uptime".to_string(),
                short_help: "seabird core uptime".to_string(),
                full_help: "seabird core uptime".to_string(),
            },
            CommandMetadata {
                name: "version".to_string(),
                short_help: "plugin version".to_string(),
                full_help: "plugin version".to_string(),
            },
        ]
    }

    async fn run(self, bot: Arc<Client>) -> Result<()> {
        let mut stream = bot.subscribe();

        while let Ok(ctx) = stream.recv().await {
            let res = match ctx.as_event() {
                Ok(Event::Command("uptime", _)) => self.reply_with_uptime(&ctx).await,
                Ok(Event::Command("core_uptime", _)) => self.reply_with_core_uptime(&ctx).await,
                Ok(Event::Command("version", _)) => self.reply_with_version(&ctx).await,
                Ok(Event::Mention(message)) => self.handle_mention(&ctx, message).await,
                _ => Ok(()),
            };

            crate::check_err(&ctx, res).await;
        }

        Err(format_err!("introspection plugin lagged"))
    }
}
