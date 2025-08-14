use std::path::Path;

use sqlx::migrate::Migrator;

use crate::prelude::*;

pub async fn run(pool: &sqlx::SqlitePool) -> Result<()> {
    Migrator::new(Path::new("./migrations"))
        .await?
        .run(pool)
        .await?;

    Ok(())
}
