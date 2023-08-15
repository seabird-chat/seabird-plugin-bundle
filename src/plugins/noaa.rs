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
            &ctx.get_db(),
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
            &ctx.get_db(),
            ctx.sender()
                .ok_or_else(|| format_err!("couldn't set location: event missing sender"))?,
            &station[..],
        )
        .await?;

        let _ = lines.next();
        for line in lines.flatten() {
            ctx.mention_reply(line.trim()).await?;
        }

        Ok(())
    }
}

#[derive(sqlx::FromRow, Debug)]
pub struct NoaaLocation {
    pub nick: String,
    pub station: String,
}

impl NoaaLocation {
    async fn get_by_name(conn: &sqlx::PgPool, nick: &str) -> Result<Option<Self>> {
        Ok(sqlx::query_as!(
            NoaaLocation,
            "SELECT nick, station FROM noaa_location WHERE nick=$1;",
            nick
        )
        .fetch_optional(conn)
        .await?)
    }

    async fn set_for_name(conn: &sqlx::PgPool, nick: &str, station: &str) -> Result<()> {
        sqlx::query!(
            "INSERT INTO noaa_location (nick, station) VALUES ($1, $2)
ON CONFLICT (nick) DO UPDATE SET station=EXCLUDED.station;",
            nick,
            station
        )
        .execute(conn)
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
            &ctx.get_db(),
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
                ctx.mention_reply("Missing station argument.").await?;
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
                ctx.mention_reply("Missing station argument.").await?;
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

    fn command_metadata(&self) -> Vec<CommandMetadata> {
        vec![
            CommandMetadata {
                name: "metar".to_string(),
                short_help: "usage: metar [station]. fetches current METAR for given station.".to_string(),
                full_help: "fetches current METAR for given station. if no station provided, most recent station is used.".to_string(),
            },
            CommandMetadata {
                name: "taf".to_string(),
                short_help: "usage: taf [station]. fetches current TAF for given station.".to_string(),
                full_help: "fetches current TAF for given station. if no station provided, most recent station is used.".to_string(),
            },
        ]
    }

    async fn run(self, bot: Arc<Client>) -> Result<()> {
        let mut stream = bot.subscribe();

        while let Ok(ctx) = stream.recv().await {
            let res = match ctx.as_event() {
                Ok(Event::Command("metar", arg)) => self.handle_metar(&ctx, arg).await,
                Ok(Event::Command("taf", arg)) => self.handle_taf(&ctx, arg).await,
                _ => Ok(()),
            };

            crate::check_err(&ctx, res).await;
        }

        Err(format_err!("noaa plugin lagged"))
    }
}
