# seabird-rs

## Requirements

- Rust 1.42
- Postgres

## Settings

Settings can either be included as a part of the environment or in a .env file.

- `SEABIRD_URL`
- `SEABIRD_TOKEN`
- `DATABASE_URL` - connection string for the database - can either be in connection string or url format
- `SEABIRD_ENABLED_PLUGINS` - comma-separated list of enabled plugins - if this is empty, all plugins will be loaded
- `SEABIRD_DISABLED_PLUGINS` - comma-separated list of plugins that should not be enabled
- `DARKSKY_API_KEY` - needed for forecast/weather support
- `OPENWEATHERMAP_API_KEY` - needed for forecast/weather support

## Writing a new plugin

Unfortunately, writing a new plugin requires a few steps.

1. You must pick a unique name for your plugin. See `supported_plugins` in `src/plugin.rs` `load()` for the list of all existing plugin names.
2. You must create a new file in `src/plugins` that adheres to the `Plugin` async trait. See existing plugins in `src/plugins` for reference.
3. You must add your plugin to `src/plugins/mod.rs`. See existing entries in the file for reference.
4. You must add your unique plugin name to the `supported_plugins` `Vec` from step one.
5. You must load your plugin in `src/plugin.rs` `load()`. See existing entries for reference.
