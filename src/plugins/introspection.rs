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
const GIT_VERSION: &str = git_version!();

#[async_trait]
impl Plugin for IntrospectionPlugin {
    fn new_from_env() -> Result<Self> {
        Ok(IntrospectionPlugin::new())
    }

    async fn run(self, _bot: Arc<Client>, mut stream: EventStream) -> Result<()> {
        while let Some(ctx) = stream.next().await {
            match ctx.as_event() {
                Event::Command("uptime", _) => {
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
                Event::Command("version", _) => {
                    ctx.mention_reply(&format!("seabird {}-{}", SEABIRD_VERSION, GIT_VERSION))
                        .await?;
                }
                _ => {}
            }
        }

        Err(format_err!("introspection plugin exited early"))
    }
}
