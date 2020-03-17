use std::convert::{TryFrom, TryInto};

use serde::Deserialize;

use crate::prelude::*;

const BASE_URL: &str = "https://api.darksky.net/forecast";

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
        let url = format!("{}/{}/{:.16},{:.16}", BASE_URL, self.api_key, lat, lng);

        let res: DarkskyResponse = self
            .inner
            .get(&url)
            .query(&[
                ("units", "auto"),
                ("exclude", "currently,hourly,minutely,alerts"),
            ])
            .send()
            .await?
            .json()
            .await?;

        res.try_into()
    }

    pub async fn weather(&self, lat: f64, lng: f64) -> Result<CurrentWeather> {
        let url = format!("{}/{}/{:.16},{:.16}", BASE_URL, self.api_key, lat, lng);

        let res: DarkskyResponse = self
            .inner
            .get(&url)
            .query(&[("units", "auto"), ("exclude", "hourly,minutely,alerts")])
            .send()
            .await?
            .json()
            .await?;

        res.try_into()
    }
}

pub struct Forecast {
    pub data: Vec<Weather>,
    pub flags: Flags,
}

impl TryFrom<DarkskyResponse> for Forecast {
    type Error = anyhow::Error;

    fn try_from(resp: DarkskyResponse) -> Result<Forecast> {
        Ok(Forecast {
            data: resp
                .daily
                .ok_or_else(|| anyhow::anyhow!("Missing daily"))?
                .data
                .into_iter()
                .map(|data_point| data_point.try_into())
                .collect::<Result<Vec<Weather>>>()?,
            flags: resp.flags.ok_or_else(|| anyhow::anyhow!("Missing flags"))?,
        })
    }
}

pub struct CurrentWeather {
    pub data: CurrentWeatherData,
    pub flags: Flags,
}

pub struct CurrentWeatherData {
    pub time: time::OffsetDateTime,
    pub temperature: f64,
    pub temperature_high: f64,
    pub temperature_low: f64,
    pub humidity: f64,
    pub summary: String,
}

impl TryFrom<DarkskyResponse> for CurrentWeather {
    type Error = anyhow::Error;

    fn try_from(resp: DarkskyResponse) -> Result<CurrentWeather> {
        let current = resp
            .currently
            .ok_or_else(|| anyhow::anyhow!("Missing current"))?;

        let today = resp.daily.ok_or_else(|| anyhow::anyhow!("Missing daily"))?;

        let today = today
            .data
            .into_iter()
            .next()
            .ok_or_else(|| anyhow::anyhow!("Missing day 0"))?;

        Ok(CurrentWeather {
            data: CurrentWeatherData {
                time: time::OffsetDateTime::from_unix_timestamp(today.time),
                temperature: current
                    .temperature
                    .ok_or_else(|| anyhow::anyhow!("Missing temperature"))?,
                temperature_high: today
                    .temperature_high
                    .ok_or_else(|| anyhow::anyhow!("Missing temperature_high"))?,
                temperature_low: today
                    .temperature_low
                    .ok_or_else(|| anyhow::anyhow!("Missing temperature_low"))?,
                humidity: today
                    .humidity
                    .ok_or_else(|| anyhow::anyhow!("Missing humidity"))?
                    * 100.0,
                summary: today
                    .summary
                    .ok_or_else(|| anyhow::anyhow!("Missing summary"))?,
            },
            flags: resp.flags.ok_or_else(|| anyhow::anyhow!("Missing flags"))?,
        })
    }
}

pub struct Weather {
    pub time: time::OffsetDateTime,
    pub temperature_high: f64,
    pub temperature_low: f64,
    pub humidity: f64,
    pub summary: String,
}

impl TryFrom<DataPoint> for Weather {
    type Error = anyhow::Error;

    fn try_from(data_point: DataPoint) -> Result<Weather> {
        Ok(Weather {
            time: time::OffsetDateTime::from_unix_timestamp(data_point.time),
            temperature_high: data_point
                .temperature_high
                .ok_or_else(|| anyhow::anyhow!("Missing temperature_high"))?,
            temperature_low: data_point
                .temperature_low
                .ok_or_else(|| anyhow::anyhow!("Missing temperature_low"))?,
            humidity: data_point
                .humidity
                .ok_or_else(|| anyhow::anyhow!("Missing humidity"))?
                * 100.0,
            summary: data_point
                .summary
                .ok_or_else(|| anyhow::anyhow!("Missing summary"))?,
        })
    }
}

#[derive(Deserialize)]
struct DarkskyResponse {
    currently: Option<DataPoint>,
    daily: Option<DataBlock>,
    flags: Option<Flags>,
}

#[derive(Deserialize)]
struct DataBlock {
    data: Vec<DataPoint>,
}

#[derive(Deserialize)]
struct DataPoint {
    time: i64,
    temperature: Option<f64>,
    #[serde(rename = "temperatureMax")]
    temperature_high: Option<f64>,
    #[serde(rename = "temperatureMin")]
    temperature_low: Option<f64>,
    humidity: Option<f64>,
    summary: Option<String>,
}

#[derive(Deserialize)]
pub struct Flags {
    pub units: String,
}
