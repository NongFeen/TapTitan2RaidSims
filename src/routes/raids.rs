use std::sync::Arc;

use axum::{Json, extract::State, http::StatusCode};

use crate::{
    error::AppError,
    models::{
        app::{
            CreateSimulationJobRequest, CurrentBossUpdateRequest, CurrentBossView,
            LiveAttackBossView, RaidEventAccepted,
        },
        boss::{Boss, BossPartName},
    },
    services::{job_service, raid_event_service},
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
    boss_data.global_raid_modifier_amount = None;
    boss_data.curse_damage_per_curse = 0.06;
    boss_data.sync_part_states_from_current_values();

    let mut tx = state.db()?.begin().await?;
    sqlx::query("SELECT pg_advisory_xact_lock(721934761)")
        .execute(&mut *tx)
        .await?;

    let current_boss_version: i64 = sqlx::query_scalar("INSERT INTO current_boss (singleton, version, boss_data, attackable_parts) VALUES (TRUE,1,$1,$2) ON CONFLICT (singleton) DO UPDATE SET version=current_boss.version+1, boss_data=EXCLUDED.boss_data, attackable_parts=EXCLUDED.attackable_parts, source_raid_id=NULL, source_titan_index=NULL, source_enemy_id=NULL, updated_at=NOW() RETURNING version")
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
                    "Sims boss data was replaced, old simulations were scheduled for background cleanup, and {} new job(s) were queued",
                    created_jobs.len()
                )
            } else {
                "Sims boss data was replaced and old simulations were scheduled for background cleanup; no simulations were started".to_string()
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
    .ok_or_else(|| AppError::NotFound("No sims boss data".to_string()))?;
    Ok(Json(boss))
}

pub async fn live_from_attack(
    State(state): State<Arc<AppState>>,
) -> Result<Json<LiveAttackBossView>, AppError> {
    let mut boss = state.live_attack_boss.read().await.clone().ok_or_else(|| {
        AppError::NotFound(
            "No live current boss has been received from an attack event".to_string(),
        )
    })?;
    if let Some(db) = state.optional_db() {
        let display_metadata: Option<(serde_json::Value, serde_json::Value)> = sqlx::query_as(
            "SELECT raid_data,titan_targets FROM raid_current_state WHERE raid_id=$1 AND raid_data IS NOT NULL AND titan_targets IS NOT NULL",
        )
        .bind(boss.raid_id)
        .fetch_optional(db)
        .await?;
        boss.display_parts = display_metadata
            .as_ref()
            .map(|(raid_data, titan_targets)| {
                raid_event_service::live_boss_display_parts(
                    &boss.boss_data,
                    raid_data,
                    titan_targets,
                )
            })
            .transpose()?
            .flatten();
    }
    Ok(Json(boss))
}
