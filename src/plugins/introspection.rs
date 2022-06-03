use std::fmt::Write;

use git_version::git_version;
use time::{Duration, Instant};

use crate::prelude::*;

pub struct IntrospectionPlugin {
    started: Instant,
}

const SEABIRD_VERSION: &str = env!("CARGO_PKG_VERSION");

// Pull the git hash, but allow us to fall back to the SOURCE_COMMIT variable
// which is used in automated Docker Hub builds. For some reason Docker Hub
// doesn't give us access to the full repository, so we have to use this as a
// fallback. Thankfully, this will fail if the SOURCE_COMMIT variable also isn't
// defined.
const GIT_VERSION: &str = git_version!();

fn format_duration(dur: &Duration) -> Result<String> {
    let days = dur.whole_days();
    let hours = dur.whole_hours() - (days * 24);
    let minutes = dur.whole_minutes() - (days * 24 * 60) - (hours * 60);

    let mut ret = String::new();

    if days > 0 {
        write!(ret, "{} days ", days)?;
    }

    write!(ret, "{:02}:{:02}", hours, minutes)?;

    Ok(ret)
}

impl IntrospectionPlugin {
    pub fn new() -> Self {
        IntrospectionPlugin {
            started: Instant::now(),
        }
    }

    async fn handle_uptime(&self, ctx: &Context) -> Result<()> {
        let elapsed = self.started.elapsed();

        let resp = ctx.get_core_info().await?;
        println!("{} {}", resp.current_timestamp, resp.startup_timestamp);
        let core_uptime = Duration::new(
            (resp.current_timestamp - resp.startup_timestamp).try_into()?,
            0,
        );

        let mut ret = String::new();

        write!(
            ret,
            "Core Uptime: {}, Plugin Uptime: {}",
            format_duration(&core_uptime)?,
            format_duration(&elapsed)?
        )?;

        ctx.mention_reply(&ret[..]).await
    }

    async fn handle_backends(&self, ctx: &Context) -> Result<()> {
        let response = ctx.list_backends().await?;

        let mut lines = Vec::new();
        for backend in response.backends.into_iter() {
            lines.push(backend.id);
        }

        lines.sort();

        ctx.mention_reply(&lines.join(", ")).await
    }

    async fn handle_backend_metadata(&self, ctx: &Context, arg: &str) -> Result<()> {
        let response = ctx.get_backend_info(arg.to_string()).await?;

        let mut lines = Vec::new();
        for (key, value) in response.metadata.into_iter() {
            lines.push(format!("{}={}", key, value));
        }

        lines.sort();

        ctx.mention_reply(&lines.join(", ")).await
    }

    async fn handle_version(&self, ctx: &Context) -> Result<()> {
        ctx.mention_reply(&format!(
            "seabird-plugin-bundle {}-{}",
            SEABIRD_VERSION, GIT_VERSION
        ))
        .await
    }
}

#[async_trait]
impl Plugin for IntrospectionPlugin {
    fn new_from_env() -> Result<Self> {
        Ok(IntrospectionPlugin::new())
    }

    fn command_metadata(&self) -> Vec<CommandMetadata> {
        vec![
            CommandMetadata {
                name: "uptime".to_string(),
                short_help: "usage: uptime. gets current introspection plugin uptime.".to_string(),
                full_help: "gets current introspection plugin uptime.".to_string(),
            },
            CommandMetadata {
                name: "backends".to_string(),
                short_help: "usage: backends. gets all connected backends.".to_string(),
                full_help: "gets all connected backends.".to_string(),
            },
            CommandMetadata {
                name: "backend_metadata".to_string(),
                short_help:
                    "usage: backend_metadata <backend_id>. gets metadata for the given backend."
                        .to_string(),
                full_help: "gets metadata for the given backend.".to_string(),
            },
            CommandMetadata {
                name: "version".to_string(),
                short_help: "usage: version. gets introspection plugin version.".to_string(),
                full_help: "gets introspection plugin version.".to_string(),
            },
        ]
    }

    async fn run(self, bot: Arc<Client>) -> Result<()> {
        let mut stream = bot.subscribe();

        while let Ok(ctx) = stream.recv().await {
            let res = match ctx.as_event() {
                Ok(Event::Command("uptime", _)) => self.handle_uptime(&ctx).await,
                Ok(Event::Command("backends", _)) => self.handle_backends(&ctx).await,
                Ok(Event::Command("backend_metadata", None)) => {
                    Err(format_err!("usage: backend_metadata <backend_id>"))
                }
                Ok(Event::Command("backend_metadata", Some(arg))) => {
                    self.handle_backend_metadata(&ctx, arg).await
                }
                Ok(Event::Command("version", _)) => self.handle_version(&ctx).await,
                _ => Ok(()),
            };

            crate::check_err(&ctx, res).await;
        }

        Err(format_err!("introspection plugin lagged"))
    }
}
