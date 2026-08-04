use sqlx::PgPool;
use std::sync::Arc;
use tokio::sync::Semaphore;
use uuid::Uuid;

use crate::error::AppError;

#[derive(Clone)]
pub struct AppState {
    db: Option<PgPool>,
    pub simulation_slots: Arc<Semaphore>,
    pub recommendation_slots: Arc<Semaphore>,
    pub internal_api_key: Arc<str>,
}

impl AppState {
    pub fn new(
        db: Option<PgPool>,
        simulation_concurrency: usize,
        internal_api_key: String,
    ) -> Self {
        Self {
            db,
            simulation_slots: Arc::new(Semaphore::new(simulation_concurrency)),
            recommendation_slots: Arc::new(Semaphore::new(1)),
            internal_api_key: Arc::from(internal_api_key),
        }
    }

    pub fn db(&self) -> Result<&PgPool, AppError> {
        self.db.as_ref().ok_or(AppError::DatabaseUnavailable)
    }

    pub fn optional_db(&self) -> Option<&PgPool> {
        self.db.as_ref()
    }

    pub async fn recover_pending_jobs(self: &Arc<Self>) {
        let Some(db) = self.db.as_ref() else {
            return;
        };
        let recovered: Result<Vec<Uuid>, _> = sqlx::query_scalar(
            "UPDATE simulation_jobs SET status='pending', updated_at=NOW() WHERE status IN ('running','optimizing') RETURNING id",
        )
        .fetch_all(db)
        .await;
        let pending: Result<Vec<Uuid>, _> = sqlx::query_scalar(
            "SELECT id FROM simulation_jobs WHERE status='pending' ORDER BY created_at",
        )
        .fetch_all(db)
        .await;
        match (recovered, pending) {
            (Ok(recovered), Ok(pending)) => {
                tracing::info!(
                    recovered = recovered.len(),
                    queued = pending.len(),
                    "recovering simulation queue"
                );
                for job_id in pending {
                    crate::services::job_service::spawn_job(Arc::clone(self), job_id);
                }
            }
            (Err(error), _) | (_, Err(error)) => {
                tracing::error!(?error, "failed to recover interrupted simulation jobs");
            }
        }
    }
}
