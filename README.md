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
- `DATABASE_URL` - connection string for the database - can either be in connection string or url format
- `SEABIRD_COMMAND_PREFIX` - defaults to `!` - multi-character strings can be used
- `DARKSKY_API_KEY` - needed for forecast/weather support
- `GOOGLE_MAPS_API_KEY` - needed for forecast/weather support
