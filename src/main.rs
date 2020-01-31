use std::net::ToSocketAddrs;

use native_tls::TlsConnector;
use tokio::io::{AsyncRead, AsyncWrite, ReadHalf, WriteHalf, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::stream::StreamExt;
use tokio::sync::mpsc;
use tokio_tls::TlsStream;
use tokio_util::codec::FramedRead;

mod codec;
use codec::IrcCodec;

async fn read_task<T>(reader: T, out: mpsc::Sender<String>) -> Result<(), anyhow::Error>
where
    T: AsyncRead + Unpin,
{
    let mut framed = FramedRead::new(reader, IrcCodec::new());

    while let Some(msg) = framed.next().await.transpose()? {
        println!("<-- {}", msg);
    }

    Ok(())
}

async fn send_task<T>(mut writer: T, mut msgs: mpsc::Receiver<String>) -> Result<(), anyhow::Error>
where
    T: AsyncWrite + Unpin,
{
    while let Some(line) = msgs.recv().await {
        println!("--> {}", line);
        writer.write_all(line.as_bytes()).await?;
        writer.write_all(b"\r\n").await?;
    }

    // TODO: this is actually an error - the send queue dried up.
    Ok(())
}

async fn connect(
    target: &str,
) -> Result<
    (
        ReadHalf<TlsStream<TcpStream>>,
        WriteHalf<TlsStream<TcpStream>>,
    ),
    anyhow::Error,
> {
    let addr = target
        .to_socket_addrs()?
        .next()
        .ok_or(anyhow::anyhow!("Failed to look up address"))?;

    let socket = TcpStream::connect(&addr).await?;
    let cx = TlsConnector::builder()
        .danger_accept_invalid_certs(true)
        .build()?;
    let cx = tokio_tls::TlsConnector::from(cx);

    let socket = cx.connect(target, socket).await?;

    let (reader, writer) = tokio::io::split(socket);

    Ok((reader, writer))
}

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    let (reader, writer) = connect("chat.freenode.net:6697").await?;

    let (mut tx_send, rx_send) = mpsc::channel(100);

    let read = tokio::spawn(read_task(reader, tx_send.clone()));
    let send = tokio::spawn(send_task(writer, rx_send));

    tx_send.send("NICK seabird51".to_string()).await?;
    tx_send
        .send(format!("USER seabird 0.0.0.0 0.0.0.0 :Seabird Bot"))
        .await?;

    match tokio::try_join!(read, send) {
        Err(e) => {
            println!("{}", e);
            Ok(())
        }
        Ok(_) => Ok(()),
    }
}
