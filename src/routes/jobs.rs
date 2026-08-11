use std::sync::Arc;

use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

use crate::{
    error::AppError,
    models::app::{CreateSimulationJobRequest, SimulationJobView},
    services::job_service,
    state::AppState,
};

#[derive(Serialize)]
pub struct JobAccepted {
    pub job_id: Uuid,
    pub created: bool,
}

#[derive(Deserialize)]
pub struct CreateSimulationBatchRequest {
    pub player_ids: Vec<String>,
    #[serde(default)]
    pub include_body_phase: bool,
}

#[derive(Serialize)]
pub struct SimulationBatchAccepted {
    pub batch_id: Uuid,
    pub requested: usize,
    pub queued: usize,
    pub existing: usize,
}

#[derive(Serialize)]
pub struct SimulationBatchView {
    pub batch_id: Uuid,
    pub status: &'static str,
    pub requested: i32,
    pub tracked: usize,
    pub queued: usize,
    pub pending: usize,
    pub running: usize,
    pub optimizing: usize,
    pub completed: usize,
    pub failed: usize,
    pub simulation_time_ms: u64,
    pub recommendation_time_ms: u64,
    pub combined_processing_time_ms: u64,
    pub wall_time_ms: i64,
    pub created_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
}

pub async fn create(
    State(state): State<Arc<AppState>>,
    Json(request): Json<CreateSimulationJobRequest>,
) -> Result<(StatusCode, Json<JobAccepted>), AppError> {
    let (job_id, created) = job_service::create_job(&state, request).await?;
    Ok((StatusCode::ACCEPTED, Json(JobAccepted { job_id, created })))
}

pub async fn create_batch(
    State(state): State<Arc<AppState>>,
    Json(request): Json<CreateSimulationBatchRequest>,
) -> Result<(StatusCode, Json<SimulationBatchAccepted>), AppError> {
    let mut player_ids = request
        .player_ids
        .into_iter()
        .map(|player_id| player_id.trim().to_string())
        .filter(|player_id| !player_id.is_empty())
        .collect::<Vec<_>>();
    player_ids.sort();
    player_ids.dedup();
    if player_ids.is_empty() || player_ids.len() > 100 {
        return Err(AppError::BadRequest(
            "player_ids must contain between 1 and 100 unique players".to_string(),
        ));
    }

    let batch_id = Uuid::new_v4();
    sqlx::query("INSERT INTO simulation_batches (id, include_body_phase, requested_count) VALUES ($1,$2,$3)")
        .bind(batch_id)
        .bind(request.include_body_phase)
        .bind(player_ids.len() as i32)
        .execute(state.db()?)
        .await?;

    let mut queued = 0usize;
    let mut existing = 0usize;
    for player_id in &player_ids {
        let (job_id, created) = job_service::create_job(
            &state,
            CreateSimulationJobRequest {
                player_id: player_id.clone(),
                include_body_phase: request.include_body_phase,
            },
        )
        .await?;
        sqlx::query("INSERT INTO simulation_batch_jobs (batch_id, simulation_job_id, created_for_batch) VALUES ($1,$2,$3) ON CONFLICT DO NOTHING")
            .bind(batch_id)
            .bind(job_id)
            .bind(created)
            .execute(state.db()?)
            .await?;
        if created {
            queued += 1;
        } else {
            existing += 1;
        }
    }

    Ok((
        StatusCode::ACCEPTED,
        Json(SimulationBatchAccepted {
            batch_id,
            requested: player_ids.len(),
            queued,
            existing,
        }),
    ))
}

pub async fn get_batch(
    State(state): State<Arc<AppState>>,
    Path(batch_id): Path<Uuid>,
) -> Result<Json<SimulationBatchView>, AppError> {
    let batch: Option<(i32, DateTime<Utc>)> =
        sqlx::query_as("SELECT requested_count, created_at FROM simulation_batches WHERE id=$1")
            .bind(batch_id)
            .fetch_optional(state.db()?)
            .await?;
    let Some((requested, created_at)) = batch else {
        return Err(AppError::NotFound("Simulation batch not found".to_string()));
    };
    let jobs: Vec<(bool, String, Option<Value>, Option<DateTime<Utc>>)> = sqlx::query_as(
        "SELECT b.created_for_batch, j.status, j.result, j.completed_at FROM simulation_batch_jobs b JOIN simulation_jobs j ON j.id=b.simulation_job_id WHERE b.batch_id=$1",
    )
    .bind(batch_id)
    .fetch_all(state.db()?)
    .await?;

    let count_status = |status: &str| jobs.iter().filter(|job| job.1 == status).count();
    let metric = |key: &str| {
        jobs.iter()
            .filter(|job| job.0)
            .filter_map(|job| job.2.as_ref()?.get(key)?.as_u64())
            .sum::<u64>()
    };
    let pending = count_status("pending");
    let running = count_status("running");
    let optimizing = count_status("optimizing");
    let completed = count_status("completed");
    let failed = count_status("failed");
    let finished = !jobs.is_empty() && completed + failed == jobs.len();
    let completed_at = finished
        .then(|| jobs.iter().filter_map(|job| job.3).max())
        .flatten();
    let wall_end = completed_at.unwrap_or_else(Utc::now);
    let status = if !finished {
        "running"
    } else if failed > 0 {
        "completed_with_failures"
    } else {
        "completed"
    };
    let simulation_time_ms = metric("simulation_duration_ms");
    let recommendation_time_ms = metric("recommendation_duration_ms");

    Ok(Json(SimulationBatchView {
        batch_id,
        status,
        requested,
        tracked: jobs.len(),
        queued: jobs.iter().filter(|job| job.0).count(),
        pending,
        running,
        optimizing,
        completed,
        failed,
        simulation_time_ms,
        recommendation_time_ms,
        combined_processing_time_ms: simulation_time_ms + recommendation_time_ms,
        wall_time_ms: (wall_end - created_at).num_milliseconds().max(0),
        created_at,
        completed_at,
    }))
}

pub async fn get(
    State(state): State<Arc<AppState>>,
    Path(job_id): Path<Uuid>,
) -> Result<Json<SimulationJobView>, AppError> {
    Ok(Json(job_service::get_job(&state, job_id).await?))
}

pub async fn list_for_player(
    State(state): State<Arc<AppState>>,
    Path(player_id): Path<String>,
) -> Result<Json<Vec<SimulationJobView>>, AppError> {
    Ok(Json(
        job_service::list_player_jobs(&state, &player_id).await?,
    ))
}

pub async fn retry(
    State(state): State<Arc<AppState>>,
    Path(job_id): Path<Uuid>,
) -> Result<(StatusCode, Json<JobAccepted>), AppError> {
    job_service::retry_job(&state, job_id).await?;
    Ok((
        StatusCode::ACCEPTED,
        Json(JobAccepted {
            job_id,
            created: false,
        }),
    ))
}
