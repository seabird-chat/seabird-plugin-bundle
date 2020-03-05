use std::net::ToSocketAddrs;
use std::sync::Arc;

use futures::future::try_join_all;
use native_tls::TlsConnector;
use tokio::io::{AsyncRead, AsyncWrite, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::stream::StreamExt;
use tokio::sync::{mpsc, Mutex};
use tokio_util::codec::FramedRead;
use tracing::{error, field, trace, trace_span};
use tracing_futures::Instrument;
use uuid::Uuid;

use crate::codec::IrcCodec;
use crate::plugins;
use crate::prelude::*;

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

    pub command_prefix: String,

    pub include_message_id_in_logs: bool,

    pub db_url: String,
}

impl ClientConfig {
    pub fn new(
        host: String,
        nick: String,
        user: Option<String>,
        name: Option<String>,
        db_url: String,
        command_prefix: String,
        include_message_id_in_logs: bool,
    ) -> Self {
        let user = user.unwrap_or_else(|| nick.clone());
        let name = name.unwrap_or_else(|| user.clone());

        ClientConfig {
            target: host,
            nick,
            user,
            name,
            db_url,
            command_prefix,
            include_message_id_in_logs,
        }
    }
}

// ClientState represents the internal state of the client at any given point in
// time.
pub struct ClientState {
    pub current_nick: String,
    pub config: Arc<ClientConfig>,
}

// Client represents the running bot.
pub struct Client {
    plugins: Vec<Box<dyn Plugin>>,
    state: Mutex<Arc<ClientState>>,

    db_client: Arc<tokio_postgres::Client>,
}

impl Client {
    async fn handle_message(&self, ctx: &Context) -> Result<()> {
        match ctx.as_event() {
            Event::Raw("PING", params) => {
                ctx.send("PONG", params).await?;
            }
            Event::RplWelcome(client, _) => {
                ctx.send("JOIN", vec!["#encoded-test"]).await?;

                // Copy what the server called us.
                let mut guard = self.state.lock().await;
                let state = &mut *guard;
                *state = Arc::new(ClientState {
                    current_nick: client.to_string(),
                    config: state.config.clone(),
                });

                trace!("Setting current nick to \"{}\"", state.current_nick);
            }
            _ => {}
        }

        Ok(())
    }

    async fn writer_task<T>(
        &self,
        mut writer: T,
        mut rx_sender: mpsc::Receiver<ToSend>,
    ) -> Result<()>
    where
        T: AsyncWrite + Unpin,
    {
        while let Some(to_send) = rx_sender.recv().await {
            let span = if self.state.lock().await.config.include_message_id_in_logs {
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

    async fn reader_task<R>(&self, reader: R, tx_sender: mpsc::Sender<ToSend>) -> Result<()>
    where
        R: AsyncRead + Unpin,
    {
        // Read all messages as irc::Messages.
        let mut framed = FramedRead::new(reader, IrcCodec::new());

        while let Some(msg) = framed.next().await.transpose()? {
            let mut ctx = Context::new(
                self.state.lock().await.clone(),
                msg,
                tx_sender.clone(),
                self.db_client.clone(),
            );

            let message_span = if ctx.client_state.config.include_message_id_in_logs {
                trace_span!("recv", id = field::debug(ctx.id))
            } else {
                trace_span!("recv")
            };
            let _enter = message_span.enter();

            trace!("<-- {}", ctx.msg);

            // Run any core handlers before plugins.
            self.handle_message(&ctx)
                .instrument(trace_span!("core"))
                .await?;

            // The state may have been changed in handle message, so we
            // re-create it.
            ctx.client_state = self.state.lock().await.clone();

            // Create an Arc out of our context to make it easier for async
            // plugins.
            let ctx = Arc::new(ctx);

            let plugins: Vec<_> = self
                .plugins
                .iter()
                .map(|p| p.handle_message(&ctx).instrument(trace_span!("plugin")))
                .collect();
            // TODO: add better context around error
            if let Err(e) = try_join_all(plugins).await {
                error!("Plugin(s) failed to execute: {}", e);
            }
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
    let plugins: Vec<Box<dyn Plugin>> = vec![
        Box::new(plugins::ChancePlugin::new()),
        Box::new(plugins::NoaaPlugin::new()),
        Box::new(plugins::KarmaPlugin::new()),
    ];

    let (mut db_client, db_connection) =
        tokio_postgres::connect(&config.db_url, tokio_postgres::NoTls).await?;

    // The connection object performs the actual communication with the database,
    // so spawn it off to run on its own.
    tokio::spawn(async move {
        if let Err(e) = db_connection.await {
            eprintln!("connection error: {}", e);
        }
    });

    crate::migrations::runner()
        .run_async(&mut db_client)
        .await?;

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
    let (tx_sender, rx_sender) = mpsc::channel(100);

    send_startup_messages(&config, tx_sender.clone()).await?;

    let state = Arc::new(ClientState {
        current_nick: config.nick.clone(),
        config: Arc::new(config),
    });

    let client = Client {
        plugins: plugins,
        state: Mutex::new(state),
        db_client: Arc::new(db_client),
    };

    let send = client.writer_task(writer, rx_sender);
    let read = client.reader_task(reader, tx_sender);

    let (send, read) = tokio::join!(send, read);

    send?;
    read?;

    Ok(())
}

#[derive(Clone)]
pub struct Context {
    pub msg: irc::Message,
    pub id: Uuid,

    sender: mpsc::Sender<ToSend>,
    client_state: Arc<ClientState>,

    db_client: Arc<tokio_postgres::Client>,
}

impl Context {
    fn new(
        client_state: Arc<ClientState>,
        msg: irc::Message,
        sender: mpsc::Sender<ToSend>,
        db_client: Arc<tokio_postgres::Client>,
    ) -> Self {
        Context {
            client_state,
            msg,
            sender,
            id: Uuid::new_v4(),
            db_client,
        }
    }

    pub async fn reply_target(&self) -> Result<&str> {
        match (&self.msg.command[..], self.msg.params.len()) {
            // If the first param is not the current nick, we need to respond to
            // the target, otherwise the prefix's nick.
            ("PRIVMSG", 2) => Ok(
                if self.msg.params[0] != self.client_state.current_nick[..] {
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

    pub fn command_prefix(&self) -> &str {
        &self.client_state.config.command_prefix
    }

    pub fn as_event(&self) -> Event<'_> {
        Event::from_message(self.client_state.clone(), &self.msg)
    }

    pub fn get_db(&self) -> Arc<tokio_postgres::Client> {
        self.db_client.clone()
    }
}
