use futures::prelude::*;
use irc::client::prelude::*;

#[tokio::main]
async fn main() -> irc::error::Result<()> {
    // By default, env_logger doesn't give a way of filtering logs, so we set
    // that up ourselves.
    let env = env_logger::Env::new().filter("RUST_LOG_FILTER");
    env_logger::init_from_env(env);

    let config = Config {
        nickname: Some("seabird51".to_owned()),
        server: Some("chat.freenode.net".to_owned()),
        port: Some(6697),
        use_ssl: true,
        channels: vec!["#encoded".to_owned()],
        ..Default::default()
    };

    let mut client = Client::from_config(config).await?;
    client.identify()?;

    let mut stream = client.stream()?;

    loop {
        let message = stream.select_next_some().await?;

        if let Command::PRIVMSG(ref target, ref msg) = message.command {
            if msg.starts_with("seabird:") {
                client.send_privmsg(target, "Hi!").unwrap();
            }
        }
    }
}
