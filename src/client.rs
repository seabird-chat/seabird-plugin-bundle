use std::collections::BTreeSet;
use std::sync::Arc;

use futures::future::{try_join_all, TryFutureExt};
use http::Uri;
use tokio::stream::StreamExt;
use tokio::sync::{broadcast, Mutex};
use tonic::transport::{Channel, ClientTlsConfig};

use crate::prelude::*;
use crate::proto::seabird_client::SeabirdClient;

#[derive(Clone, Debug)]
pub struct ClientConfig {
    pub url: String,
    pub token: String,

    pub enabled_plugins: BTreeSet<String>,
    pub disabled_plugins: BTreeSet<String>,

    pub db_url: String,
}

impl ClientConfig {
    pub fn new(
        url: String,
        token: String,
        db_url: String,
        enabled_plugins: BTreeSet<String>,
        disabled_plugins: BTreeSet<String>,
    ) -> Self {
        ClientConfig {
            url,
            token,
            db_url,
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

// Client represents the running bot.
#[derive(Debug)]
pub struct Client {
    config: ClientConfig,
    identity: Option<proto::Identity>,
    inner: Mutex<SeabirdClient<tonic::transport::Channel>>,
    db_client: Arc<tokio_postgres::Client>,
    broadcast: broadcast::Sender<Arc<Context>>,
}

impl Client {
    pub async fn send_message(&self, target: &str, message: &str) -> Result<()> {
        self.inner
            .lock()
            .await
            .send_message(proto::SendMessageRequest {
                identity: self.identity.clone(),
                target: target.to_string(),
                message: message.to_string(),
            })
            .await?;
        Ok(())
    }

    pub fn subscribe(&self) -> broadcast::Receiver<Arc<Context>> {
        self.broadcast.subscribe()
    }

    pub fn get_config(&self) -> &ClientConfig {
        &self.config
    }
}

impl Client {
    pub async fn new(config: ClientConfig) -> Result<Self> {
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

        let identity = Some(proto::Identity {
            auth_method: Some(proto::identity::AuthMethod::Token(config.token.clone())),
        });

        let uri: Uri = config.url.parse().context("failed to parse SEABIRD_URL")?;
        let mut channel_builder = Channel::builder(uri.clone());

        match uri.scheme_str() {
            None | Some("https") => {
                println!("Enabling tls");
                channel_builder = channel_builder
                    .tls_config(ClientTlsConfig::new().domain_name(uri.host().unwrap()));
            }
            _ => {}
        }

        let channel = channel_builder
            .connect()
            .await
            .context("Failed to connect to seabird")?;

        let seabird_client = crate::proto::seabird_client::SeabirdClient::new(channel);

        let (sender, _) = broadcast::channel(100);

        Ok(Client {
            config,
            identity,
            broadcast: sender,
            db_client: Arc::new(db_client),
            inner: Mutex::new(seabird_client),
        })
    }

    async fn reader_task(self: &Arc<Self>) -> Result<()> {
        let mut stream = self
            .inner
            .lock()
            .await
            .stream_events(proto::StreamEventsRequest {
                identity: self.identity.clone(),
                commands: HashMap::new(),
            })
            .await?
            .into_inner();

        while let Some(event) = stream.next().await.transpose()? {
            info!("<-- {:?}", event);

            // Create an Arc out of our context to make it easier for async
            // plugins.
            if let Some(inner) = event.inner {
                let ctx = Arc::new(Context::new(self.clone(), inner));

                self.broadcast
                    .send(ctx)
                    .map_err(|_| format_err!("failed to broadcast incoming event"))?;
            } else {
                warn!("Got SeabirdEvent missing an inner");
            }
        }

        Err(format_err!("reader_task exited early"))
    }

    pub async fn run(self) -> Result<()> {
        let client = Arc::new(self);

        // TODO: it's unfortunately easiest to load plugins in run, even though
        // it would make more sense in new().
        let plugin_tasks = crate::plugin::load(client.clone()).await?;

        let (_client, plugins) = tokio::try_join!(
            client.reader_task(),
            try_join_all(plugin_tasks).map_err(|e| e.into()),
        )?;

        // Ensure no plugins exited
        for plugin in plugins {
            plugin?;
        }

        Ok(())
    }
}

#[derive(Clone, Debug)]
pub struct Context {
    pub raw_event: SeabirdEvent,

    client: Arc<Client>,
}

impl Context {
    fn new(client: Arc<Client>, raw_event: SeabirdEvent) -> Self {
        Context { raw_event, client }
    }

    pub fn as_event(&self) -> Event<'_> {
        (&self.raw_event).into()
    }

    pub fn reply_target(&self) -> Option<&str> {
        match &self.raw_event {
            SeabirdEvent::Message(message) => Some(message.reply_to.as_str()),
            SeabirdEvent::PrivateMessage(message) => Some(message.reply_to.as_str()),
            SeabirdEvent::Command(message) => Some(message.reply_to.as_str()),
            SeabirdEvent::Mention(message) => Some(message.reply_to.as_str()),
        }
    }

    pub fn sender(&self) -> Option<&str> {
        match &self.raw_event {
            SeabirdEvent::Message(message) => Some(message.sender.as_str()),
            SeabirdEvent::PrivateMessage(message) => Some(message.sender.as_str()),
            SeabirdEvent::Command(message) => Some(message.sender.as_str()),
            SeabirdEvent::Mention(message) => Some(message.sender.as_str()),
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
        //
        // TODO: switch to matching on PrivateMessage
        if target == sender {
            self.reply(msg).await
        } else {
            self.reply(&format!("{}: {}", sender, msg)[..]).await
        }
    }

    pub async fn reply(&self, msg: &str) -> Result<()> {
        self.send_message(
            self.reply_target()
                .ok_or_else(|| format_err!("Tried to reply to an event without a targets"))?,
            msg,
        )
        .await
    }

    pub async fn send_message(&self, target: &str, message: &str) -> Result<()> {
        self.client.send_message(target, message).await
    }

    pub fn get_db(&self) -> Arc<tokio_postgres::Client> {
        self.client.db_client.clone()
    }
}
