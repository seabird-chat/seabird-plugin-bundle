use serde::Deserialize;

use crate::prelude::*;

const BASE_URL: &str = "https://maps.googleapis.com/maps/api";

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

    pub async fn forward(&self, loc: &str) -> Result<Vec<Location>> {
        let url = format!("{}/geocode/json", BASE_URL);

        let response: GeocodeResponse = self
            .inner
            .get(&url)
            .query(&[("key", &self.api_key[..]), ("address", loc)])
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;

        match response.status.as_ref() {
            "OK" | "ZERO_RESULTS" => Ok(response
                .results
                .into_iter()
                .map(|result| Location {
                    display_name: result.formatted_address,
                    lat: result.geometry.location.lat,
                    lng: result.geometry.location.lng,
                })
                .collect()),
            status => Err(format_err!("unexpected response status: {}", status)),
        }
    }
}

#[derive(Debug)]
pub struct Location {
    pub display_name: String,
    pub lat: f64,
    pub lng: f64,
}

#[derive(Deserialize, Debug)]
struct GeocodeResponse {
    results: Vec<GeocodeResponseResult>,
    status: String,
}

#[derive(Deserialize, Debug)]
struct GeocodeResponseResult {
    formatted_address: String,
    geometry: Geometry,
}

#[derive(Deserialize, Debug)]
struct Geometry {
    location: GeometryLocation,
}

#[derive(Deserialize, Debug)]
struct GeometryLocation {
    lat: f64,
    lng: f64,
}
