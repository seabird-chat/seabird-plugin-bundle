use std::path::Path;

use sqlx::migrate::Migrator;

use crate::prelude::*;

pub async fn run(pool: &sqlx::SqlitePool) -> Result<()> {
    sqlx::migrate!("./migrations").run(pool).await?;

    Ok(())
}
