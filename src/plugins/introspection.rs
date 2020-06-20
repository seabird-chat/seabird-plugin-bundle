use std::fmt::Write;

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
}

const SEABIRD_VERSION: &str = env!("CARGO_PKG_VERSION");

// Pull the git hash, but allow us to fall back to the SOURCE_COMMIT variable
// which is used in automated Docker Hub builds. For some reason Docker Hub
// doesn't give us access to the full repository, so we have to use this as a
// fallback. Thankfully, this will fail if the SOURCE_COMMIT variable also isn't
// defined.
const GIT_VERSION: &str = git_version!(fallback = env!("SOURCE_COMMIT"),);

#[async_trait]
impl Plugin for IntrospectionPlugin {
    fn new_from_env() -> Result<Self> {
        Ok(IntrospectionPlugin::new())
    }

    fn command_metadata(&self) -> Vec<CommandMetadata> {
        vec![CommandMetadata {
            name: "uptime".to_string(),
            short_help: "".to_string(),
            full_help: "".to_string(),
        }]
    }

    async fn run(self, _bot: Arc<Client>, mut stream: EventStream) -> Result<()> {
        while let Some(ctx) = stream.next().await {
            match ctx.as_event() {
                Ok(Event::Command("uptime", _)) => {
                    let elapsed = self.started.elapsed();

                    let days = elapsed.whole_days();
                    let hours = elapsed.whole_hours() - (days * 24);
                    let minutes = elapsed.whole_minutes() - (days * 24 * 60) - (hours * 60);

                    let mut ret = String::new();

                    if days > 0 {
                        write!(ret, "{} days ", days).unwrap();
                    }

                    write!(ret, "{:02}:{:02}", hours, minutes).unwrap();

                    ctx.mention_reply(&ret[..]).await?;
                }
                Ok(Event::Command("version", _)) => {
                    ctx.mention_reply(&format!("seabird {}-{}", SEABIRD_VERSION, GIT_VERSION))
                        .await?;
                }
                _ => {}
            }
        }

        Err(format_err!("introspection plugin exited early"))
    }
}
