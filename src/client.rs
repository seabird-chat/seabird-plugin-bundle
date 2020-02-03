use std::net::ToSocketAddrs;

use native_tls::TlsConnector;
use tokio::io::{AsyncRead, AsyncWrite, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::stream::StreamExt;
use tokio::sync::mpsc;
use tokio_util::codec::FramedRead;

use crate::codec::IrcCodec;
use crate::{Plugin, Result};

use crate::core::{Ping, Welcome};

struct ClientConfig {}

pub struct Client {
    sender: mpsc::Sender<String>,
    core_plugins: Vec<Box<dyn Plugin>>,
    plugins: Vec<Box<dyn Plugin>>,
}

impl Client {
    pub async fn new(target: &str) -> Result<()> {
        // Step 1: Connect to the server
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

        // Step 2: Wire up all the pieces
        let (tx_send, rx_send) = mpsc::channel(100);

        let client = Client {
            sender: tx_send,
            core_plugins: vec![Box::new(Ping::new())],
            plugins: vec![Box::new(Welcome::new())],
        };

        // Start the read and write tasks.
        tokio::spawn(send_task(writer, rx_send));

        read_task(reader, client).await
    }

    pub async fn send(&self, command: &str, params: Vec<&str>) -> Result<()> {
        self.send_msg(&irc::Message::new(
            command.to_string(),
            params.into_iter().map(|s| s.to_string()).collect(),
        ))
        .await
    }

    pub async fn send_msg(&self, msg: &irc::Message) -> Result<()> {
        self.sender.clone().send(msg.to_string()).await?;
        Ok(())
    }
}

async fn read_task<R>(reader: R, client: Client) -> Result<()>
where
    R: AsyncRead + Unpin,
{
    // Read all messages as irc::Messages.
    let mut framed = FramedRead::new(reader, IrcCodec::new());

    while let Some(msg) = framed.next().await.transpose()? {
        println!("<-- {}", msg);

        let ctx = Context::new(msg, client.sender.clone());

        // Run through all core plugins before less important plugins.
        let plugins: Vec<_> = client
            .core_plugins
            .iter()
            .map(|p| p.handle_message(&ctx))
            .collect();
        for plugin in plugins {
            plugin.await?;
        }

        let plugins: Vec<_> = client
            .plugins
            .iter()
            .map(|p| p.handle_message(&ctx))
            .collect();
        for plugin in plugins {
            plugin.await?;
        }
    }

    Ok(())
}

async fn send_task<T>(mut writer: T, mut msgs: mpsc::Receiver<String>) -> Result<()>
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

pub struct Context {
    pub msg: irc::Message,
    sender: mpsc::Sender<String>,
}

impl Context {
    fn new(msg: irc::Message, sender: mpsc::Sender<String>) -> Self {
        Context { msg, sender }
    }

    pub async fn send(&self, command: &str, params: Vec<&str>) -> Result<()> {
        self.send_msg(&irc::Message::new(
            command.to_string(),
            params.into_iter().map(|s| s.to_string()).collect(),
        ))
        .await
    }

    pub async fn send_msg(&self, msg: &irc::Message) -> Result<()> {
        self.sender.clone().send(msg.to_string()).await?;
        Ok(())
    }
}
