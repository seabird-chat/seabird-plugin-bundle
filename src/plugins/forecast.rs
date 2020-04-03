use crate::utils::{darksky, maps};

use crate::prelude::*;

pub struct ForecastPlugin {
    darksky: darksky::Client,
    maps: maps::Client,
}

impl ForecastPlugin {
    pub fn new(darksky_api_key: String, maps_api_key: String) -> Arc<Self> {
        Arc::new(ForecastPlugin {
            darksky: darksky::Client::new(darksky_api_key),
            maps: maps::Client::new(maps_api_key),
        })
    }
}

impl ForecastPlugin {
    async fn lookup_weather(
        self: Arc<Self>,
        ctx: Arc<Context>,
        location: ForecastLocation,
    ) -> Result<()> {
        let res = self.darksky.weather(location.lat, location.lng).await?;

        // Only set the station if a request was successful.
        ForecastLocation::set_for_name(
            ctx.get_db(),
            ctx.sender()?,
            &location.address[..],
            location.lat,
            location.lng,
        )
        .await?;

        let temp_unit = match &res.flags.units[..] {
            "auto" => "°",
            "us" => "°F",
            _ => "°C",
        };

        ctx.mention_reply(&format!(
            "{}. Currently {:.1}{}. High {:.2}{}, Low {:.2}{}, Humidity {:.0}%. {}",
            location.address,
            res.data.temperature,
            temp_unit,
            res.data.temperature_high,
            temp_unit,
            res.data.temperature_low,
            temp_unit,
            res.data.humidity,
            res.data.summary,
        ))
        .await?;

        Ok(())
    }

    async fn lookup_forecast(
        self: Arc<Self>,
        ctx: Arc<Context>,
        location: ForecastLocation,
    ) -> Result<()> {
        let res = self.darksky.forecast(location.lat, location.lng).await?;

        // Only set the station if a request was successful.
        ForecastLocation::set_for_name(
            ctx.get_db(),
            ctx.sender()?,
            &location.address[..],
            location.lat,
            location.lng,
        )
        .await?;

        let temp_unit = match &res.flags.units[..] {
            "auto" => "°",
            "us" => "°F",
            _ => "°C",
        };

        ctx.mention_reply(&format!("3 day forecast for {}.", location.address))
            .await?;

        for day in res.data.into_iter().skip(1).take(3) {
            let weekday = day.time.format("%A");

            ctx.mention_reply(&format!(
                "{}: High {:.2}{}, Low {:.2}{}, Humidity {:.0}%. {}",
                weekday,
                day.temperature_high,
                temp_unit,
                day.temperature_low,
                temp_unit,
                day.humidity,
                day.summary,
            ))
            .await?;
        }

        Ok(())
    }

    async fn extract_location(
        self: &Arc<Self>,
        ctx: &Arc<Context>,
        arg: Option<&str>,
    ) -> Result<Option<ForecastLocation>> {
        match arg {
            Some(address) => {
                let results = self.maps.forward(address).await?;

                let mut iter = results.into_iter();
                let loc = match (iter.next(), iter.next()) {
                    (None, _) => Err(format_err!("No location results found")),
                    (Some(loc), None) => Ok(loc),
                    (Some(_), Some(_)) => Err(format_err!("More than one location result")),
                }?;

                Ok(Some(ForecastLocation::new(
                    ctx.sender()?.to_string(),
                    address.to_string(),
                    loc.lat,
                    loc.lng,
                )))
            }
            None => Ok(ForecastLocation::get_by_name(ctx.get_db(), ctx.sender()?).await?),
        }
    }
}

#[derive(Debug)]
pub struct ForecastLocation {
    pub nick: String,
    pub address: String,
    pub lat: f64,
    pub lng: f64,
}

impl ForecastLocation {
    fn new(nick: String, address: String, lat: f64, lng: f64) -> Self {
        ForecastLocation {
            nick,
            address,
            lat,
            lng,
        }
    }

    async fn get_by_name(conn: Arc<tokio_postgres::Client>, nick: &str) -> Result<Option<Self>> {
        Ok(conn
            .query_opt(
                "SELECT nick, address, lat, lng FROM forecast_location WHERE nick=$1;",
                &[&nick],
            )
            .await?
            .map(|row| ForecastLocation {
                nick: row.get(0),
                address: row.get(1),
                lat: row.get(2),
                lng: row.get(3),
            }))
    }

    async fn set_for_name(
        conn: Arc<tokio_postgres::Client>,
        nick: &str,
        address: &str,
        lat: f64,
        lng: f64,
    ) -> Result<()> {
        conn.execute(
            "INSERT INTO forecast_location (nick, address, lat, lng) VALUES ($1, $2, $3, $4)
ON CONFLICT (nick) DO
UPDATE SET address=EXCLUDED.address, lat=EXCLUDED.lat, lng=EXCLUDED.lng;",
            &[&nick, &address, &lat, &lng],
        )
        .await?;

        Ok(())
    }
}

#[async_trait]
impl Plugin for Arc<ForecastPlugin> {
    fn new_from_env() -> Result<Self> {
        Ok(ForecastPlugin::new(
            dotenv::var("DARKSKY_API_KEY").map_err(|_| {
                anyhow::format_err!(
                    "Missing $DARKSKY_API_KEY. Required by the \"forecast\" plugin."
                )
            })?,
            dotenv::var("GOOGLE_MAPS_API_KEY").map_err(|_| {
                anyhow::format_err!("Missing $MAPS_API_KEY. Required by the \"forecast\" plugin.")
            })?,
        ))
    }

    async fn handle_message(&self, ctx: &Arc<Context>) -> Result<()> {
        match ctx.as_event() {
            Event::Command("weather", arg) => match self.extract_location(ctx, arg).await? {
                Some(location) => {
                    let plugin = (*self).clone();
                    let ctx = (*ctx).clone();

                    crate::spawn(plugin.lookup_weather(ctx, location));
                }
                None => {
                    ctx.mention_reply(&format!(
                        "Missing location argument. Usage: {}weather <station>",
                        ctx.command_prefix()
                    ))
                    .await?;
                }
            },

            Event::Command("forecast", arg) => match self.extract_location(ctx, arg).await? {
                Some(location) => {
                    let plugin = (*self).clone();
                    let ctx = (*ctx).clone();

                    crate::spawn(plugin.lookup_forecast(ctx, location));
                }
                None => {
                    ctx.mention_reply(&format!(
                        "Missing location argument. Usage: {}forecast <station>",
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
