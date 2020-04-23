use crate::prelude::*;

// API clients
pub mod maps;
pub mod openweathermap;

pub mod hex_slice;
pub use hex_slice::HexSlice;

pub fn to_sentence_case(s: &str) -> String {
    let mut graphemes = s.graphemes(true);
    let mut cap = String::with_capacity(s.len());
    cap.push_str(&graphemes.next().unwrap().to_uppercase());
    cap.push_str(graphemes.as_str());
    cap
}

pub struct HostPort {
    pub host: String,
    pub port: u16,
}

pub fn split_host_port(hostport: &str, default_port: &str) -> Result<HostPort> {
    let parts: Vec<&str> = hostport.splitn(2, ':').collect();
    let host = parts
        .get(0)
        .map(|s| (*s).to_string())
        .ok_or_else(|| format_err!("missing hostport string (this should be impossible)"))?;
    let port = parts
        .get(1)
        .unwrap_or_else(|| &default_port)
        .parse()
        .with_context(|| "hostport string has invalid port specifier")?;

    Ok(HostPort { host, port })
}
