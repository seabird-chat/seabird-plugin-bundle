# seabird-rs

## Requirements

- Rust 1.42
- Postgres

## Settings

Settings can either be included as a part of the environment or in a .env file.

- `SEABIRD_HOST` - host/port to connect to
- `SEABIRD_NICK` - bot nick
- `SEABIRD_USER` - defaults to the `SEABIRD_NICK`
- `SEABIRD_NAME` - defaults to the `SEABIRD_USER`
- `SEABIRD_PASS` - optional server password
- `DATABASE_URL` - connection string for the database - can either be in connection string or url format
- `SEABIRD_COMMAND_PREFIX` - defaults to `!` - multi-character strings can be used
- `SEABIRD_ENABLED_PLUGINS` - comma-separated list of enabled plugins - if this is empty, all plugins will be loaded
- `SEABIRD_DISABLED_PLUGINS` - comma-separated list of plugins that should not be enabled
- `DARKSKY_API_KEY` - needed for forecast/weather support
- `GOOGLE_MAPS_API_KEY` - needed for forecast/weather support
- `MINECRAFT_TOPIC_UPDATE_ENABLED` - allow the Minecraft plugin to update a channel's topic with server information
- `MINECRAFT_TOPIC_UPDATE_HOSTPORT` - `hostname[:port]` for the Minecraft plugin topic updater to use
- `MINECRAFT_TOPIC_UPDATE_CHANNEL` - `#some-channel` IRC channel for the Minecraft plugin topic updater to update
- `MINECRAFT_TOPIC_UPDATE_INTERVAL_SECONDS` - The number of seconds in between each topic update. Defaults to 60 seconds.

## Writing a new plugin

Unfortunately, writing a new plugin requires a few steps.

1. You must pick a unique name for your plugin. See `supported_plugins` in `src/plugin.rs` `load()` for the list of all existing plugin names.
2. You must create a new file in `src/plugins` that adheres to the `Plugin` async trait. See existing plugins in `src/plugins` for reference.
3. You must add your plugin to `src/plugins/mod.rs`. See existing entries in the file for reference.
4. You must add your unique plugin name to the `supported_plugins` `Vec` from step one.
5. You must load your plugin in `src/plugin.rs` `load()`. See existing entries for reference.
