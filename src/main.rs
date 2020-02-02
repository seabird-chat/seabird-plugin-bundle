use tokio::sync::mpsc;

mod codec;
mod context;
mod io;

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
async fn main() -> Result<(), anyhow::Error> {
    // Load our config from command line arguments
    let config = Config::from_args();

    let (reader, writer) = io::connect(&config.host[..]).await?;

    let (mut tx_send, rx_send) = mpsc::channel(100);

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

    // Start the read and write tasks.
    let read = tokio::spawn(io::read_task(reader, tx_send));
    let send = tokio::spawn(io::send_task(writer, rx_send));

    match tokio::try_join!(read, send) {
        Err(e) => {
            println!("{}", e);
            Ok(())
        }
        Ok(_) => Ok(()),
    }
}
