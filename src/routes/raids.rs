use std::sync::Arc;

use axum::{Json, extract::State, http::StatusCode};
use uuid::Uuid;

use crate::{
    error::AppError,
    models::app::{
        CreateSimulationJobRequest, CurrentBossView, RaidBossEventRequest, RaidEventAccepted,
    },
    services::job_service,
    state::AppState,
};

pub async fn ingest_event(
    State(state): State<Arc<AppState>>,
    Json(request): Json<RaidBossEventRequest>,
) -> Result<(StatusCode, Json<RaidEventAccepted>), AppError> {
    if request.raid_external_id.trim().is_empty()
        || request.raid_name.trim().is_empty()
        || request.event_id.trim().is_empty()
    {
        return Err(AppError::BadRequest(
            "raid_external_id, raid_name, and event_id are required".to_string(),
        ));
    }
    if request.attackable_parts.is_empty() {
        return Err(AppError::BadRequest(
            "attackable_parts cannot be empty".to_string(),
        ));
    }
    let mut unique_parts = request.attackable_parts.clone();
    unique_parts.sort();
    unique_parts.dedup();
    if unique_parts.len() != request.attackable_parts.len() {
        return Err(AppError::BadRequest(
            "attackable_parts contains duplicates".to_string(),
        ));
    }

    let mut tx = state.db()?.begin().await?;
    sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended($1, 0))")
        .bind(request.raid_external_id.trim())
        .execute(&mut *tx)
        .await?;
    if let Some((raid_id, boss_id)) = sqlx::query_as::<_, (Uuid, Uuid)>(
        "SELECT raid_id, id FROM raid_bosses WHERE external_event_id=$1",
    )
    .bind(&request.event_id)
    .fetch_optional(&mut *tx)
    .await?
    {
        tx.commit().await?;
        return Ok((
            StatusCode::OK,
            Json(RaidEventAccepted {
                raid_id,
                boss_id,
                created_jobs: Vec::new(),
                duplicate: true,
            }),
        ));
    }

    let raid_id = Uuid::new_v4();
    let (raid_id,): (Uuid,) = sqlx::query_as(
        "INSERT INTO raids (id, external_id, name) VALUES ($1,$2,$3) ON CONFLICT (external_id) DO UPDATE SET name=EXCLUDED.name, status='active', updated_at=NOW() RETURNING id",
    )
    .bind(raid_id)
    .bind(request.raid_external_id.trim())
    .bind(request.raid_name.trim())
    .fetch_one(&mut *tx)
    .await?;
    let version: i64 =
        sqlx::query_scalar("SELECT COALESCE(MAX(version),0)+1 FROM raid_bosses WHERE raid_id=$1")
            .bind(raid_id)
            .fetch_one(&mut *tx)
            .await?;
    sqlx::query("UPDATE raid_bosses SET active=FALSE, updated_at=NOW() WHERE active=TRUE")
        .execute(&mut *tx)
        .await?;
    let boss_id = Uuid::new_v4();
    sqlx::query("INSERT INTO raid_bosses (id, raid_id, external_event_id, version, boss_data, attackable_parts) VALUES ($1,$2,$3,$4,$5,$6)")
        .bind(boss_id)
        .bind(raid_id)
        .bind(request.event_id.trim())
        .bind(version)
        .bind(serde_json::to_value(request.boss_data)?)
        .bind(serde_json::to_value(request.attackable_parts)?)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;

    let player_ids: Vec<String> = sqlx::query_scalar(
        "SELECT p.player_id FROM players p WHERE p.auto_sims=TRUE AND EXISTS (SELECT 1 FROM player_stat_versions v WHERE v.player_id=p.id)",
    )
    .fetch_all(state.db()?)
    .await?;
    let mut created_jobs = Vec::new();
    for player_id in player_ids {
        let (job_id, _) = job_service::create_job(
            &state,
            CreateSimulationJobRequest {
                player_id,
                raid_boss_id: boss_id,
            },
        )
        .await?;
        created_jobs.push(job_id);
    }

    Ok((
        StatusCode::ACCEPTED,
        Json(RaidEventAccepted {
            raid_id,
            boss_id,
            created_jobs,
            duplicate: false,
        }),
    ))
}

pub async fn current(
    State(state): State<Arc<AppState>>,
) -> Result<Json<CurrentBossView>, AppError> {
    let boss = sqlx::query_as(
        "SELECT r.id AS raid_id, r.external_id AS raid_external_id, r.name AS raid_name, b.id AS boss_id, b.external_event_id AS event_id, b.version, b.boss_data, b.attackable_parts, b.spawned_at, b.updated_at FROM raid_bosses b JOIN raids r ON r.id=b.raid_id WHERE b.active=TRUE ORDER BY b.spawned_at DESC LIMIT 1",
    )
    .fetch_optional(state.db()?)
    .await?
    .ok_or_else(|| AppError::NotFound("No active raid boss".to_string()))?;
    Ok(Json(boss))
}
