use std::sync::Arc;

use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
};
use serde::Serialize;
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

pub async fn create(
    State(state): State<Arc<AppState>>,
    Json(request): Json<CreateSimulationJobRequest>,
) -> Result<(StatusCode, Json<JobAccepted>), AppError> {
    let (job_id, created) = job_service::create_job(&state, request).await?;
    Ok((StatusCode::ACCEPTED, Json(JobAccepted { job_id, created })))
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
