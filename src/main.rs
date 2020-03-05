use tracing::info;
use tracing_subscriber::{filter::EnvFilter, FmtSubscriber};

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
    // We need to try loading the dotenv up here so the log level can be pulled
    // from here.
    let dotenv_result = dotenv::dotenv();

    let filter =
        EnvFilter::new(dotenv::var("SEABIRD_LOG_FILTER").unwrap_or_else(|_| "".to_string()))
            .add_directive(
                format!(
                    "seabird={}",
                    dotenv::var("SEABIRD_LOG_LEVEL").unwrap_or_else(|_| "trace".to_string())
                )
                .parse()?,
            );

    FmtSubscriber::builder()
        .with_env_filter(filter)
        .with_ansi(atty::is(atty::Stream::Stdout))
        .init();

    // We ignore failures here because we want to fall back to loading from the
    // environment.
    if let Ok(path) = dotenv_result {
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
        dotenv::var("INCLUDE_MESSAGE_ID_IN_LOGS")
            .unwrap_or("true".to_string())
            .parse::<bool>()?,
    );

    client::run(config.into()).await
}
