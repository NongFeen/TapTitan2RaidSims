use std::sync::Arc;

use uuid::Uuid;

use crate::{
    error::AppError,
    models::{
        app::{CreateSimulationJobRequest, SimulationJobView},
        boss::{Boss, BossPartName},
        cards::CardName,
        player_raid_data::PlayerRaidData,
        sim_payload::SimPayLoad,
    },
    services::taptitan::{
        recommendation::optimize_decks,
        sim_service::{SimRunResult, SimService},
    },
    state::AppState,
};

const SIMULATOR_VERSION: &str = env!("CARGO_PKG_VERSION");

pub async fn create_job(
    state: &Arc<AppState>,
    request: CreateSimulationJobRequest,
) -> Result<(Uuid, bool), AppError> {
    let internal_player_id: Uuid = sqlx::query_scalar("SELECT id FROM players WHERE player_id=$1")
        .bind(request.player_id.trim())
        .fetch_optional(state.db()?)
        .await?
        .ok_or_else(|| AppError::NotFound("Player not found".to_string()))?;
    let row: Option<(Uuid, i64, serde_json::Value)> = sqlx::query_as(
        "SELECT id, version, stats FROM player_stat_versions WHERE player_id = $1 ORDER BY version DESC LIMIT 1",
    )
    .bind(internal_player_id)
    .fetch_optional(state.db()?)
    .await?;
    let (stats_version_id, stats_version, stats_json) =
        row.ok_or_else(|| AppError::NotFound("Player has no stored stats".to_string()))?;
    let player_stats: PlayerRaidData = serde_json::from_value(stats_json)?;

    let boss_row: Option<(serde_json::Value, serde_json::Value, i64)> = sqlx::query_as(
        "SELECT boss_data, attackable_parts, version FROM raid_bosses WHERE id = $1",
    )
    .bind(request.raid_boss_id)
    .fetch_optional(state.db()?)
    .await?;
    let (boss_json, attackable_json, boss_version) =
        boss_row.ok_or_else(|| AppError::NotFound("Raid boss not found".to_string()))?;
    let boss_data: Boss = serde_json::from_value(boss_json)?;
    let attackable_part: Vec<BossPartName> = serde_json::from_value(attackable_json)?;
    let usable_card: Vec<CardName> = player_stats
        .card_list
        .iter()
        .map(|card| card.card_id)
        .collect();

    let payload = SimPayLoad {
        player_raid_data: player_stats,
        boss_data,
        attackable_part,
        usable_card,
    };
    let deduplication_key = format!(
        "{}:{}:{}:{}:{}",
        internal_player_id, stats_version, request.raid_boss_id, boss_version, SIMULATOR_VERSION
    );
    let job_id = Uuid::new_v4();
    let inserted: Option<(Uuid,)> = sqlx::query_as(
        "INSERT INTO simulation_jobs (id, player_id, player_stat_version_id, raid_boss_id, deduplication_key, simulator_version, status, payload) VALUES ($1,$2,$3,$4,$5,$6,'pending',$7) ON CONFLICT (deduplication_key) DO NOTHING RETURNING id",
    )
    .bind(job_id)
    .bind(internal_player_id)
    .bind(stats_version_id)
    .bind(request.raid_boss_id)
    .bind(&deduplication_key)
    .bind(SIMULATOR_VERSION)
    .bind(serde_json::to_value(payload)?)
    .fetch_optional(state.db()?)
    .await?;

    let (id, created) = if let Some((id,)) = inserted {
        (id, true)
    } else {
        let (id,): (Uuid,) =
            sqlx::query_as("SELECT id FROM simulation_jobs WHERE deduplication_key = $1")
                .bind(&deduplication_key)
                .fetch_one(state.db()?)
                .await?;
        (id, false)
    };
    if created {
        spawn_job(Arc::clone(state), id);
    }
    Ok((id, created))
}

pub fn spawn_job(state: Arc<AppState>, job_id: Uuid) {
    tokio::spawn(async move {
        if let Err(error) = process_job(&state, job_id).await {
            tracing::error!(%job_id, ?error, "simulation job failed");
            if let Ok(db) = state.db() {
                let _ = sqlx::query("UPDATE simulation_jobs SET status='failed', error_message=$2, completed_at=NOW(), updated_at=NOW() WHERE id=$1")
                    .bind(job_id)
                    .bind(error.to_string())
                    .execute(db)
                    .await;
            }
        }
    });
}

async fn process_job(state: &Arc<AppState>, job_id: Uuid) -> Result<(), AppError> {
    let _permit = state
        .simulation_slots
        .clone()
        .acquire_owned()
        .await
        .map_err(|_| AppError::Internal("Simulation worker is shutting down".to_string()))?;

    let payload: Option<(serde_json::Value,)> = sqlx::query_as(
        "UPDATE simulation_jobs SET status='running', attempts=attempts+1, started_at=NOW(), error_message=NULL, updated_at=NOW() WHERE id=$1 AND status IN ('pending','failed') RETURNING payload",
    )
    .bind(job_id)
    .fetch_optional(state.db()?)
    .await?;
    let Some((payload_json,)) = payload else {
        return Ok(());
    };
    let payload: SimPayLoad = serde_json::from_value(payload_json)?;
    let result = tokio::task::spawn_blocking(move || SimService::run_simulation(payload))
        .await
        .map_err(|error| AppError::Internal(format!("Simulation worker panicked: {error}")))?;

    sqlx::query("UPDATE simulation_jobs SET status='optimizing', updated_at=NOW() WHERE id=$1")
        .bind(job_id)
        .execute(state.db()?)
        .await?;
    persist_results(state.db()?, job_id, &result).await?;
    sqlx::query("UPDATE simulation_jobs SET status='completed', result=$2, completed_at=NOW(), updated_at=NOW() WHERE id=$1")
        .bind(job_id)
        .bind(serde_json::json!({ "deck_result_count": result.decks.len() }))
        .execute(state.db()?)
        .await?;
    Ok(())
}

async fn persist_results(
    pool: &sqlx::PgPool,
    job_id: Uuid,
    result: &SimRunResult,
) -> Result<(), AppError> {
    let mut tx = pool.begin().await?;
    sqlx::query("DELETE FROM deck_recommendations WHERE simulation_job_id=$1")
        .bind(job_id)
        .execute(&mut *tx)
        .await?;
    sqlx::query("DELETE FROM simulation_deck_results WHERE simulation_job_id=$1")
        .bind(job_id)
        .execute(&mut *tx)
        .await?;

    let candidates =
        crate::services::taptitan::recommendation::candidates_from_results(&result.decks);
    let mut result_ids = vec![None; result.decks.len()];
    let mut rows = Vec::with_capacity(candidates.len());
    for candidate in &candidates {
        let id = Uuid::new_v4();
        result_ids[candidate.source_index] = Some(id);
        rows.push(serde_json::json!({
            "id": id,
            "cards": candidate.cards,
            "card_mask": candidate.card_mask as i64,
            "average_damage": candidate.average_damage.to_string(),
            "result": result.decks[candidate.source_index],
        }));
    }
    for chunk in rows.chunks(500) {
        sqlx::query(
            "INSERT INTO simulation_deck_results (id, simulation_job_id, cards, card_mask, average_damage, result) SELECT (row->>'id')::UUID, $1, row->'cards', (row->>'card_mask')::BIGINT, (row->>'average_damage')::NUMERIC, row->'result' FROM jsonb_array_elements($2::JSONB) AS row",
        )
        .bind(job_id)
        .bind(serde_json::to_value(chunk)?)
        .execute(&mut *tx)
        .await?;
    }
    for deck_count in [6usize, 9usize] {
        if let Some(recommendation) = optimize_decks(&candidates, deck_count) {
            let recommendation_id = Uuid::new_v4();
            sqlx::query("INSERT INTO deck_recommendations (id, simulation_job_id, deck_count, total_average_damage) VALUES ($1,$2,$3,CAST($4 AS NUMERIC))")
                .bind(recommendation_id)
                .bind(job_id)
                .bind(deck_count as i32)
                .bind(recommendation.total_average_damage.to_string())
                .execute(&mut *tx)
                .await?;
            for (position, deck) in recommendation.decks.iter().enumerate() {
                let result_id = result_ids[deck.source_index].ok_or_else(|| {
                    AppError::Internal("Recommendation references missing deck result".to_string())
                })?;
                sqlx::query("INSERT INTO deck_recommendation_items (recommendation_id, position, simulation_deck_result_id) VALUES ($1,$2,$3)")
                    .bind(recommendation_id)
                    .bind(position as i32)
                    .bind(result_id)
                    .execute(&mut *tx)
                    .await?;
            }
        }
    }
    tx.commit().await?;
    Ok(())
}

pub async fn get_job(state: &AppState, job_id: Uuid) -> Result<SimulationJobView, AppError> {
    sqlx::query_as("SELECT j.id, p.player_id, j.player_stat_version_id, j.raid_boss_id, j.simulator_version, j.status, j.result, j.error_message, j.attempts, j.created_at, j.started_at, j.completed_at, j.updated_at FROM simulation_jobs j JOIN players p ON p.id=j.player_id WHERE j.id=$1")
        .bind(job_id)
        .fetch_optional(state.db()?)
        .await?
        .ok_or_else(|| AppError::NotFound("Simulation job not found".to_string()))
}

pub async fn list_player_jobs(
    state: &AppState,
    player_id: &str,
) -> Result<Vec<SimulationJobView>, AppError> {
    let jobs = sqlx::query_as("SELECT j.id, p.player_id, j.player_stat_version_id, j.raid_boss_id, j.simulator_version, j.status, j.result, j.error_message, j.attempts, j.created_at, j.started_at, j.completed_at, j.updated_at FROM simulation_jobs j JOIN players p ON p.id=j.player_id WHERE p.player_id=$1 ORDER BY j.created_at DESC LIMIT 100")
        .bind(player_id)
        .fetch_all(state.db()?)
        .await?;
    Ok(jobs)
}

pub async fn retry_job(state: &Arc<AppState>, job_id: Uuid) -> Result<(), AppError> {
    let updated = sqlx::query("UPDATE simulation_jobs SET status='pending', error_message=NULL, completed_at=NULL, updated_at=NOW() WHERE id=$1 AND status='failed'")
        .bind(job_id)
        .execute(state.db()?)
        .await?
        .rows_affected();
    if updated == 0 {
        return Err(AppError::Conflict(
            "Only failed jobs can be retried".to_string(),
        ));
    }
    spawn_job(Arc::clone(state), job_id);
    Ok(())
}
