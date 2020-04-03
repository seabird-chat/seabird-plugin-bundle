use std::collections::BTreeSet;
use std::net::ToSocketAddrs;
use std::sync::Arc;

use futures::future::{try_join_all, TryFutureExt};
use native_tls::TlsConnector;
use tokio::io::{AsyncBufReadExt, AsyncRead, AsyncWrite, AsyncWriteExt, BufReader};
use tokio::net::TcpStream;
use tokio::stream::StreamExt;
use tokio::sync::{mpsc, Mutex};

use crate::prelude::*;

#[derive(Clone, Debug)]
pub struct ClientConfig {
    pub target: String,
    pub nick: String,
    pub user: String,
    pub name: String,
    pub password: Option<String>,

    pub command_prefix: String,

    pub enabled_plugins: BTreeSet<String>,
    pub disabled_plugins: BTreeSet<String>,

    pub db_url: String,
}

impl ClientConfig {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        host: String,
        nick: String,
        user: Option<String>,
        name: Option<String>,
        password: Option<String>,
        db_url: String,
        command_prefix: String,
        enabled_plugins: BTreeSet<String>,
        disabled_plugins: BTreeSet<String>,
    ) -> Self {
        let user = user.unwrap_or_else(|| nick.clone());
        let name = name.unwrap_or_else(|| user.clone());

        ClientConfig {
            target: host,
            nick,
            user,
            name,
            password,
            db_url,
            command_prefix,
            enabled_plugins,
            disabled_plugins,
        }
    }
}

impl ClientConfig {
    /// If enabled_plugins is not specified or is empty, all plugins are allowed
    /// to be loaded, otherwise only specified plugins will be loaded.
    ///
    /// Any plugins in disabled_plugins which were otherwise enabled, will be
    /// skipped.
    ///
    /// Note that this function does not check for plugin validity, only if it
    /// would be enabled based on the name.
    pub fn plugin_enabled(&self, plugin_name: &str) -> bool {
        if self.disabled_plugins.contains(plugin_name) {
            return false;
        }

        // If enabled_plugins has no values, all are enabled.
        self.enabled_plugins.is_empty() || self.enabled_plugins.contains(plugin_name)
    }
}

// ClientState represents the internal state of the client at any given point in
// time.
#[derive(Debug)]
pub struct ClientState {
    pub current_nick: String,
    pub config: Arc<ClientConfig>,
}

// Client represents the running bot.
pub struct Client {
    plugin_senders: Mutex<Vec<mpsc::Sender<Arc<Context>>>>,
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
                info!("Connected!");

                ctx.send("JOIN", vec!["#main"]).await?;
                ctx.send("JOIN", vec!["#encoded"]).await?;
                ctx.send("JOIN", vec!["#encoded-test"]).await?;
                ctx.send("JOIN", vec!["#minecraft"]).await?;

                // Copy what the server called us.
                let mut guard = self.state.lock().await;
                let state = &mut *guard;
                *state = Arc::new(ClientState {
                    current_nick: client.to_string(),
                    config: state.config.clone(),
                });

                debug!("Setting current nick to \"{}\"", state.current_nick);
            }
            _ => {}
        }

        Ok(())
    }

    async fn writer_task<T>(
        &self,
        mut writer: T,
        mut rx_sender: mpsc::Receiver<String>,
    ) -> Result<()>
    where
        T: AsyncWrite + Unpin,
    {
        while let Some(message) = rx_sender.recv().await {
            trace!("--> {}", message);
            writer.write_all(message.as_bytes()).await?;
            writer.write_all(b"\r\n").await?;
            writer.flush().await?;
        }

        // TODO: this is actually an error - the send queue dried up.
        Err(format_err!("writer_task exited early"))
    }

    async fn reader_task<R>(&self, reader: R, tx_sender: mpsc::Sender<String>) -> Result<()>
    where
        R: AsyncRead + Unpin,
    {
        let mut stream = BufReader::new(reader).lines();

        while let Some(line) = stream.next().await.transpose()? {
            let msg: irc::Message = line.parse()?;

            let mut ctx = Context::new(
                self.state.lock().await.clone(),
                msg,
                tx_sender.clone(),
                self.db_client.clone(),
            );

            trace!("<-- {}", ctx.msg);

            // Run any core handlers before plugins.
            self.handle_message(&ctx).await?;

            // The state may have been changed in handle message, so we
            // re-create it.
            ctx.client_state = self.state.lock().await.clone();

            // Create an Arc out of our context to make it easier for async
            // plugins.
            let ctx = Arc::new(ctx);

            for plugin in self.plugin_senders.lock().await.iter_mut() {
                plugin.send(ctx.clone()).await?;
            }
        }

        Err(format_err!("reader_task exited early"))
    }
}

async fn send_startup_messages(
    config: &ClientConfig,
    mut tx_send: mpsc::Sender<String>,
) -> Result<()> {
    if let Some(password) = &config.password {
        tx_send.send(format!("PASS :{}", &password)).await?;
    }

    tx_send.send(format!("NICK :{}", &config.nick)).await?;
    tx_send
        .send(format!(
            "USER {} 0.0.0.0 0.0.0.0 :{}",
            &config.user, &config.name
        ))
        .await?;

    Ok(())
}

pub async fn run(config: ClientConfig) -> Result<()> {
    let (mut db_client, db_connection) =
        tokio_postgres::connect(&config.db_url, tokio_postgres::NoTls).await?;

    // The connection object performs the actual communication with the
    // database, so spawn it off to run on its own.
    //
    // TODO: make sure it actually fails the bot if it exits.
    tokio::spawn(async move {
        if let Err(e) = db_connection.await {
            panic!("connection error: {}", e);
        }
    });

    crate::migrations::runner()
        .run_async(&mut db_client)
        .await?;

    let (plugin_senders, plugin_tasks): (Vec<_>, Vec<_>) =
        crate::plugin::load(&config)?.into_iter().unzip();

    // Step 1: Connect to the server
    let addr = config
        .target
        .to_socket_addrs()?
        .next()
        .ok_or_else(|| format_err!("Failed to look up address"))?;

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
        plugin_senders: Mutex::new(plugin_senders),
        state: Mutex::new(state),
        db_client: Arc::new(db_client),
    };

    let send = client.writer_task(writer, rx_sender);
    let read = client.reader_task(reader, tx_sender);

    let (_send, _read, plugins) =
        tokio::try_join!(send, read, try_join_all(plugin_tasks).map_err(|e| e.into()))?;

    // Ensure no plugins exited
    for plugin in plugins {
        plugin?;
    }

    Ok(())
}

#[derive(Clone, Debug)]
pub struct Context {
    pub msg: irc::Message,

    sender: mpsc::Sender<String>,
    client_state: Arc<ClientState>,

    db_client: Arc<tokio_postgres::Client>,
}

impl Context {
    fn new(
        client_state: Arc<ClientState>,
        msg: irc::Message,
        sender: mpsc::Sender<String>,
        db_client: Arc<tokio_postgres::Client>,
    ) -> Self {
        Context {
            client_state,
            msg,
            sender,
            db_client,
        }
    }

    pub fn reply_target(&self) -> Option<&str> {
        match (&self.msg.command[..], self.msg.params.len()) {
            // If the first param is not the current nick, we need to respond to
            // the target, otherwise the prefix's nick.
            ("PRIVMSG", 2) => {
                if self.msg.params[0] != self.client_state.current_nick[..] {
                    Some(&self.msg.params[0][..])
                } else {
                    self.msg.prefix.as_ref().map(|p| &p.nick[..])
                }
            }
            _ => None,
        }
    }

    pub fn sender(&self) -> Option<&str> {
        match (&self.msg.command[..], self.msg.params.len()) {
            // Only return the prefix if it came from a valid message.
            ("PRIVMSG", 2) => self.msg.prefix.as_ref().map(|p| &p.nick[..]),
            _ => None,
        }
    }

    pub async fn mention_reply(&self, msg: &str) -> Result<()> {
        let sender = self
            .sender()
            .ok_or_else(|| format_err!("Tried to get the sender of an event without one"))?;
        let target = self
            .reply_target()
            .ok_or_else(|| format_err!("Tried to reply to an event without a targets"))?;

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
        self.send(
            "PRIVMSG",
            vec![
                self.reply_target()
                    .ok_or_else(|| format_err!("Tried to reply to an event without a targets"))?,
                msg,
            ],
        )
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
        self.sender.clone().send(msg.to_string()).await?;
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
