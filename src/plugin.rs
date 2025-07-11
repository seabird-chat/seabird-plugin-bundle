use std::collections::BTreeSet;

use maplit::btreeset;

use crate::plugins;
use crate::prelude::*;

#[async_trait]
pub trait Plugin {
    fn new_from_env() -> Result<Self>
    where
        Self: Sized;

    fn command_metadata(&self) -> Vec<CommandMetadata> {
        Vec::new()
    }

    async fn run(self, bot: Arc<Client>) -> Result<()>;
}

pub type CommandMetadata = crate::proto::CommandMetadata;

pub type PluginHandle = tokio::task::JoinHandle<Result<()>>;

#[derive(Debug)]
pub struct PluginMetadata {
    pub handle: PluginHandle,
    pub commands: Vec<CommandMetadata>,
}

fn start_plugin<P>(bot: &Arc<Client>) -> Result<PluginMetadata>
where
    P: Plugin + Send + 'static,
{
    let plugin = P::new_from_env()?;
    let commands = plugin.command_metadata();
    let bot = bot.clone();

    // TODO: we have a Result getting lost here
    let handle = tokio::task::spawn(async move { plugin.run(bot).await });

    Ok(PluginMetadata { handle, commands })
}

pub async fn load(bot: Arc<Client>) -> Result<Vec<PluginMetadata>> {
    let supported_plugins = btreeset![
        "barista",
        "chance",
        "forecast",
        "karma",
        "mention",
        "minecraft",
        "net_tools",
        "noaa",
        "riddle",
        "scryfall",
        "introspection",
        "help",
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

    if config.plugin_enabled("barista") {
        ret.push(start_plugin::<plugins::BaristaPlugin>(&bot)?);
    }

    if config.plugin_enabled("chance") {
        ret.push(start_plugin::<plugins::ChancePlugin>(&bot)?);
    }

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

    if config.plugin_enabled("riddle") {
        ret.push(start_plugin::<plugins::RiddlePlugin>(&bot)?);
    }

    if config.plugin_enabled("scryfall") {
        ret.push(start_plugin::<plugins::ScryfallPlugin>(&bot)?);
    }

    if config.plugin_enabled("introspection") {
        ret.push(start_plugin::<plugins::IntrospectionPlugin>(&bot)?);
    }

    if config.plugin_enabled("help") {
        ret.push(start_plugin::<plugins::HelpPlugin>(&bot)?);
    }

    Ok(ret)
}
