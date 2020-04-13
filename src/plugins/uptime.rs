use std::fmt::Write;

use time::Instant;

use crate::prelude::*;

pub struct UptimePlugin {
    started: Instant,
}

impl UptimePlugin {
    pub fn new() -> Self {
        UptimePlugin {
            started: Instant::now(),
        }
    }
}

#[async_trait]
impl Plugin for UptimePlugin {
    fn new_from_env() -> Result<Self> {
        Ok(UptimePlugin::new())
    }

    async fn run(self, mut stream: Receiver<Arc<Context>>) -> Result<()> {
        while let Some(ctx) = stream.next().await {
            if let Event::Command("uptime", _) = ctx.as_event() {
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
        }

        Err(format_err!("uptime plugin exited early"))
    }
}
