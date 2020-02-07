use std::io::BufRead;
use std::sync::Arc;
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

async fn lines_from_url(url: &str) -> Result<std::io::Lines<std::io::Cursor<String>>> {
    let start = Instant::now();
    let data = reqwest::get(url).await?.error_for_status()?.text().await?;

    trace!(
        "Got station information from \"{}\" in {}ms",
        url,
        start.elapsed().as_millis()
    );

    Ok(std::io::Cursor::new(data).lines())
}

#[async_trait]
impl Plugin for NoaaPlugin {
    async fn handle_message(&self, ctx: &Arc<Context>) -> Result<()> {
        match ctx.as_event() {
            Event::Command("metar", Some(station)) => {
                let mut station = station.to_string();
                station.make_ascii_uppercase();

                let mut lines = lines_from_url(&format!(
                    "{}/observations/metar/stations/{}.TXT",
                    self.base_url, station
                ))
                .await?;
                let _ = lines.next();
                let line = lines
                    .next()
                    .transpose()?
                    .ok_or_else(|| anyhow::anyhow!("No results"))?;

                ctx.mention_reply(&line[..]).await?;
            }
            // TODO: implement stored stations
            Event::Command("metar", None) => {
                ctx.mention_reply(&format!(
                    "Missing station argument. Usage: {}metar <station>",
                    ctx.command_prefix()
                ))
                .await?;
            }
            Event::Command("taf", Some(station)) => {
                let mut station = station.to_string();
                station.make_ascii_uppercase();

                let mut lines = lines_from_url(&format!(
                    "{}/forecasts/taf/stations/{}.TXT",
                    self.base_url, station
                ))
                .await?;
                let _ = lines.next();

                for line in lines {
                    if let Ok(line) = line {
                        ctx.mention_reply(&line.trim()).await?;
                    }
                }
            }
            // TODO: implement stored stations
            Event::Command("taf", None) => {
                ctx.mention_reply(&format!(
                    "Missing station argument. Usage: {}taf <station>",
                    ctx.command_prefix()
                ))
                .await?;
            }
            _ => {}
        }

        Ok(())
    }
}
