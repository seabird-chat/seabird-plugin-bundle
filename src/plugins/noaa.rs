use std::io::BufRead;
use std::time::Instant;

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

impl NoaaPlugin {
    async fn lookup_metar(&self, ctx: &Context, station: String) -> Result<()> {
        let mut station = station;
        station.make_ascii_uppercase();

        let mut lines = lines_from_url(&format!(
            "{}/observations/metar/stations/{}.TXT",
            self.base_url, station
        ))
        .await?;

        // Only set the station if a request was successful.
        NoaaLocation::set_for_name(
            ctx.get_db(),
            ctx.sender()
                .ok_or_else(|| format_err!("couldn't set location: event missing sender"))?,
            &station[..],
        )
        .await?;

        let _ = lines.next();
        let line = lines
            .next()
            .transpose()?
            .ok_or_else(|| format_err!("No results"))?;

        ctx.mention_reply(&line[..]).await?;

        Ok(())
    }

    async fn lookup_taf(&self, ctx: &Context, station: String) -> Result<()> {
        let mut station = station;
        station.make_ascii_uppercase();

        let mut lines = lines_from_url(&format!(
            "{}/forecasts/taf/stations/{}.TXT",
            self.base_url, station
        ))
        .await?;

        // Only set the station if a request was successful.
        NoaaLocation::set_for_name(
            ctx.get_db(),
            ctx.sender()
                .ok_or_else(|| format_err!("couldn't set location: event missing sender"))?,
            &station[..],
        )
        .await?;

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

async fn extract_station(ctx: &Context, arg: Option<&str>) -> Result<Option<String>> {
    match arg {
        Some(station) => Ok(Some(station.to_string())),
        None => Ok(NoaaLocation::get_by_name(
            ctx.get_db(),
            ctx.sender()
                .ok_or_else(|| format_err!("couldn't look up station: event missing sender"))?,
        )
        .await?
        .map(|station| station.station)),
    }
}

impl NoaaPlugin {
    async fn handle_metar(&self, ctx: &Context, arg: Option<&str>) -> Result<()> {
        match extract_station(ctx, arg).await? {
            Some(station) => {
                self.lookup_metar(ctx, station).await?;
            }
            None => {
                ctx.mention_reply(&format!(
                    "Missing station argument. Usage: {}metar <station>",
                    ctx.command_prefix()
                ))
                .await?;
            }
        }

        Ok(())
    }

    async fn handle_taf(&self, ctx: &Context, arg: Option<&str>) -> Result<()> {
        match extract_station(ctx, arg).await? {
            Some(station) => {
                self.lookup_taf(ctx, station).await?;
            }
            None => {
                ctx.mention_reply(&format!(
                    "Missing station argument. Usage: {}taf <station>",
                    ctx.command_prefix()
                ))
                .await?;
            }
        }

        Ok(())
    }
}

#[async_trait]
impl Plugin for NoaaPlugin {
    fn new_from_env() -> Result<Self> {
        Ok(NoaaPlugin::new())
    }

    async fn run(self, _bot: Arc<Client>, mut stream: Receiver<Arc<Context>>) -> Result<()> {
        while let Some(ctx) = stream.next().await {
            let res = match ctx.as_event() {
                Event::Command("metar", arg) => self.handle_metar(&ctx, arg).await,
                Event::Command("taf", arg) => self.handle_taf(&ctx, arg).await,
                _ => Ok(()),
            };

            crate::check_err(&ctx, res).await;
        }

        Err(format_err!("noaa plugin exited early"))
    }
}
