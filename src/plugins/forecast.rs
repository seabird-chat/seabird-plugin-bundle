use crate::utils::{darksky, maps};

use crate::prelude::*;

pub struct ForecastPlugin {
    darksky: darksky::Client,
    maps: maps::Client,
}

impl ForecastPlugin {
    pub fn new(darksky_api_key: String, maps_api_key: String) -> Self {
        ForecastPlugin {
            darksky: darksky::Client::new(darksky_api_key),
            maps: maps::Client::new(maps_api_key),
        }
    }
}

impl ForecastPlugin {
    async fn handle_weather(&self, ctx: &Context, arg: Option<&str>) -> Result<()> {
        match self.extract_location(ctx, arg).await? {
            LocationStatus::SingleLocation(location) => {
                self.lookup_weather(ctx, location).await?;
            }
            LocationStatus::MultipleLocations(locations) => {
                ctx.mention_reply(&format!(
                    "Multiple possible locations. {}.",
                    locations
                        .into_iter()
                        .take(5)
                        .map(|loc| loc.address)
                        .join(", "),
                ))
                .await?;
            }
            LocationStatus::NoLocations => {
                ctx.mention_reply(&format!(
                    "Missing location argument or unknown location. Usage: {}weather <station>",
                    ctx.command_prefix()
                ))
                .await?;
            }
        }

        Ok(())
    }

    async fn handle_forecast(&self, ctx: &Context, arg: Option<&str>) -> Result<()> {
        match self.extract_location(ctx, arg).await? {
            LocationStatus::SingleLocation(location) => {
                self.lookup_forecast(ctx, location).await?;
            }
            LocationStatus::MultipleLocations(locations) => {
                ctx.mention_reply(&format!(
                    "Multiple possible locations. {}.",
                    locations
                        .into_iter()
                        .take(5)
                        .map(|loc| loc.address)
                        .join(", "),
                ))
                .await?;
            }
            LocationStatus::NoLocations => {
                ctx.mention_reply(&format!(
                    "Missing location argument or unknown location. Usage: {}forecast <station>",
                    ctx.command_prefix()
                ))
                .await?;
            }
        }

        Ok(())
    }

    async fn lookup_weather(&self, ctx: &Context, location: ForecastLocation) -> Result<()> {
        let res = self.darksky.weather(location.lat, location.lng).await?;

        // Only set the station if a request was successful.
        ForecastLocation::set_for_name(
            ctx.get_db(),
            ctx.sender()
                .ok_or_else(|| format_err!("couldn't set location: event missing sender"))?,
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

    async fn lookup_forecast(&self, ctx: &Context, location: ForecastLocation) -> Result<()> {
        let res = self.darksky.forecast(location.lat, location.lng).await?;

        // Only set the station if a request was successful.
        ForecastLocation::set_for_name(
            ctx.get_db(),
            ctx.sender()
                .ok_or_else(|| format_err!("couldn't set location: event missing sender"))?,
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

    async fn extract_location(&self, ctx: &Context, arg: Option<&str>) -> Result<LocationStatus> {
        let sender = ctx
            .sender()
            .ok_or_else(|| format_err!("couldn't extract location: event missing sender"))?;

        match arg {
            Some(address) => {
                let results = self.maps.forward(address).await?;
                Ok(match results.len() {
                    0 => LocationStatus::NoLocations,
                    1 => LocationStatus::SingleLocation(ForecastLocation::new(
                        sender.to_string(),
                        address.to_string(),
                        results[0].lat,
                        results[0].lng,
                    )),
                    _ => LocationStatus::MultipleLocations(
                        results
                            .into_iter()
                            .map(|loc| {
                                ForecastLocation::new(
                                    sender.to_string(),
                                    loc.display_name,
                                    loc.lat,
                                    loc.lng,
                                )
                            })
                            .collect(),
                    ),
                })
            }
            None => Ok(ForecastLocation::get_by_name(ctx.get_db(), sender)
                .await?
                .map_or(LocationStatus::NoLocations, |loc| {
                    LocationStatus::SingleLocation(loc)
                })),
        }
    }
}

#[derive(Debug)]
enum LocationStatus {
    NoLocations,
    MultipleLocations(Vec<ForecastLocation>),
    SingleLocation(ForecastLocation),
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
impl Plugin for ForecastPlugin {
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

    async fn run(self, mut stream: Receiver<Arc<Context>>) -> Result<()> {
        while let Some(ctx) = stream.next().await {
            let res = match ctx.as_event() {
                Event::Command("weather", arg) => self.handle_weather(&ctx, arg).await,
                Event::Command("forecast", arg) => self.handle_forecast(&ctx, arg).await,
                _ => Ok(()),
            };

            crate::check_err(&ctx, res).await;
        }

        Err(format_err!("forecast plugin exited early"))
    }
}
