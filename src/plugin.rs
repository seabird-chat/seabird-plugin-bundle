use std::collections::BTreeSet;
use std::sync::Arc;

use async_trait::async_trait;
use maplit::btreeset;

use crate::client::{ClientConfig, Context};
use crate::error::Result;
use crate::plugins;

#[async_trait]
pub trait Plugin: Sync + Send {
    fn new_from_env() -> Result<Self>
    where
        Self: Sized;

    async fn handle_connect(&self) -> Result<()> {
        Ok(())
    }

    async fn handle_message(&self, _ctx: &Arc<Context>) -> Result<()> {
        Ok(())
    }
}

pub fn load(config: &ClientConfig) -> Result<Vec<Box<dyn Plugin>>> {
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
    let mut ret: Vec<Box<dyn Plugin>> = Vec::new();

    // Here we optionally instantiate all supported plugins.

    if config.plugin_enabled("forecast") {
        ret.push(Box::new(Arc::<plugins::ForecastPlugin>::new_from_env()?));
    }

    if config.plugin_enabled("karma") {
        ret.push(Box::new(plugins::KarmaPlugin::new_from_env()?));
    }

    if config.plugin_enabled("minecraft") {
        ret.push(Box::new(plugins::MinecraftPlugin::new()));
    }

    if config.plugin_enabled("mention") {
        ret.push(Box::new(plugins::MentionPlugin::new_from_env()?));
    }

    if config.plugin_enabled("net_tools") {
        ret.push(Box::new(plugins::NetToolsPlugin::new_from_env()?));
    }

    if config.plugin_enabled("noaa") {
        ret.push(Box::new(Arc::<plugins::NoaaPlugin>::new_from_env()?));
    }

    if config.plugin_enabled("uptime") {
        ret.push(Box::new(plugins::UptimePlugin::new_from_env()?));
    }

    if config.plugin_enabled("url") {
        ret.push(Box::new(Arc::<plugins::UrlPlugin>::new_from_env()?));
    }

    Ok(ret)
}
