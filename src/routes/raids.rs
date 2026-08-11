use std::sync::Arc;

use axum::{Json, extract::State, http::StatusCode};

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
    mut boss_data: Boss,
    attackable_parts: Vec<BossPartName>,
    trigger_simulations: bool,
) -> Result<(StatusCode, Json<RaidEventAccepted>), AppError> {
    boss_data.sync_part_states_from_current_values();

    let mut tx = state.db()?.begin().await?;
    sqlx::query("SELECT pg_advisory_xact_lock(721934761)")
        .execute(&mut *tx)
        .await?;

    let current_boss_version: i64 = sqlx::query_scalar("INSERT INTO current_boss (singleton, version, boss_data, attackable_parts) VALUES (TRUE,1,$1,$2) ON CONFLICT (singleton) DO UPDATE SET version=current_boss.version+1, boss_data=EXCLUDED.boss_data, attackable_parts=EXCLUDED.attackable_parts, updated_at=NOW() RETURNING version")
        .bind(serde_json::to_value(boss_data)?)
        .bind(serde_json::to_value(attackable_parts)?)
        .fetch_one(&mut *tx)
        .await?;
    tx.commit().await?;

    job_service::spawn_old_job_cleanup(Arc::clone(state), current_boss_version);

    let mut created_jobs = Vec::new();
    if trigger_simulations {
        let player_ids: Vec<String> = sqlx::query_scalar(
            "SELECT p.player_id FROM players p WHERE p.auto_sims=TRUE AND EXISTS (SELECT 1 FROM player_stats s WHERE s.player_id=p.id)",
        )
        .fetch_all(state.db()?)
        .await?;
        for player_id in player_ids {
            let (job_id, _) = job_service::create_job(
                state,
                CreateSimulationJobRequest {
                    player_id,
                    include_body_phase: false,
                },
            )
            .await?;
            created_jobs.push(job_id);
        }
    }

    Ok((
        StatusCode::ACCEPTED,
        Json(RaidEventAccepted {
            status: "accepted",
            message: if trigger_simulations {
                format!(
                    "Current boss was replaced, old simulations were scheduled for background cleanup, and {} new job(s) were queued",
                    created_jobs.len()
                )
            } else {
                "Current boss was replaced and old simulations were scheduled for background cleanup; no simulations were started".to_string()
            },
            simulations_triggered: trigger_simulations,
            deleted_jobs: 0,
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
        "SELECT boss_data, attackable_parts, created_at, updated_at FROM current_boss WHERE singleton=TRUE",
    )
    .fetch_optional(state.db()?)
    .await?
    .ok_or_else(|| AppError::NotFound("No current raid boss".to_string()))?;
    Ok(Json(boss))
}
