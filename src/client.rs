use std::collections::{BTreeMap, BTreeSet};
use std::net::ToSocketAddrs;
use std::sync::Arc;

use anyhow::format_err;
use futures::future::try_join_all;
use maplit::btreeset;
use native_tls::TlsConnector;
use tokio::io::{AsyncBufReadExt, AsyncRead, AsyncWrite, AsyncWriteExt, BufReader};
use tokio::net::TcpStream;
use tokio::stream::StreamExt;
use tokio::sync::{mpsc, Mutex};

use crate::plugins;
use crate::prelude::*;

#[derive(Clone)]
pub struct ClientConfig {
    pub target: String,
    pub nick: String,
    pub user: String,
    pub name: String,
    pub password: Option<String>,

    pub command_prefix: String,

    pub enabled_plugins: Vec<String>,

    pub db_url: String,

    pub darksky_api_key: Option<String>,
    pub maps_api_key: Option<String>,
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
        enabled_plugins: Vec<String>,
        darksky_api_key: Option<String>,
        maps_api_key: Option<String>,
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
            darksky_api_key,
            maps_api_key,
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
    plugins: Vec<Option<Box<dyn Plugin>>>,
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
        Ok(())
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

            let mut futures = Vec::new();
            for plugin in self.plugins.iter() {
                if let Some(plugin) = plugin {
                    futures.push(plugin.handle_message(&ctx));
                }
            }

            // TODO: add better context around error
            if let Err(e) = try_join_all(futures).await {
                error!("Plugin(s) failed to execute: {}", e);
            }
        }

        Ok(())
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

#[derive(PartialEq, Eq)]
enum PluginState {
    Enabled,
    Disabled,
}

/// Validates the plugins given in the bot's environment
/// and builds a map of plugin_name -> plugin_state for
/// use later.
fn validate_plugin_config(config: &ClientConfig) -> Result<BTreeMap<String, PluginState>> {
    let supported_plugins = btreeset![
        "forecast".to_string(),
        "karma".to_string(),
        "mention".to_string(),
        "net_tools".to_string(),
        "noaa".to_string(),
        "uptime".to_string(),
        "url".to_string()
    ];
    let enabled_plugins: BTreeSet<_> = config.enabled_plugins.iter().collect();

    // Check that all of the provided plugins are supported
    let mut unknown_plugins = Vec::new();
    for plugin_name in enabled_plugins.iter() {
        if !supported_plugins.contains(&plugin_name.to_string()) {
            unknown_plugins.push(plugin_name.to_string());
        }
    }

    if !unknown_plugins.is_empty() {
        return Err(format_err!(
            "{} plugin(s) not supported: {}",
            unknown_plugins.len(),
            unknown_plugins.join(", ")
        ));
    }

    // Set plugin states for each valid plugin
    let mut plugin_states = BTreeMap::new();
    for plugin_name in supported_plugins.iter() {
        plugin_states.insert(
            plugin_name.to_string(),
            if enabled_plugins.contains(plugin_name) {
                PluginState::Enabled
            } else {
                PluginState::Disabled
            },
        );
    }

    Ok(plugin_states)
}

/// Convenience function to wrap getting the state
/// for a specific plugin. Here to avoid more boilerplate.
fn plugin_state<'a>(
    states: &'a BTreeMap<String, PluginState>,
    plugin_name: &str,
) -> Result<&'a PluginState> {
    states
        .get(plugin_name)
        .ok_or_else(|| format_err!("{} plugin not found", plugin_name))
}

pub async fn run(config: ClientConfig) -> Result<()> {
    let plugin_states = validate_plugin_config(&config)?;

    // Here we optionally instantiate all supported plugins.
    //
    // Plugins are only instantiated if the user has added
    // them to the $SEABIRD_ENABLED_PLUGINS environment
    // variable.
    let plugins: Vec<Option<Box<dyn Plugin>>> = vec![
        match plugin_state(&plugin_states, "forecast")? {
            PluginState::Enabled => Some(Box::new(plugins::ForecastPlugin::new(
                config.darksky_api_key.clone().ok_or_else(|| format_err!(
                    "Missing $DARKSKY_API_KEY. Required by the enabled \"forecast\" plugin."
                ))?,
                config.maps_api_key.clone().ok_or_else(|| format_err!(
                    "Missing $MAPS_API_KEY. Required by the enabled \"forecast\" plugin."
                ))?,
            ))),
            PluginState::Disabled => None,
        },
        match plugin_state(&plugin_states, "karma")? {
            PluginState::Enabled => Some(Box::new(plugins::KarmaPlugin::new())),
            PluginState::Disabled => None,
        },
        match plugin_state(&plugin_states, "mention")? {
            PluginState::Enabled => Some(Box::new(plugins::MentionPlugin::new())),
            PluginState::Disabled => None,
        },
        match plugin_state(&plugin_states, "net_tools")? {
            PluginState::Enabled => Some(Box::new(plugins::NetToolsPlugin::new())),
            PluginState::Disabled => None,
        },
        match plugin_state(&plugin_states, "noaa")? {
            PluginState::Enabled => Some(Box::new(plugins::NoaaPlugin::new())),
            PluginState::Disabled => None,
        },
        match plugin_state(&plugin_states, "uptime")? {
            PluginState::Enabled => Some(Box::new(plugins::UptimePlugin::new())),
            PluginState::Disabled => None,
        },
        match plugin_state(&plugin_states, "url")? {
            PluginState::Enabled => Some(Box::new(plugins::UrlPlugin::new())),
            PluginState::Disabled => None,
        },
    ];

    let (mut db_client, db_connection) =
        tokio_postgres::connect(&config.db_url, tokio_postgres::NoTls).await?;

    // The connection object performs the actual communication with the
    // database, so spawn it off to run on its own.
    //
    // TODO: make sure it actually fails the bot if it exits.
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
        plugins,
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
                        .ok_or_else(|| format_err!("Prefix missing"))?
                        .nick[..]
                },
            ),
            _ => Err(format_err!("Tried to find a target for an invalid message")),
        }
    }

    pub fn sender(&self) -> Result<&str> {
        match (&self.msg.command[..], self.msg.params.len()) {
            // Only return the prefix if it came from a valid message.
            ("PRIVMSG", 2) => Ok(&self
                .msg
                .prefix
                .as_ref()
                .ok_or_else(|| format_err!("Prefix missing"))?
                .nick[..]),
            _ => Err(format_err!("Tried to find a sender for an invalid message")),
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
