use std::vec::Vec;

use futures::future::{self, FutureExt};
use sqlx::error::BoxDynError;
use sqlx::migrate::{Migration, MigrationSource, Migrator};

use crate::prelude::*;

pub async fn run(pool: &sqlx::PgPool) -> Result<()> {
    Migrator::new(IntegratedMigrations::new())
        .await?
        .run(pool)
        .await?;

    Ok(())
}

#[derive(Debug)]
struct IntegratedMigrations(Vec<Migration>);

impl IntegratedMigrations {
    fn new() -> Self {
        IntegratedMigrations(vec![
            Migration::new(1, "karma".into(), include_str!("1_karma.sql").into()),
            Migration::new(2, "bucket".into(), include_str!("2_bucket.sql").into()),
            Migration::new(3, "noaa".into(), include_str!("3_noaa.sql").into()),
            Migration::new(4, "forecast".into(), include_str!("4_forecast.sql").into()),
        ])
    }
}

impl MigrationSource<'static> for IntegratedMigrations {
    fn resolve(
        self,
    ) -> future::BoxFuture<'static, std::result::Result<Vec<Migration>, BoxDynError>> {
        future::ok(self.0).boxed()
    }
}
