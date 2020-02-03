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

#[tokio::main]
async fn main() -> Result<()> {
    // Load our config from command line arguments
    let config = Config::from_args();

    client::Client::new(&config.host[..]).await?;

    /*

    // Queue up the registration messages. Note that we need to do this manually
    // to avoid tx_send living past where it is given to the read_task.
    tx_send.send(format!("NICK :{}", &config.nick)).await?;
    tx_send
        .send(format!(
            "USER {} 0.0.0.0 0.0.0.0 :{}",
            config.user.as_ref().unwrap_or(&config.nick),
            config
                .name
                .as_ref()
                .or(config.user.as_ref())
                .unwrap_or(&config.nick)
        ))
        .await?;

    */

    Ok(())
}
