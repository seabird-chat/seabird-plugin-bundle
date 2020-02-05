#[cfg(feature = "db")]
#[macro_use]
extern crate diesel;

#[cfg(feature = "db")]
#[macro_use]
extern crate diesel_migrations;

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
}

impl Config {
    fn new(
        host: String,
        nick: String,
        user: Option<String>,
        name: Option<String>,
        #[cfg(feature = "db")] db_url: String,
    ) -> Self {
        Config {
            host,
            nick,
            user,
            name,

            #[cfg(feature = "db")]
            db_url,
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
        }
    }
}

#[tokio::main]
async fn main() -> error::Result<()> {
    // We ignore failures here because we want to fall back to loading from the
    // environment.
    if let Ok(path) = dotenv::dotenv() {
        println!("Loading env from {:?}", path);
    }

    // Load our config from command line arguments
    let config = Config::new(
        dotenv::var("IRC_URL")?,
        dotenv::var("SEABIRD_NICK")?,
        dotenv::var("SEABIRD_USER").ok(),
        dotenv::var("SEABIRD_NAME").ok(),
        #[cfg(feature = "db")]
        dotenv::var("DATABASE_URL")?,
    );

    let client = client::Client::new(config.into())?;
    client.run().await
}
