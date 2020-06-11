#![warn(missing_debug_implementations, rust_2018_idioms)]

use anyhow::Context;

#[macro_use]
extern crate log;

mod client;
mod error;
mod migrations;
mod plugin;
mod plugins;
mod prelude;
pub(crate) mod utils;

pub mod proto {
    pub mod common {
        tonic::include_proto!("common");
    }
    pub mod seabird {
        tonic::include_proto!("seabird");
    }

    pub use self::common::*;
    pub use self::seabird::*;
}

async fn check_err<T>(ctx: &client::Context, res: error::Result<T>) {
    if let Err(err) = res {
        error!("unexpected error: {}", err);

        let inner = ctx
            .mention_reply(&format!("unexpected error: {}", err))
            .await;
        if let Err(inner) = inner {
            error!("unexpected error ({}) while handling: {}", inner, err);
        }
    }
}

#[tokio::main]
async fn main() -> error::Result<()> {
    // Try to load dotenv before loading the logger or trying to set defaults.
    let env_res = dotenv::dotenv();

    // There's a little bit of an oddity here, since we want to set it if it
    // hasn't already been set, but we want this done before the logger is loaded.
    if std::env::var("RUST_LOG").is_err() {
        std::env::set_var("RUST_LOG", "info,seabird=debug");
    }

    // Now that everything is set up, load up the logger.
    pretty_env_logger::init_timed();

    // We ignore failures here because we want to fall back to loading from the
    // environment.
    if let Ok(path) = env_res {
        info!("Loaded env from {:?}", path);
    }

    // Load our config from command line arguments
    let config = client::ClientConfig::new(
        dotenv::var("SEABIRD_URL")
            .context("Missing $SEABIRD_URL. You must specify a Seabird host.")?,
        dotenv::var("SEABIRD_TOKEN")
            .context("Missing $SEABIRD_TOKEN. You must specify a valid auth token.")?,
        dotenv::var("DATABASE_URL")
            .context("Missing $DATABASE_URL. You must specify a Postgresql URL.")?,
        dotenv::var("SEABIRD_ENABLED_PLUGINS")
            .unwrap_or_else(|_| "".to_string())
            .split_terminator(',')
            .map(|s| s.to_string())
            .collect(),
        dotenv::var("SEABIRD_DISABLED_PLUGINS")
            .unwrap_or_else(|_| "".to_string())
            .split_terminator(',')
            .map(|s| s.to_string())
            .collect(),
    );

    let client = client::Client::new(config).await?;
    client.run().await
}
