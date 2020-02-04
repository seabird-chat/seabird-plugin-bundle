use std::net::ToSocketAddrs;

use futures::future::try_join_all;
use native_tls::TlsConnector;
use tokio::io::{AsyncRead, AsyncWrite, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::stream::StreamExt;
use tokio::sync::mpsc;
use tokio_util::codec::FramedRead;

use crate::codec::IrcCodec;
use crate::plugins;
use crate::{Event, Plugin, Result};

pub struct ClientConfig {
    pub target: String,
    pub nick: String,
    pub user: String,
    pub name: String,
}

pub struct Client {
    config: ClientConfig,
    plugins: Vec<Box<dyn Plugin>>,
}

struct ClientState {
    current_nick: String,
}

impl ClientState {
    async fn handle_message(&mut self, ctx: &mut Context) -> Result<()> {
        match ctx.as_event() {
            Event::Raw("PING", params) => {
                ctx.send("PONG", params).await?;
            }
            Event::RPL_WELCOME(client, _) => {
                ctx.send("JOIN", vec!["#encoded-test"]).await?;

                // Copy what the server called us.
                self.current_nick = client.to_string();
                ctx.current_nick = client.to_string();
            }
            _ => {}
        }

        Ok(())
    }
}

impl Client {
    pub fn new(config: ClientConfig) -> Self {
        let mut plugins: Vec<Box<dyn Plugin>> = Vec::new();

        #[cfg(feature = "db")]
        {
            plugins.push(Box::new(Karma::new()));
        }

        plugins.push(Box::new(plugins::Chance::new()));

        Client { config, plugins }
    }

    pub async fn run(self) -> Result<()> {
        // Step 1: Connect to the server
        let addr = self
            .config
            .target
            .to_socket_addrs()?
            .next()
            .ok_or_else(|| anyhow::anyhow!("Failed to look up address"))?;

        let socket = TcpStream::connect(&addr).await?;
        let cx = TlsConnector::builder()
            .danger_accept_invalid_certs(true)
            .build()?;
        let cx = tokio_tls::TlsConnector::from(cx);

        let socket = cx.connect(&self.config.target, socket).await?;

        let (reader, writer) = tokio::io::split(socket);

        // Step 2: Wire up all the pieces
        let (tx_send, rx_send) = mpsc::channel(100);

        let send = tokio::spawn(Self::send_task(writer, rx_send));
        let read = tokio::spawn(Self::read_task(reader, tx_send.clone(), self));

        let (send, read) = tokio::try_join!(send, read)?;

        send?;
        read?;

        Ok(())
    }
}

impl Client {
    async fn register_task(mut tx_send: mpsc::Sender<String>, config: &ClientConfig) -> Result<()> {
        tx_send.send(format!("NICK :{}", &config.nick)).await?;
        tx_send
            .send(format!(
                "USER {} 0.0.0.0 0.0.0.0 :{}",
                &config.user, &config.name
            ))
            .await?;

        Ok(())
    }

    async fn read_task<R>(reader: R, tx_send: mpsc::Sender<String>, client: Client) -> Result<()>
    where
        R: AsyncRead + Unpin,
    {
        Self::register_task(tx_send.clone(), &client.config).await?;

        let mut state = ClientState {
            current_nick: client.config.nick.clone(),
        };

        // Read all messages as irc::Messages.
        let mut framed = FramedRead::new(reader, IrcCodec::new());

        while let Some(msg) = framed.next().await.transpose()? {
            println!("<-- {}", msg);

            let mut ctx = Context::new(msg, state.current_nick.clone(), tx_send.clone());

            // Run any core handlers before plugins.
            state.handle_message(&mut ctx).await?;

            let plugins: Vec<_> = client
                .plugins
                .iter()
                .map(|p| p.handle_message(&ctx))
                .collect();
            try_join_all(plugins).await?;
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
}

pub struct Context {
    pub msg: irc::Message,
    sender: mpsc::Sender<String>,
    current_nick: String,
}

impl Context {
    fn new(msg: irc::Message, current_nick: String, sender: mpsc::Sender<String>) -> Self {
        Context {
            msg,
            current_nick,
            sender,
        }
    }

    pub fn reply_target(&self) -> Result<&str> {
        match (&self.msg.command[..], self.msg.params.len()) {
            // If the first param is not the current nick, we need to respond to
            // the target, otherwise the prefix's nick.
            ("PRIVMSG", 2) => Ok(if self.msg.params[0] != &self.current_nick[..] {
                &self.msg.params[0][..]
            } else {
                &self
                    .msg
                    .prefix
                    .as_ref()
                    .ok_or_else(|| anyhow::anyhow!("Prefix missing"))?
                    .nick[..]
            }),
            _ => Err(anyhow::anyhow!(
                "Tried to find a target for an invalid message"
            )),
        }
    }

    pub fn sender(&self) -> Result<&str> {
        match (&self.msg.command[..], self.msg.params.len()) {
            // Only return the prefix if it came from a valid message.
            ("PRIVMSG", 2) => Ok(&self
                .msg
                .prefix
                .as_ref()
                .ok_or_else(|| anyhow::anyhow!("Prefix missing"))?
                .nick[..]),
            _ => Err(anyhow::anyhow!(
                "Tried to find a sender for an invalid message"
            )),
        }
    }

    pub async fn mention_reply(&self, msg: &str) -> Result<()> {
        let sender = self.sender()?;
        let target = self.reply_target()?;

        // If the target matches the sender, it's a privmsg so we shouldn't send
        // a prefix.
        if target == sender {
            self.send("PRIVMSG", vec![target, msg]).await
        } else {
            self.send("PRIVMSG", vec![target, &format!("{}: {}", sender, msg)[..]]).await
        }
    }

    pub async fn reply(&self, msg: &str) -> Result<()> {
        self.send("PRIVMSG", vec![self.reply_target()?, msg]).await
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

    pub fn as_event(&self) -> Event<'_> {
        (&self.msg).into()
    }
}
