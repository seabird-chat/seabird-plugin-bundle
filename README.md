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
- `SEABIRD_ENABLED_PLUGINS` - comma-separated (no spaces) list of plugins (plugins not present here will not be instantiated)
- `DARKSKY_API_KEY` - needed for forecast/weather support
- `GOOGLE_MAPS_API_KEY` - needed for forecast/weather support

## Writing a new plugin

Unfortunately, writing a new plugin requires a few steps.

1. You must pick a unique name for your plugin. See `src/client.rs` `validate_plugin_config()` for the list of all existing plugin names.

2. You must create a new file in `src/plugins` that adheres to the `Plugin` async trait. See existing plugins in `src/plugins` for reference.

3. You must add your plugin to `src/plugins/mod.rs`. See existing entries in the file for reference.

4. You must add your unique plugin name to the `supported_plugins` `Vec` from step one.

5. You must add an entry to the `plugins` Vec in `src/client.rs` `run()`. See existing entries for reference.

6. At this point, you can add your new plugin to your `$SEABIRD_ENABLED_PLUGINS` environment variable, and you'll be able to use your new plugin.
