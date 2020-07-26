use std::collections::BTreeSet;
use std::convert::{TryFrom, TryInto};
use std::sync::Arc;

use futures::future::{select_all, FutureExt};
use tokio::sync::{broadcast, Mutex};

use crate::prelude::*;

#[derive(Clone, Debug)]
pub struct ClientConfig {
    pub inner: seabird::ClientConfig,
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
            inner: seabird::ClientConfig { url, token },
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
    inner: Mutex<seabird::Client>,
    db_client: Arc<tokio_postgres::Client>,
    broadcast: broadcast::Sender<Arc<Context>>,
}

impl Client {
    pub async fn send_message(
        &self,
        channel_id: impl Into<String>,
        text: impl Into<String>,
    ) -> Result<()> {
        self.inner
            .lock()
            .await
            .send_message(channel_id, text)
            .await?;
        Ok(())
    }

    pub async fn send_private_message(
        &self,
        user_id: impl Into<String>,
        text: impl Into<String>,
    ) -> Result<()> {
        self.inner
            .lock()
            .await
            .send_private_message(user_id, text)
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

        let seabird_client = seabird::Client::new(config.inner.clone()).await?;

        let (sender, _) = broadcast::channel(100);

        Ok(Client {
            config,
            broadcast: sender,
            db_client: Arc::new(db_client),
            inner: Mutex::new(seabird_client),
        })
    }

    async fn reader_task(
        self: &Arc<Self>,
        commands: HashMap<String, crate::plugin::CommandMetadata>,
    ) -> Result<()> {
        let mut stream = self
            .inner
            .lock()
            .await
            .inner_mut_ref()
            .stream_events(proto::StreamEventsRequest { commands })
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
        let plugin_meta = crate::plugin::load(client.clone()).await?;

        let mut plugin_tasks = Vec::new();
        let mut plugin_commands = HashMap::new();

        for meta in plugin_meta.into_iter() {
            plugin_tasks.push(meta.handle);

            for command in meta.commands.into_iter() {
                if plugin_commands.contains_key(&command.name) {
                    anyhow::bail!("Duplicate commands defined with the name {}", command.name);
                }

                plugin_commands.insert(command.name.clone(), command);
            }
        }

        // There's not a great way to do this... if anything exits, it's
        // considered an error. If they returned an error, display that,
        // otherwise, throw a generic error.
        futures::select!(
            reader_res = client.reader_task(plugin_commands).fuse() => {
                reader_res?;
                anyhow::bail!("Reader task exited early");
            },
            (task, _, _) = select_all(plugin_tasks).fuse() => {
                task??;
                anyhow::bail!("A plugin task exited early");
            },
        );
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

    pub fn as_event(&self) -> Result<Event<'_>> {
        self.try_into()
    }

    pub fn is_private(&self) -> bool {
        if let SeabirdEvent::PrivateMessage(_) = self.raw_event {
            return true;
        } else {
            return false;
        }
    }

    pub fn sender(&self) -> Option<&str> {
        match &self.raw_event {
            SeabirdEvent::Action(message) => message
                .source
                .as_ref()
                .and_then(|s| s.user.as_ref().map(|u| u.display_name.as_str())),
            SeabirdEvent::Message(message) => message
                .source
                .as_ref()
                .and_then(|s| s.user.as_ref().map(|u| u.display_name.as_str())),
            SeabirdEvent::Command(message) => message
                .source
                .as_ref()
                .and_then(|s| s.user.as_ref().map(|u| u.display_name.as_str())),
            SeabirdEvent::Mention(message) => message
                .source
                .as_ref()
                .and_then(|s| s.user.as_ref().map(|u| u.display_name.as_str())),

            // NOTE: PrivateMessage and PrivateAction are in a different format
            SeabirdEvent::PrivateAction(message) => {
                message.source.as_ref().map(|u| u.display_name.as_str())
            }
            SeabirdEvent::PrivateMessage(message) => {
                message.source.as_ref().map(|u| u.display_name.as_str())
            }

            // Seabird-sent events
            SeabirdEvent::SendMessage(message) => Some(message.sender.as_str()),
            SeabirdEvent::SendPrivateMessage(message) => Some(message.sender.as_str()),
            SeabirdEvent::PerformAction(message) => Some(message.sender.as_str()),
            SeabirdEvent::PerformPrivateAction(message) => Some(message.sender.as_str()),
        }
    }

    pub async fn mention_reply(&self, msg: &str) -> Result<()> {
        let sender = self
            .sender()
            .ok_or_else(|| format_err!("Tried to get the sender of an event without one"))?;

        // If it's a private message, we shouldn't send the prefix.
        if self.is_private() {
            self.reply(msg).await
        } else {
            self.reply(&format!("{}: {}", sender, msg)[..]).await
        }
    }

    pub async fn reply(&self, text: &str) -> Result<()> {
        match &self.raw_event {
            SeabirdEvent::Action(message) => {
                self.client
                    .send_message(
                        message
                            .source
                            .as_ref()
                            .map(|s| s.channel_id.as_str())
                            .ok_or_else(|| format_err!("message missing channel_id"))?,
                        text,
                    )
                    .await
            }
            SeabirdEvent::Message(message) => {
                self.client
                    .send_message(
                        message
                            .source
                            .as_ref()
                            .map(|s| s.channel_id.as_str())
                            .ok_or_else(|| format_err!("message missing channel_id"))?,
                        text,
                    )
                    .await
            }
            SeabirdEvent::Command(message) => {
                self.client
                    .send_message(
                        message
                            .source
                            .as_ref()
                            .map(|s| s.channel_id.as_str())
                            .ok_or_else(|| format_err!("message missing channel_id"))?,
                        text,
                    )
                    .await
            }
            SeabirdEvent::Mention(message) => {
                self.client
                    .send_message(
                        message
                            .source
                            .as_ref()
                            .map(|s| s.channel_id.as_str())
                            .ok_or_else(|| format_err!("message missing channel_id"))?,
                        text,
                    )
                    .await
            }
            SeabirdEvent::PrivateAction(message) => {
                self.client
                    .send_private_message(
                        message
                            .source
                            .as_ref()
                            .map(|u| u.id.as_str())
                            .ok_or_else(|| format_err!("message missing user_id"))?,
                        text,
                    )
                    .await
            }
            SeabirdEvent::PrivateMessage(message) => {
                self.client
                    .send_private_message(
                        message
                            .source
                            .as_ref()
                            .map(|u| u.id.as_str())
                            .ok_or_else(|| format_err!("message missing user_id"))?,
                        text,
                    )
                    .await
            }
            SeabirdEvent::SendMessage(_)
            | SeabirdEvent::SendPrivateMessage(_)
            | SeabirdEvent::PerformAction(_)
            | SeabirdEvent::PerformPrivateAction(_) => Err(format_err!("cannot reply to self")),
        }
    }

    pub fn get_db(&self) -> Arc<tokio_postgres::Client> {
        self.client.db_client.clone()
    }
}

#[non_exhaustive]
pub enum Event<'a> {
    // PRIVMSG target :msg
    Message(&'a str, &'a str),
    PrivateMessage(&'a str, &'a str),

    // PRIVMSG somewhere :!command arg
    Command(&'a str, Option<&'a str>),

    // PRIVMSG somewhere :seabird: arg
    Mention(&'a str),

    Unknown(&'a SeabirdEvent),
}

impl<'a> TryFrom<&'a Context> for Event<'a> {
    type Error = anyhow::Error;

    fn try_from(ctx: &'a Context) -> Result<Self> {
        Ok(match &ctx.raw_event {
            SeabirdEvent::Message(msg) => Event::Message(
                ctx.sender()
                    .ok_or_else(|| format_err!("event missing sender"))?,
                msg.text.as_str(),
            ),
            SeabirdEvent::PrivateMessage(msg) => Event::PrivateMessage(
                ctx.sender()
                    .ok_or_else(|| format_err!("event missing sender"))?,
                msg.text.as_str(),
            ),
            SeabirdEvent::Command(msg) => {
                let inner = msg.arg.trim();
                Event::Command(
                    msg.command.as_str(),
                    if inner.is_empty() { None } else { Some(inner) },
                )
            }
            SeabirdEvent::Mention(msg) => Event::Mention(msg.text.as_str()),

            #[allow(unreachable_patterns)]
            event => Event::Unknown(event),
        })
    }
}
