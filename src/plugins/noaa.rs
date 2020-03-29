use std::io::BufRead;
use std::time::Instant;

use crate::prelude::*;

pub struct NoaaPlugin {
    base_url: &'static str,
}

impl NoaaPlugin {
    pub fn new() -> Arc<Self> {
        Arc::new(NoaaPlugin {
            base_url: "https://tgftp.nws.noaa.gov/data",
        })
    }
}

impl NoaaPlugin {
    async fn lookup_metar(self: Arc<Self>, ctx: Arc<Context>, station: String) -> Result<()> {
        let mut station = station;
        station.make_ascii_uppercase();

        let mut lines = lines_from_url(&format!(
            "{}/observations/metar/stations/{}.TXT",
            self.base_url, station
        ))
        .await?;

        // Only set the station if a request was successful.
        NoaaLocation::set_for_name(ctx.get_db(), ctx.sender()?, &station[..]).await?;

        let _ = lines.next();
        let line = lines
            .next()
            .transpose()?
            .ok_or_else(|| format_err!("No results"))?;

        ctx.mention_reply(&line[..]).await?;

        Ok(())
    }

    async fn lookup_taf(self: Arc<Self>, ctx: Arc<Context>, station: String) -> Result<()> {
        let mut station = station;
        station.make_ascii_uppercase();

        let mut lines = lines_from_url(&format!(
            "{}/forecasts/taf/stations/{}.TXT",
            self.base_url, station
        ))
        .await?;

        // Only set the station if a request was successful.
        NoaaLocation::set_for_name(ctx.get_db(), ctx.sender()?, &station[..]).await?;

        let _ = lines.next();
        for line in lines {
            if let Ok(line) = line {
                ctx.mention_reply(&line.trim()).await?;
            }
        }

        Ok(())
    }
}

#[derive(Debug)]
pub struct NoaaLocation {
    pub nick: String,
    pub station: String,
}

impl NoaaLocation {
    async fn get_by_name(conn: Arc<tokio_postgres::Client>, nick: &str) -> Result<Option<Self>> {
        Ok(conn
            .query_opt(
                "SELECT nick, station FROM noaa_location WHERE nick=$1;",
                &[&nick],
            )
            .await?
            .map(|row| NoaaLocation {
                nick: row.get(0),
                station: row.get(1),
            }))
    }

    async fn set_for_name(
        conn: Arc<tokio_postgres::Client>,
        nick: &str,
        station: &str,
    ) -> Result<()> {
        conn.execute(
            "INSERT INTO noaa_location (nick, station) VALUES ($1, $2)
ON CONFLICT (nick) DO UPDATE SET station=EXCLUDED.station;",
            &[&nick, &station],
        )
        .await?;

        Ok(())
    }
}

async fn lines_from_url(url: &str) -> Result<std::io::Lines<std::io::Cursor<String>>> {
    let start = Instant::now();
    let data = reqwest::get(url).await?.error_for_status()?.text().await?;

    debug!(
        "Got station information from \"{}\" in {}ms",
        url,
        start.elapsed().as_millis()
    );

    Ok(std::io::Cursor::new(data).lines())
}

async fn extract_station(ctx: &Arc<Context>, arg: Option<&str>) -> Result<Option<String>> {
    match arg {
        Some(station) => Ok(Some(station.to_string())),
        None => Ok(NoaaLocation::get_by_name(ctx.get_db(), ctx.sender()?)
            .await?
            .map(|station| station.station)),
    }
}

#[async_trait]
impl Plugin for Arc<NoaaPlugin> {
    async fn handle_message(&self, ctx: &Arc<Context>) -> Result<()> {
        match ctx.as_event() {
            Event::Command("metar", arg) => match extract_station(ctx, arg).await? {
                Some(station) => {
                    let plugin = (*self).clone();
                    let ctx = (*ctx).clone();

                    crate::spawn(plugin.lookup_metar(ctx, station));
                }
                None => {
                    ctx.mention_reply(&format!(
                        "Missing station argument. Usage: {}metar <station>",
                        ctx.command_prefix()
                    ))
                    .await?;
                }
            },

            Event::Command("taf", arg) => match extract_station(ctx, arg).await? {
                Some(station) => {
                    let plugin = (*self).clone();
                    let ctx = (*ctx).clone();

                    crate::spawn(plugin.lookup_taf(ctx, station));
                }
                None => {
                    ctx.mention_reply(&format!(
                        "Missing station argument. Usage: {}taf <station>",
                        ctx.command_prefix()
                    ))
                    .await?;
                }
            },

            _ => {}
        }

        Ok(())
    }
}
