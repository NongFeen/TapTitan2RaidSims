use std::sync::Arc;

use axum::{Json, extract::State, http::StatusCode};
use uuid::Uuid;

use crate::{
    error::AppError,
    models::{
        app::{
            CreateSimulationJobRequest, CurrentBossUpdateRequest, CurrentBossView,
            RaidEventAccepted,
        },
        boss::{Boss, BossPartName},
    },
    services::job_service,
    state::AppState,
};

pub async fn update_current_boss(
    State(state): State<Arc<AppState>>,
    Json(request): Json<CurrentBossUpdateRequest>,
) -> Result<(StatusCode, Json<RaidEventAccepted>), AppError> {
    validate_attackable_parts(&request.attackable_parts)?;
    replace_current_boss(
        &state,
        request.boss_data,
        request.attackable_parts,
        request.run_sims,
    )
    .await
}

async fn replace_current_boss(
    state: &Arc<AppState>,
    boss_data: Boss,
    attackable_parts: Vec<BossPartName>,
    trigger_simulations: bool,
) -> Result<(StatusCode, Json<RaidEventAccepted>), AppError> {
    let mut tx = state.db()?.begin().await?;
    sqlx::query("SELECT pg_advisory_xact_lock(721934761)")
        .execute(&mut *tx)
        .await?;

    let deleted_jobs = sqlx::query("DELETE FROM simulation_jobs")
        .execute(&mut *tx)
        .await?
        .rows_affected();
    sqlx::query("DELETE FROM raid_bosses")
        .execute(&mut *tx)
        .await?;
    sqlx::query("DELETE FROM raids").execute(&mut *tx).await?;

    let raid_id = Uuid::new_v4();
    sqlx::query("INSERT INTO raids (id, external_id, name) VALUES ($1,$2,$3)")
        .bind(raid_id)
        .bind("current")
        .bind("Current Raid")
        .execute(&mut *tx)
        .await?;
    let boss_id = Uuid::new_v4();
    let event_id = format!("current-{}", Uuid::new_v4());
    sqlx::query("INSERT INTO raid_bosses (id, raid_id, external_event_id, version, boss_data, attackable_parts, active) VALUES ($1,$2,$3,1,$4,$5,TRUE)")
        .bind(boss_id)
        .bind(raid_id)
        .bind(event_id)
        .bind(serde_json::to_value(boss_data)?)
        .bind(serde_json::to_value(attackable_parts)?)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;

    let mut created_jobs = Vec::new();
    if trigger_simulations {
        let player_ids: Vec<String> = sqlx::query_scalar(
            "SELECT p.player_id FROM players p WHERE p.auto_sims=TRUE AND EXISTS (SELECT 1 FROM player_stat_versions v WHERE v.player_id=p.id)",
        )
        .fetch_all(state.db()?)
        .await?;
        for player_id in player_ids {
            let (job_id, _) =
                job_service::create_job(state, CreateSimulationJobRequest { player_id }).await?;
            created_jobs.push(job_id);
        }
    }

    Ok((
        StatusCode::ACCEPTED,
        Json(RaidEventAccepted {
            status: "accepted",
            message: if trigger_simulations {
                format!(
                    "Current boss was replaced, {deleted_jobs} old job(s) were deleted, and {} new job(s) were queued",
                    created_jobs.len()
                )
            } else {
                format!(
                    "Current boss was replaced and {deleted_jobs} old simulation job(s) were deleted; no simulations were started"
                )
            },
            simulations_triggered: trigger_simulations,
            deleted_jobs,
            created_jobs,
        }),
    ))
}

fn validate_attackable_parts(attackable_parts: &[BossPartName]) -> Result<(), AppError> {
    if attackable_parts.is_empty() {
        return Err(AppError::BadRequest(
            "attackable_parts cannot be empty".to_string(),
        ));
    }
    let mut unique_parts = attackable_parts.to_vec();
    unique_parts.sort();
    unique_parts.dedup();
    if unique_parts.len() != attackable_parts.len() {
        return Err(AppError::BadRequest(
            "attackable_parts contains duplicates".to_string(),
        ));
    }
    Ok(())
}

pub async fn current(
    State(state): State<Arc<AppState>>,
) -> Result<Json<CurrentBossView>, AppError> {
    let boss = sqlx::query_as(
        "SELECT b.boss_data, b.attackable_parts, b.spawned_at, b.updated_at FROM raid_bosses b WHERE b.active=TRUE LIMIT 1",
    )
    .fetch_optional(state.db()?)
    .await?
    .ok_or_else(|| AppError::NotFound("No current raid boss".to_string()))?;
    Ok(Json(boss))
}
