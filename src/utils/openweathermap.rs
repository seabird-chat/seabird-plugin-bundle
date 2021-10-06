use std::convert::TryInto;

use serde::Deserialize;

use crate::prelude::*;

const BASE_URL: &str = "https://api.openweathermap.org/data/2.5";

pub struct Client {
    api_key: String,
    inner: reqwest::Client,
}

impl Client {
    pub fn new(api_key: String) -> Self {
        Client {
            api_key,
            inner: reqwest::Client::new(),
        }
    }

    pub async fn forecast(&self, lat: f64, lng: f64) -> Result<Forecast> {
        let url = format!("{}/onecall", BASE_URL);

        let query = vec![
            ("appid", self.api_key.clone()),
            ("lat", format!("{:.16}", lat)),
            ("lon", format!("{:.16}", lng)),
            ("units", "imperial".to_string()),
        ];

        let res: OneCallResponse = self
            .inner
            .get(&url)
            .query(&query)
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;

        res.try_into()
    }

    pub async fn weather(&self, lat: f64, lng: f64) -> Result<CurrentWeather> {
        let url = format!("{}/onecall", BASE_URL);

        let query = vec![
            ("appid", self.api_key.clone()),
            ("lat", format!("{:.16}", lat)),
            ("lon", format!("{:.16}", lng)),
            ("units", "imperial".to_string()),
        ];

        let res: OneCallResponse = self
            .inner
            .get(&url)
            .query(&query)
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;

        res.try_into()
    }
}

impl TryInto<Forecast> for OneCallResponse {
    type Error = anyhow::Error;

    fn try_into(self) -> Result<Forecast> {
        self.daily
            .into_iter()
            .map(|daily| -> Result<Weather> {
                let summary = if daily.weather.is_empty() {
                    "unknown".to_string()
                } else {
                    daily.weather.into_iter().map(|w| w.description).join(", ")
                };

                Ok(Weather {
                    time: time::OffsetDateTime::from_unix_timestamp(daily.dt)?,
                    temperature_high: daily.temp.max,
                    temperature_low: daily.temp.min,
                    humidity: daily.humidity,
                    summary,
                })
            })
            .collect()
    }
}

impl TryInto<CurrentWeather> for OneCallResponse {
    type Error = anyhow::Error;

    fn try_into(self) -> Result<CurrentWeather> {
        let summary = if self.current.weather.is_empty() {
            "unknown".to_string()
        } else {
            self.current
                .weather
                .into_iter()
                .map(|w| w.description)
                .join(", ")
        };

        Ok(CurrentWeather {
            time: time::OffsetDateTime::from_unix_timestamp(self.current.dt)?,
            temperature: self.current.temp,
            temperature_feels_like: self.current.feels_like,
            temperature_high: self.daily[0].temp.max,
            temperature_low: self.daily[0].temp.min,
            humidity: self.current.humidity,
            summary,
        })
    }
}

pub struct Weather {
    pub time: time::OffsetDateTime,
    pub temperature_high: f64,
    pub temperature_low: f64,
    pub humidity: i32,
    pub summary: String,
}

pub type Forecast = Vec<Weather>;

pub struct CurrentWeather {
    pub time: time::OffsetDateTime,
    pub temperature: f64,
    pub temperature_feels_like: f64,
    pub temperature_high: f64,
    pub temperature_low: f64,
    pub humidity: i32,
    pub summary: String,
}

#[derive(Deserialize)]
struct OneCallResponse {
    current: CurrentResponse,
    daily: Vec<DailyResponse>,
}

#[derive(Deserialize)]
struct CurrentResponse {
    dt: i64,
    temp: f64,
    feels_like: f64,
    humidity: i32,
    weather: Vec<WeatherStatus>,
}

#[derive(Deserialize)]
struct DailyResponse {
    dt: i64,
    temp: DailyTemperatureResponse,
    weather: Vec<WeatherStatus>,
    humidity: i32,
}

#[derive(Deserialize)]
struct DailyTemperatureResponse {
    min: f64,
    max: f64,
}

#[derive(Deserialize)]
struct WeatherStatus {
    description: String,
}
