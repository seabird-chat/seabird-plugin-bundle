use std::fmt::Write;
use std::sync::Arc;

use async_trait::async_trait;
use time::{Duration, Instant};

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
    async fn handle_message(&self, ctx: &Arc<Context>) -> Result<()> {
        if let Event::Command("uptime", _) = ctx.as_event() {
            let elapsed = self.started.elapsed();

            let days = elapsed.whole_days();
            let elapsed = elapsed - Duration::days(days);
            let hours = elapsed.whole_hours();
            let elapsed = elapsed - Duration::hours(hours);
            let minutes = elapsed.whole_minutes();

            let mut ret = String::new();

            if days > 0 {
                write!(ret, "{} days ", days)?;
            }

            write!(ret, "{:02}:{:02}", hours, minutes)?;

            ctx.mention_reply(&ret[..]).await?;
        }

        Ok(())
    }
}
