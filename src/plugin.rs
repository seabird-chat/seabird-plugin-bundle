use std::collections::BTreeSet;

use maplit::btreeset;
use tokio::sync::broadcast;

use crate::plugins;
use crate::prelude::*;
#[async_trait]
pub trait Plugin {
    fn new_from_env() -> Result<Self>
    where
        Self: Sized;

    async fn run(self, bot: Arc<Client>, stream: EventStream) -> Result<()>;
}

pub struct EventStream(Option<broadcast::Receiver<Arc<Context>>>);

impl Stream for EventStream {
    type Item = Arc<Context>;

    fn poll_next(
        mut self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Self::Item>> {
        use std::task::Poll;

        let inner = &mut self.0;

        if let Some(poller) = inner {
            // NOTE: we need to use this undocumented method or it'll never be
            // woken up.
            match poller.poll_recv(cx) {
                Poll::Ready(Err(_)) => {
                    // If the stream is done, drop the inner receiver and return
                    // a finalized stream.
                    inner.take();
                    Poll::Ready(None)
                }
                Poll::Ready(Ok(item)) => Poll::Ready(Some(item)),
                Poll::Pending => Poll::Pending,
            }
        } else {
            Poll::Ready(None)
        }
    }
}

pub type PluginHandle = tokio::task::JoinHandle<Result<()>>;

fn start_plugin<P>(bot: &Arc<Client>) -> Result<PluginHandle>
where
    P: Plugin + Send + 'static,
{
    let plugin = P::new_from_env()?;
    let bot = bot.clone();

    let stream = bot.subscribe();

    // TODO: we have a Result getting lost here
    let handle =
        tokio::task::spawn(async move { plugin.run(bot, EventStream(Some(stream))).await });

    Ok(handle)
}

pub async fn load(bot: Arc<Client>) -> Result<Vec<PluginHandle>> {
    let supported_plugins = btreeset![
        "forecast",
        "karma",
        "mention",
        "minecraft",
        "net_tools",
        "noaa",
        "introspection",
    ];

    let config = bot.get_config();

    // Check that all of the provided plugins are supported
    let mut unknown_plugins = Vec::new();
    for plugin_name in config.enabled_plugins.iter() {
        if !supported_plugins.contains(&plugin_name[..]) {
            unknown_plugins.push(plugin_name.to_string());
        }
    }

    if !unknown_plugins.is_empty() {
        anyhow::bail!(
            "{} plugin(s) not supported: {}",
            unknown_plugins.len(),
            unknown_plugins.join(", ")
        );
    }

    // If the user tries to disable a plugin that doesn't exist, it's only a
    // warning.
    for plugin_name in config.disabled_plugins.iter() {
        if !supported_plugins.contains(&plugin_name[..]) {
            warn!("Tried to disable unknown plugin {}", plugin_name);
        }
    }

    // Check that plugins are only present in one of the lists
    let enabled_set = config
        .enabled_plugins
        .iter()
        .cloned()
        .collect::<BTreeSet<_>>();
    let disabled_set = config
        .disabled_plugins
        .iter()
        .cloned()
        .collect::<BTreeSet<_>>();
    let intersection: Vec<_> = enabled_set.intersection(&disabled_set).cloned().collect();
    if !intersection.is_empty() {
        anyhow::bail!(
            "{} plugin(s) marked as both enabled and disabled: {}",
            intersection.len(),
            intersection.join(", "),
        );
    }

    // For all the plugins we know, try to enable them.
    let mut ret = Vec::new();

    // Here we optionally instantiate all supported plugins.

    if config.plugin_enabled("forecast") {
        ret.push(start_plugin::<plugins::ForecastPlugin>(&bot)?);
    }

    if config.plugin_enabled("karma") {
        ret.push(start_plugin::<plugins::KarmaPlugin>(&bot)?);
    }

    if config.plugin_enabled("mention") {
        ret.push(start_plugin::<plugins::MentionPlugin>(&bot)?);
    }

    if config.plugin_enabled("net_tools") {
        ret.push(start_plugin::<plugins::NetToolsPlugin>(&bot)?);
    }

    if config.plugin_enabled("noaa") {
        ret.push(start_plugin::<plugins::NoaaPlugin>(&bot)?);
    }

    if config.plugin_enabled("introspection") {
        ret.push(start_plugin::<plugins::IntrospectionPlugin>(&bot)?);
    }

    Ok(ret)
}
