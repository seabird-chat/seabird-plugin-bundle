use crate::utils::to_sentence_case;

use crate::utils::{maps, openweathermap};

use crate::prelude::*;

pub struct ForecastPlugin {
    darksky: openweathermap::Client,
    maps: maps::Client,
}

impl ForecastPlugin {
    pub fn new(openweathermap_api_key: String, maps_api_key: String) -> Self {
        ForecastPlugin {
            darksky: openweathermap::Client::new(openweathermap_api_key),
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

        ctx.mention_reply(&format!(
            "{}. Currently {:.1}°F, Feels Like {:.1}°F. High {:.1}°F, Low {:.1}°F. Humidity {}%. {}.",
            location.address,
            res.temperature,
            res.temperature_feels_like,
            res.temperature_high,
            res.temperature_low,
            res.humidity,
            to_sentence_case(&res.summary),
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

        ctx.mention_reply(&format!("3 day forecast for {}.", location.address))
            .await?;

        for day in res.into_iter().skip(1).take(3) {
            let weekday = day.time.format("%A");

            ctx.mention_reply(&format!(
                "{}: High {:.2}°F, Low {:.2}°F, Humidity {:.0}%. {}.",
                weekday,
                day.temperature_high,
                day.temperature_low,
                day.humidity,
                to_sentence_case(&day.summary),
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
            dotenv::var("OPENWEATHERMAP_API_KEY").map_err(|_| {
                anyhow::format_err!(
                    "Missing $OPENWEATHERMAP_API_KEY. Required by the \"forecast\" plugin."
                )
            })?,
            dotenv::var("GOOGLE_MAPS_API_KEY").map_err(|_| {
                anyhow::format_err!("Missing $MAPS_API_KEY. Required by the \"forecast\" plugin.")
            })?,
        ))
    }

    async fn run(self, _bot: Arc<Client>, mut stream: Receiver<Arc<Context>>) -> Result<()> {
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
