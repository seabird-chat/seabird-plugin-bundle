#[macro_use]
extern crate log;

mod client;
mod codec;
mod error;
mod event;
mod migrations;
mod plugin;
mod plugins;
mod prelude;

#[tokio::main]
async fn main() -> error::Result<()> {
    pretty_env_logger::init_timed();

    // We ignore failures here because we want to fall back to loading from the
    // environment.
    if let Ok(path) = dotenv::dotenv() {
        info!("Loaded env from {:?}", path);
    }

    // Load our config from command line arguments
    let config = client::ClientConfig::new(
        dotenv::var("SEABIRD_HOST")?,
        dotenv::var("SEABIRD_NICK")?,
        dotenv::var("SEABIRD_USER").ok(),
        dotenv::var("SEABIRD_NAME").ok(),
        dotenv::var("DATABASE_URL")?,
        dotenv::var("SEABIRD_COMMAND_PREFIX").unwrap_or("!".to_string()),
    );

    client::run(config.into()).await
}
