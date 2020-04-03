use std::collections::BTreeSet;

use maplit::btreeset;
use tokio::sync::mpsc;

use crate::prelude::*;

use crate::plugins;

const PLUGIN_MESSAGE_BUF: usize = 100;

#[async_trait]
pub trait Plugin {
    fn new_from_env() -> Result<Self>
    where
        Self: Sized;

    async fn run(self, stream: Receiver<Arc<Context>>) -> Result<()>;
}

// TODO: this should be a struct type rather than a tuple, but it's so much more
// convenient to just unzip it on the other end for now.
type PluginMeta = (
    tokio::sync::mpsc::Sender<Arc<Context>>,
    tokio::task::JoinHandle<Result<()>>,
);

fn start_plugin<P>() -> Result<PluginMeta>
where
    P: Plugin + Send + 'static,
{
    let (sender, receiver) = mpsc::channel(PLUGIN_MESSAGE_BUF);
    let plugin = P::new_from_env()?;
    let handle = tokio::task::spawn(async move { plugin.run(receiver).await });
    Ok((sender, handle))
}

pub fn load(config: &ClientConfig) -> Result<Vec<PluginMeta>> {
    let supported_plugins = btreeset![
        "forecast",
        "karma",
        "mention",
        "minecraft",
        "net_tools",
        "noaa",
        "uptime",
        "url",
    ];

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
        ret.push(start_plugin::<plugins::ForecastPlugin>()?);
    }

    if config.plugin_enabled("karma") {
        ret.push(start_plugin::<plugins::KarmaPlugin>()?);
    }

    if config.plugin_enabled("minecraft") {
        ret.push(start_plugin::<plugins::MinecraftPlugin>()?);
    }

    if config.plugin_enabled("mention") {
        ret.push(start_plugin::<plugins::MentionPlugin>()?);
    }

    if config.plugin_enabled("net_tools") {
        ret.push(start_plugin::<plugins::NetToolsPlugin>()?);
    }

    if config.plugin_enabled("noaa") {
        ret.push(start_plugin::<plugins::NoaaPlugin>()?);
    }

    if config.plugin_enabled("uptime") {
        ret.push(start_plugin::<plugins::UptimePlugin>()?);
    }

    if config.plugin_enabled("url") {
        ret.push(start_plugin::<plugins::UrlPlugin>()?);
    }

    Ok(ret)
}
