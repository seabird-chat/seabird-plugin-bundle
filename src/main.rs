mod irc;

use tokio::prelude::*;
use tokio::codec::Framed;
use tokio::net::TcpStream;
use tokio_tls::TlsConnector;

use failure::Error;

#[tokio::main]
async fn main() -> Result<(), Error> {
    let stream = TcpStream::connect("irc.hs.gy:9999").await?;

    let connector = native_tls::TlsConnector::builder()
        .danger_accept_invalid_certs(true)
        .danger_accept_invalid_hostnames(true)
        .build()?;
    let connector = TlsConnector::from(connector);

    let stream = connector.connect("irc.hs.gy", stream).await?;

    let mut lines = Framed::new(stream, irc::Codec::new());

    // In a loop, read data from the socket and write the data back.
    loop {
        let msg = lines.next().await;

        match msg.unwrap() {
            Ok(msg) => println!("{}: {}", msg.command, msg),
            Err(_err) => println!("Error"),
        };
    }
}
