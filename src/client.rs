use std::net::ToSocketAddrs;

use futures::future::try_join_all;
use native_tls::TlsConnector;
use tokio::io::{AsyncRead, AsyncWrite, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::stream::StreamExt;
use tokio::sync::mpsc;
use tokio_util::codec::FramedRead;
use tracing::{field, trace, trace_span};
use tracing_futures::Instrument;
use uuid::Uuid;

#[cfg(feature = "db")]
use diesel::{r2d2, PgConnection};

use crate::codec::IrcCodec;
use crate::plugins;
use crate::prelude::*;

#[cfg(feature = "db")]
embed_migrations!("./migrations/");

#[cfg(feature = "db")]
pub type DbPool = r2d2::Pool<r2d2::ConnectionManager<PgConnection>>;
pub type DbConn = r2d2::PooledConnection<r2d2::ConnectionManager<PgConnection>>;

#[derive(Debug)]
struct ToSend {
    message: String,
    source_message_id: Option<Uuid>,
}

impl ToSend {
    fn raw(message: String) -> Self {
        Self {
            message,
            source_message_id: None,
        }
    }

    fn with_source(message: String, source_message_id: Uuid) -> Self {
        Self {
            message,
            source_message_id: Some(source_message_id),
        }
    }
}

pub struct ClientConfig {
    pub target: String,
    pub nick: String,
    pub user: String,
    pub name: String,

    #[cfg(feature = "db")]
    pub db_url: String,
}

pub struct Client {
    config: ClientConfig,
    plugins: Vec<Box<dyn Plugin>>,

    #[cfg(feature = "db")]
    db_pool: DbPool,
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
            Event::RplWelcome(client, _) => {
                ctx.send("JOIN", vec!["#encoded-test"]).await?;

                // Copy what the server called us.
                self.current_nick = client.to_string();
                ctx.current_nick = client.to_string();

                trace!("Setting current nick to \"{}\"", self.current_nick);
            }
            _ => {}
        }

        Ok(())
    }
}

impl Client {
    pub fn new(config: ClientConfig) -> Result<Self> {
        let mut plugins: Vec<Box<dyn Plugin>> = vec![
            Box::new(plugins::ChancePlugin::new()),
            Box::new(plugins::NoaaPlugin::new()),
        ];

        #[cfg(feature = "db")]
        let db_pool = {
            plugins.push(Box::new(plugins::KarmaPlugin::new()));

            let db_pool = DbPool::new(r2d2::ConnectionManager::new(&config.db_url[..]))?;

            // Run all migrations
            embedded_migrations::run_with_output(&db_pool.get()?, &mut std::io::stderr())?;

            db_pool
        };

        Ok(Client {
            config,
            plugins,
            #[cfg(feature = "db")]
            db_pool,
        })
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
    async fn register_task(mut tx_send: mpsc::Sender<ToSend>, config: &ClientConfig) -> Result<()> {
        tx_send
            .send(ToSend::raw(format!("NICK :{}", &config.nick)))
            .await?;
        tx_send
            .send(ToSend::raw(format!(
                "USER {} 0.0.0.0 0.0.0.0 :{}",
                &config.user, &config.name
            )))
            .await?;

        Ok(())
    }

    async fn read_task<R>(reader: R, tx_send: mpsc::Sender<ToSend>, client: Client) -> Result<()>
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
            let mut ctx = Context::new(
                msg,
                state.current_nick.clone(),
                tx_send.clone(),
                #[cfg(feature = "db")]
                client.db_pool.clone(),
            );
            let message_span = trace_span!("recv", id = field::debug(ctx.id));
            let _enter = message_span.enter();

            trace!("<-- {}", ctx.msg);

            // Run any core handlers before plugins.
            state
                .handle_message(&mut ctx)
                .instrument(trace_span!("core"))
                .await?;

            let plugins: Vec<_> = client
                .plugins
                .iter()
                .map(|p| p.handle_message(&ctx).instrument(trace_span!("plugin")))
                .collect();
            let _results = try_join_all(plugins).await?;
            //println!("{:?}", results);
        }

        Ok(())
    }

    async fn send_task<T>(mut writer: T, mut msgs: mpsc::Receiver<ToSend>) -> Result<()>
    where
        T: AsyncWrite + Unpin,
    {
        while let Some(to_send) = msgs.recv().await {
            let source = field::debug(
                to_send
                    .source_message_id
                    .map(|id| id.to_string())
                    .unwrap_or("none".to_string()),
            );
            let span = trace_span!("send", source_id = source);
            let _enter = span.enter();

            trace!("--> {}", to_send.message);
            writer.write_all(to_send.message.as_bytes()).await?;
            writer.write_all(b"\r\n").await?;
        }

        // TODO: this is actually an error - the send queue dried up.
        Ok(())
    }
}

#[derive(Clone)]
pub struct Context {
    pub msg: irc::Message,
    sender: mpsc::Sender<ToSend>,
    current_nick: String,
    #[cfg(feature = "db")]
    pub db_pool: DbPool,
    pub id: Uuid,
}

impl Context {
    fn new(
        msg: irc::Message,
        current_nick: String,
        sender: mpsc::Sender<ToSend>,
        #[cfg(feature = "db")] db_pool: DbPool,
    ) -> Self {
        Context {
            msg,
            current_nick,
            sender,
            #[cfg(feature = "db")]
            db_pool,
            id: Uuid::new_v4(),
        }
    }

    pub fn reply_target(&self) -> Result<&str> {
        match (&self.msg.command[..], self.msg.params.len()) {
            // If the first param is not the current nick, we need to respond to
            // the target, otherwise the prefix's nick.
            ("PRIVMSG", 2) => Ok(if self.msg.params[0] != self.current_nick[..] {
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
            self.send("PRIVMSG", vec![target, &format!("{}: {}", sender, msg)[..]])
                .await
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
        self.sender
            .clone()
            .send(ToSend::with_source(msg.to_string(), self.id.clone()))
            .await?;
        Ok(())
    }

    pub fn as_event(&self) -> Event<'_> {
        (&self.msg).into()
    }
}
