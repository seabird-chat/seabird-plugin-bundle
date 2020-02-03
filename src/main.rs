mod client;
mod codec;
mod core;
mod error;
mod plugin;

use client::Context;
use error::Result;
use plugin::Plugin;

use structopt::StructOpt;

#[derive(StructOpt, Debug)]
#[structopt(name = "seabird", about = "A simple IRC bot.")]
struct Config {
    host: String,
    nick: String,

    #[structopt(long)]
    user: Option<String>,

    #[structopt(long)]
    name: Option<String>,
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
                .unwrap_or(&self.nick).to_string(),
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    // Load our config from command line arguments
    let config = Config::from_args();

    let client = client::Client::new(config.into());
    client.run().await
}
