use std::sync::Arc;

use axum::{Json, extract::State, http::StatusCode};

use chrono::Utc;

use crate::{
    error::AppError,
    models::{
        app::{
            CreateSimulationJobRequest, CurrentBossUpdateRequest, CurrentBossView,
            LiveAttackBossView, LiveAttackingPlayer, RaidEventAccepted,
        },
        boss::{Boss, BossPartName},
    },
    services::{boss_repo, job_service, raid_event_service},
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

    let current_boss_version = boss_repo::store(
        &mut tx,
        boss_repo::BossWrite {
            boss: &boss_data,
            attackable_parts: Some(&attackable_parts),
            source_raid_id: None,
            source_titan_index: None,
            source_enemy_id: None,
            bump_version: true,
        },
    )
    .await?;
    tx.commit().await?;

    job_service::spawn_old_job_cleanup(Arc::clone(state), current_boss_version);

    let mut created_jobs = Vec::new();
    if trigger_simulations {
        let player_ids: Vec<String> = sqlx::query_scalar(
            "SELECT p.player_id FROM players p WHERE p.auto_sims=TRUE AND EXISTS (SELECT 1 FROM player_stats s WHERE s.player_id=p.player_id)",
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
    let loaded = boss_repo::load(state.db()?)
        .await?
        .ok_or_else(|| AppError::NotFound("No sims boss data".to_string()))?;
    Ok(Json(CurrentBossView {
        boss_data: serde_json::to_value(&loaded.boss)?,
        attackable_parts: serde_json::to_value(&loaded.attackable_parts)?,
        created_at: loaded.created_at,
        updated_at: loaded.updated_at,
    }))
}

pub async fn live_from_attack(
    State(state): State<Arc<AppState>>,
) -> Result<Json<LiveAttackBossView>, AppError> {
    let cached = state.live_attack_boss.read().await.clone();
    let (mut boss, from_cache) = match cached {
        Some(boss) => (boss, true),
        None => {
            let fallback = build_live_boss_fallback(&state).await?;
            let boss = fallback.ok_or_else(|| {
                AppError::NotFound(
                    "No live current boss has been received from an attack event".to_string(),
                )
            })?;
            (boss, false)
        }
    };
    // The DB-reconstructed fallback already computes display_parts directly
    // from normalized data; only the real in-memory path needs this extra
    // raw-JSON enrichment step (it doesn't carry titan-target info itself).
    if from_cache {
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
    }
    Ok(Json(boss))
}

pub async fn live_attacking_players(
    State(state): State<Arc<AppState>>,
) -> Json<Vec<LiveAttackingPlayer>> {
    let mut players = state.live_attacking_players.write().await;
    let now = Utc::now();
    players.retain(|_, player| !player.is_expired(now));
    let mut list: Vec<LiveAttackingPlayer> = players.values().cloned().collect();
    list.sort_by_key(|player| player.started_at);
    Json(list)
}

/// Reconstructs a live-boss view from the persisted `current_boss` tables
/// when no attack event has been received yet since the backend last
/// restarted -- see `raid_event_service::live_boss_from_persisted`.
async fn build_live_boss_fallback(
    state: &Arc<AppState>,
) -> Result<Option<LiveAttackBossView>, AppError> {
    let Some(db) = state.optional_db() else {
        return Ok(None);
    };
    let Some(loaded) = boss_repo::load(db).await? else {
        return Ok(None);
    };
    let Some(raid_id) = loaded.source_raid_id else {
        return Ok(None);
    };
    let clan_code: Option<String> =
        sqlx::query_scalar("SELECT clan_code FROM raid_current_state WHERE raid_id=$1")
            .bind(raid_id)
            .fetch_optional(db)
            .await?;
    let cycle: Option<i32> = sqlx::query_scalar(
        "SELECT cycle FROM raid_attack_logs WHERE raid_id=$1 ORDER BY attack_datetime DESC LIMIT 1",
    )
    .bind(raid_id)
    .fetch_optional(db)
    .await?;
    Ok(raid_event_service::live_boss_from_persisted(
        &loaded,
        clan_code.unwrap_or_default(),
        cycle.unwrap_or_default(),
    ))
}
