use std::net::ToSocketAddrs;
use std::sync::Arc;

use futures::future::try_join_all;
use futures::lock::Mutex;
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
    fn new_without_source(message: String) -> Self {
        Self {
            message,
            source_message_id: None,
        }
    }

    fn new(message: String, source_message_id: Uuid) -> Self {
        Self {
            message,
            source_message_id: Some(source_message_id),
        }
    }
}

#[derive(Clone)]
pub struct ClientConfig {
    pub target: String,
    pub nick: String,
    pub user: String,
    pub name: String,

    #[cfg(feature = "db")]
    pub db_url: String,

    pub command_prefix: String,

    pub include_message_id_in_logs: bool,
}

pub struct ClientState {
    current_nick: String,
    config: ClientConfig,
    plugins: Vec<Box<dyn Plugin>>,
    #[cfg(feature = "db")]
    pub db_pool: DbPool,
}

impl ClientState {
    async fn handle_message(&mut self, ctx: &mut Context) -> Result<()> {
        match ctx.as_event().await {
            Event::Raw("PING", params) => {
                ctx.send("PONG", params).await?;
            }
            Event::RplWelcome(client, _) => {
                ctx.send("JOIN", vec!["#encoded-test"]).await?;

                // Copy what the server called us.
                self.current_nick = client.to_string();

                trace!("Setting current nick to \"{}\"", self.current_nick);
            }
            _ => {}
        }

        Ok(())
    }
}

async fn send_startup_messages(
    config: &ClientConfig,
    mut tx_send: mpsc::Sender<ToSend>,
) -> Result<()> {
    tx_send
        .send(ToSend::new_without_source(format!(
            "NICK :{}",
            &config.nick
        )))
        .await?;
    tx_send
        .send(ToSend::new_without_source(format!(
            "USER {} 0.0.0.0 0.0.0.0 :{}",
            &config.user, &config.name
        )))
        .await?;

    Ok(())
}

pub async fn run(config: ClientConfig) -> Result<()> {
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

    // Step 1: Connect to the server
    let addr = config
        .target
        .to_socket_addrs()?
        .next()
        .ok_or_else(|| anyhow::anyhow!("Failed to look up address"))?;

    let socket = TcpStream::connect(&addr).await?;
    let cx = TlsConnector::builder()
        .danger_accept_invalid_certs(true)
        .build()?;
    let cx = tokio_tls::TlsConnector::from(cx);

    let socket = cx.connect(&config.target, socket).await?;

    let (reader, writer) = tokio::io::split(socket);

    // Step 2: Wire up all the pieces
    let (tx_send, rx_send) = mpsc::channel(100);

    send_startup_messages(&config, tx_send.clone()).await?;

    let state = Arc::new(Mutex::new(ClientState {
        current_nick: config.nick.clone(),
        config: config.clone(),
        plugins: plugins,
        #[cfg(feature = "db")]
        db_pool: db_pool.clone(),
    }));

    let send = tokio::spawn(writer_task(Arc::clone(&state), writer, rx_send));
    let read = tokio::spawn(reader_task(Arc::clone(&state), reader, tx_send.clone()));

    let (send, read) = tokio::try_join!(send, read)?;

    send?;
    read?;

    Ok(())
}

async fn reader_task<R>(
    client_state: Arc<Mutex<ClientState>>,
    reader: R,
    tx_send: mpsc::Sender<ToSend>,
) -> Result<()>
where
    R: AsyncRead + Unpin,
{
    // Read all messages as irc::Messages.
    let mut framed = FramedRead::new(reader, IrcCodec::new());

    while let Some(msg) = framed.next().await.transpose()? {
        let mut ctx = Context::new(Arc::clone(&client_state), msg, tx_send.clone());
        let message_span = if client_state.lock().await.config.include_message_id_in_logs {
            trace_span!("recv", id = field::debug(ctx.id))
        } else {
            trace_span!("recv")
        };
        let _enter = message_span.enter();

        trace!("<-- {}", ctx.msg);

        // Run any core handlers before plugins.
        client_state
            .lock()
            .await
            .handle_message(&mut ctx)
            .instrument(trace_span!("core"))
            .await?;

        {
            let state = client_state.lock().await;
            // We're taking the lock recursively here and blocking forever.
            // Maybe we can freeze the state after running core handlers
            // and make everything read-only, and then remove the need
            // for a lock?
            let plugins: Vec<_> = state
                .plugins
                .iter()
                .map(|p| p.handle_message(&ctx).instrument(trace_span!("plugin")))
                .collect();
            let _results = try_join_all(plugins).await?;
        }
        //println!("{:?}", results);
    }

    Ok(())
}

async fn writer_task<T>(
    client_state: Arc<Mutex<ClientState>>,
    mut writer: T,
    mut msgs: mpsc::Receiver<ToSend>,
) -> Result<()>
where
    T: AsyncWrite + Unpin,
{
    while let Some(to_send) = msgs.recv().await {
        let span = if client_state.lock().await.config.include_message_id_in_logs {
            let source = field::debug(
                to_send
                    .source_message_id
                    .map(|id| id.to_string())
                    .unwrap_or("none".to_string()),
            );
            trace_span!("send", source_id = source)
        } else {
            trace_span!("send")
        };
        let _enter = span.enter();

        trace!("--> {}", to_send.message);
        writer.write_all(to_send.message.as_bytes()).await?;
        writer.write_all(b"\r\n").await?;
    }

    // TODO: this is actually an error - the send queue dried up.
    Ok(())
}

#[derive(Clone)]
pub struct Context {
    pub client_state: Arc<Mutex<ClientState>>,
    pub msg: irc::Message,
    sender: mpsc::Sender<ToSend>,
    pub id: Uuid,
}

impl Context {
    fn new(
        client_state: Arc<Mutex<ClientState>>,
        msg: irc::Message,
        sender: mpsc::Sender<ToSend>,
    ) -> Self {
        Context {
            client_state,
            msg,
            sender,
            id: Uuid::new_v4(),
        }
    }

    pub async fn reply_target(&self) -> Result<&str> {
        match (&self.msg.command[..], self.msg.params.len()) {
            // If the first param is not the current nick, we need to respond to
            // the target, otherwise the prefix's nick.
            ("PRIVMSG", 2) => Ok(
                if self.msg.params[0] != self.client_state.lock().await.current_nick[..] {
                    &self.msg.params[0][..]
                } else {
                    &self
                        .msg
                        .prefix
                        .as_ref()
                        .ok_or_else(|| anyhow::anyhow!("Prefix missing"))?
                        .nick[..]
                },
            ),
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
        let target = self.reply_target().await?;

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
        self.send("PRIVMSG", vec![self.reply_target().await?, msg])
            .await
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
            .send(ToSend::new(msg.to_string(), self.id.clone()))
            .await?;
        Ok(())
    }

    pub async fn as_event(&self) -> Event<'_> {
        Event::from_message(
            &self.client_state.lock().await.config.command_prefix,
            &self.msg,
        )
    }
}
