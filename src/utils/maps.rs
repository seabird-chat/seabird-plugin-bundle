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
            .json()
            .await?;

        Ok(response
            .results
            .into_iter()
            .map(|result| Location {
                display_name: result.formatted_address,
                lat: result.geometry.location.lat,
                lng: result.geometry.location.lng,
            })
            .collect())
    }
}

pub struct Location {
    pub display_name: String,
    pub lat: f64,
    pub lng: f64,
}

#[derive(Deserialize)]
struct GeocodeResponse {
    results: Vec<GeocodeResponseResult>,
}

#[derive(Deserialize)]
struct GeocodeResponseResult {
    formatted_address: String,
    geometry: Geometry,
}

#[derive(Deserialize)]
struct Geometry {
    location: GeometryLocation,
}

#[derive(Deserialize)]
struct GeometryLocation {
    lat: f64,
    lng: f64,
}
