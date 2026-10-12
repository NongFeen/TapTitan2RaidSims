use sqlx::{
    PgPool,
    postgres::{PgConnectOptions, PgPoolOptions},
};
use std::{str::FromStr, time::Duration};

/// Kept well under Postgres' own limit (100 by default), shared by the API and
/// worker processes. Past this, requests wait for a free connection instead of
/// the pool opening connections Postgres refuses, which surfaces as `PoolTimedOut`.
const DEFAULT_MAX_CONNECTIONS: u32 = 20;

/// Postgres cancels anything stuck this long, so a query waiting on a lock or a
/// transaction left open by a stalled task frees its connection on its own
/// instead of holding it until the process restarts.
const SESSION_TIMEOUTS: [(&str, &str); 3] = [
    ("lock_timeout", "30s"),
    ("idle_in_transaction_session_timeout", "60s"),
    ("statement_timeout", "5min"),
];

pub async fn connect(database_url: &str) -> Result<PgPool, sqlx::Error> {
    let connect_options = PgConnectOptions::from_str(database_url)?.options(SESSION_TIMEOUTS);
    let pool = PgPoolOptions::new()
        .max_connections(max_connections())
        .acquire_timeout(Duration::from_secs(30))
        .idle_timeout(Duration::from_secs(10 * 60))
        .max_lifetime(Duration::from_secs(30 * 60))
        .connect_with(connect_options)
        .await?;
    sqlx::migrate!().run(&pool).await?;
    Ok(pool)
}

fn max_connections() -> u32 {
    std::env::var("DATABASE_MAX_CONNECTIONS")
        .ok()
        .and_then(|value| value.parse().ok())
        .filter(|&count| count > 0)
        .unwrap_or(DEFAULT_MAX_CONNECTIONS)
}

const WATCHDOG_INTERVAL: Duration = Duration::from_secs(30);
const WATCHDOG_MAX_FAILURES: u32 = 5;

/// Exits the process when the database stays unusable through this pool for
/// several checks in a row. Docker's `restart: unless-stopped` then starts it
/// again with a fresh pool, the same as a manual restart.
pub fn spawn_watchdog(pool: PgPool) {
    tokio::spawn(async move {
        let mut consecutive_failures = 0;
        loop {
            tokio::time::sleep(WATCHDOG_INTERVAL).await;
            match sqlx::query("SELECT 1").execute(&pool).await {
                Ok(_) => consecutive_failures = 0,
                Err(error) => {
                    consecutive_failures += 1;
                    tracing::warn!(?error, consecutive_failures, "database health check failed");
                    if consecutive_failures >= WATCHDOG_MAX_FAILURES {
                        tracing::error!(
                            "database unusable for {WATCHDOG_MAX_FAILURES} checks in a row; exiting so the process restarts with a fresh pool"
                        );
                        std::process::exit(1);
                    }
                }
            }
        }
    });
}

/// Retries the initial connection forever (backoff capped at 30s), so a database
/// that is still starting delays startup instead of leaving this process without
/// one. sqlx reconnects dropped connections by itself once the pool exists.
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
