use sqlx::postgres::types::PgInterval;
use std::time::Duration;

use crate::{configuration::Settings, startup::get_connection_pool};

pub async fn run_until_stopped(config: Settings) -> Result<(), anyhow::Error> {
    let pool = get_connection_pool(&config.database);
    let expiry_duration: PgInterval = config
        .application
        .idempotency_expiry
        .try_into()
        .map_err(|e| anyhow::anyhow!("{e}"))?;
    loop {
        sqlx::query!(
            "DELETE FROM idempotency WHERE created_at + $1 < now()",
            expiry_duration
        )
        .execute(&pool)
        .await?;
        tokio::time::sleep(Duration::from_secs(10)).await;
    }
}
