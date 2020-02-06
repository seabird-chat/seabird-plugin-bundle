#[cfg(feature = "db")]
#[macro_use]
extern crate diesel;

#[cfg(feature = "db")]
#[macro_use]
extern crate diesel_migrations;

use tracing::{info, Level};
use tracing_subscriber::FmtSubscriber;

mod client;
mod codec;
mod error;
mod event;
mod plugin;
mod plugins;
mod prelude;

#[cfg(feature = "db")]
mod schema;

struct Config {
    host: String,
    nick: String,
    user: Option<String>,
    name: Option<String>,

    #[cfg(feature = "db")]
    db_url: String,

    include_message_id_in_logs: String,
}

impl Config {
    fn new(
        host: String,
        nick: String,
        user: Option<String>,
        name: Option<String>,
        #[cfg(feature = "db")] db_url: String,
        include_message_id_in_logs: String,
    ) -> Self {
        Config {
            host,
            nick,
            user,
            name,

            #[cfg(feature = "db")]
            db_url,

            include_message_id_in_logs,
        }
    }
}

impl Into<client::ClientConfig> for Config {
    fn into(self) -> client::ClientConfig {
        client::ClientConfig {
            target: self.host.to_string(),
            nick: self.nick.to_string(),
            user: self.user.as_ref().unwrap_or(&self.nick).to_string(),
            name: self
                .name
                .as_ref()
                .or_else(|| self.user.as_ref())
                .unwrap_or(&self.nick)
                .to_string(),
            #[cfg(feature = "db")]
            db_url: self.db_url,

            include_message_id_in_logs: self.include_message_id_in_logs == "true",
        }
    }
}

#[tokio::main]
async fn main() -> error::Result<()> {
    // We need to try loading the dotenv up here so the log level can be pulled
    // from here.
    let dotenv_result = dotenv::dotenv();

    let mut subscriber = FmtSubscriber::builder().with_max_level(
        dotenv::var("SEABIRD_LOG_LEVEL")
            .unwrap_or_else(|_| "trace".to_string())
            .parse::<Level>()?,
    );

    // If we have a tty in stdout, make sure to enable fancy colors.
    subscriber = subscriber.with_ansi(atty::is(atty::Stream::Stdout));

    // Install this subscriber as the default.
    subscriber.init();

    // We ignore failures here because we want to fall back to loading from the
    // environment.
    if let Ok(path) = dotenv_result {
        info!("Loaded env from {:?}", path);
    }

    // Load our config from command line arguments
    let config = Config::new(
        dotenv::var("SEABIRD_HOST")?,
        dotenv::var("SEABIRD_NICK")?,
        dotenv::var("SEABIRD_USER").ok(),
        dotenv::var("SEABIRD_NAME").ok(),
        #[cfg(feature = "db")]
        dotenv::var("DATABASE_URL")?,
        dotenv::var("INCLUDE_MESSAGE_ID_IN_LOGS").unwrap_or("true".to_string()),
    );

    let client = client::Client::new(config.into())?;
    client.run().await
}
