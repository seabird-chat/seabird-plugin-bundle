use std::net::ToSocketAddrs;

use native_tls::TlsConnector;
use tokio::io::{AsyncRead, AsyncWrite, AsyncWriteExt, ReadHalf, WriteHalf};
use tokio::net::TcpStream;
use tokio::stream::StreamExt;
use tokio::sync::mpsc;
use tokio_tls::TlsStream;
use tokio_util::codec::FramedRead;

use crate::codec::IrcCodec;
use crate::context::Context;

pub(crate) async fn connect(
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
        .ok_or_else(|| anyhow::anyhow!("Failed to look up address"))?;

    let socket = TcpStream::connect(&addr).await?;
    let cx = TlsConnector::builder()
        .danger_accept_invalid_certs(true)
        .build()?;
    let cx = tokio_tls::TlsConnector::from(cx);

    let socket = cx.connect(target, socket).await?;

    let (reader, writer) = tokio::io::split(socket);

    Ok((reader, writer))
}

pub(crate) async fn read_task<T>(reader: T, out: mpsc::Sender<String>) -> Result<(), anyhow::Error>
where
    T: AsyncRead + Unpin,
{
    // Read all messages as irc::Messages.
    let mut framed = FramedRead::new(reader, IrcCodec::new());

    while let Some(msg) = framed.next().await.transpose()? {
        println!("<-- {}", msg);

        let mut ctx = Context::new(msg, out.clone());

        match &ctx.msg.command[..] {
            "PING" => {
                ctx.send_msg(&irc::Message::new(
                    "PONG".to_string(),
                    ctx.msg.params.clone(),
                ))
                .await?;
            }
            "001" => {
                ctx.send("JOIN", vec!["#encoded-test"]).await?;
                ctx.send("JOIN", vec!["#rust"]).await?;
            }
            _ => {}
        }
    }

    Ok(())
}

pub(crate) async fn send_task<T>(
    mut writer: T,
    mut msgs: mpsc::Receiver<String>,
) -> Result<(), anyhow::Error>
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
