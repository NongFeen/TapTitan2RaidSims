use sqlx::{PgPool, postgres::PgPoolOptions};
use std::time::Duration;

pub async fn connect(database_url: &str) -> Result<PgPool, sqlx::Error> {
    let pool = PgPoolOptions::new()
        .max_connections(10)
        .acquire_timeout(Duration::from_secs(3))
        .connect(database_url)
        .await?;
    sqlx::migrate!().run(&pool).await?;
    Ok(pool)
}

/// Retries the initial connection with exponential backoff, forever --
/// a bare `connect` only ever tries once, so if the database isn't
/// reachable at the exact moment this process starts (a container race
/// during deploy, the DB still booting, or a longer outage), it would
/// permanently fall back to running with no database for the rest of this
/// process's life, requiring a manual restart. This never gives up: it
/// keeps retrying (backoff capped at 30s between attempts) until a
/// connection succeeds, so the process just waits out however long the
/// database takes to come back instead of starting degraded. Once a pool
/// is actually established, sqlx already reconnects dropped individual
/// connections on its own -- this only covers the "never got one in the
/// first place" case.
pub async fn connect_with_retry(database_url: &str) -> PgPool {
    let mut attempt = 1;
    loop {
        match connect(database_url).await {
            Ok(pool) => return pool,
            Err(error) => {
                let delay = Duration::from_secs(2u64.saturating_pow(attempt.min(5)).min(30));
                tracing::warn!(
                    ?error,
                    attempt,
                    delay_secs = delay.as_secs(),
                    "database connection failed, retrying"
                );
                tokio::time::sleep(delay).await;
                attempt += 1;
            }
        }
    }
}
