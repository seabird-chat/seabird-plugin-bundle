use std::io::BufRead;
use std::time::Instant;

use async_trait::async_trait;
use tracing::trace;

use crate::prelude::*;

pub struct NoaaPlugin {
    base_url: &'static str,
}

impl NoaaPlugin {
    pub fn new() -> Self {
        NoaaPlugin {
            base_url: "https://tgftp.nws.noaa.gov/data",
        }
    }
}

#[async_trait]
impl Plugin for NoaaPlugin {
    async fn handle_message(&self, ctx: &Context) -> Result<()> {
        match ctx.as_event() {
            Event::Command("metar", Some(station)) => {
                let mut station = station.to_string();
                station.make_ascii_uppercase();

                let start = Instant::now();
                let data = reqwest::get(
                    &format!(
                        "{}/observations/metar/stations/{}.TXT",
                        self.base_url, station
                    )[..],
                )
                .await?
                .error_for_status()?
                .text()
                .await?;
                trace!(
                    "Got station information for {} in {}ms",
                    station,
                    start.elapsed().as_millis()
                );

                // Because the first line is the date, we need to skip it.
                let mut lines = std::io::Cursor::new(data).lines();
                let _ = lines.next();
                let line = lines
                    .next()
                    .transpose()?
                    .ok_or_else(|| anyhow::anyhow!("No results"))?;

                ctx.mention_reply(&line[..]).await?;
            }
            Event::Command("metar", None) => {}
            _ => {}
        }

        Ok(())
    }
}
