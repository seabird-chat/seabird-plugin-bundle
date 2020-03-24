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
mod utils;

fn spawn<T>(task: T) -> tokio::task::JoinHandle<()>
where
    T: std::future::Future<Output = Result<(), anyhow::Error>> + Send + 'static,
{
    tokio::task::spawn(async move {
        if let Err(e) = task.await {
            error!("Background task failed to execute: {}", e);
        };
    })
}

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
        dotenv::var("SEABIRD_PASS").ok(),
        dotenv::var("DATABASE_URL")?,
        dotenv::var("SEABIRD_COMMAND_PREFIX").unwrap_or_else(|_| "!".to_string()),
        dotenv::var("DARKSKY_API_KEY")?,
        dotenv::var("GOOGLE_MAPS_API_KEY")?,
    );

    client::run(config).await
}
