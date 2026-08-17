use std::{sync::Arc, time::Instant};

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
        recommendation::{
            CandidateDeck, DeckRecommendation, optimize_decks, optimize_decks_with_required_cards,
        },
        sim_service::{SimRunResult, SimService},
    },
    state::AppState,
};

const SIMULATOR_VERSION: &str = concat!(env!("CARGO_PKG_VERSION"), "-raid-cycle-v2");
const CURRENT_RECOMMENDATION_PHASE: &str = "current";
const VOID_RECOMMENDATION_PHASE: &str = "void";
pub const DEFAULT_RECOMMENDATION_DECK_COUNT: usize = 6;
pub const MAX_RECOMMENDATION_DECK_COUNT: usize = 14;

struct PreparedRecommendation {
    deck_count: usize,
    must_include_mirror_force: bool,
    must_include_team_tactics: bool,
    recommendation_phase: &'static str,
    recommendation: DeckRecommendation,
}

struct PreparedResults {
    deck_result_count: usize,
    row_chunks: Vec<serde_json::Value>,
    result_ids: Vec<Option<Uuid>>,
    recommendations: Vec<PreparedRecommendation>,
}

fn prepare_recommendations(
    candidates: &[CandidateDeck],
    deck_counts: &[usize],
    recommendation_phase: &'static str,
) -> Vec<PreparedRecommendation> {
    let mut recommendations = Vec::with_capacity(deck_counts.len() * 4);
    for &deck_count in deck_counts {
        for (must_include_mirror_force, must_include_team_tactics) in
            [(false, false), (true, false), (false, true), (true, true)]
        {
            let mut required_cards = Vec::with_capacity(2);
            if must_include_mirror_force {
                required_cards.push(CardName::MirrorForce);
            }
            if must_include_team_tactics {
                required_cards.push(CardName::TeamTactics);
            }
            tracing::info!(
                deck_count,
                must_include_mirror_force,
                must_include_team_tactics,
                "starting top deck recommendation search"
            );
            let search_started = Instant::now();
            let recommendation = if required_cards.is_empty() {
                optimize_decks(candidates, deck_count)
            } else {
                optimize_decks_with_required_cards(candidates, deck_count, &required_cards)
            };
            if let Some(recommendation) = recommendation {
                tracing::info!(
                    deck_count,
                    must_include_mirror_force,
                    must_include_team_tactics,
                    total_average_damage = recommendation.total_average_damage,
                    elapsed_ms = search_started.elapsed().as_millis(),
                    "completed top deck recommendation search"
                );
                recommendations.push(PreparedRecommendation {
                    deck_count,
                    must_include_mirror_force,
                    must_include_team_tactics,
                    recommendation_phase,
                    recommendation,
                });
            } else {
                tracing::warn!(
                    deck_count,
                    must_include_mirror_force,
                    must_include_team_tactics,
                    "no compatible top deck recommendation found"
                );
            }
        }
    }
    recommendations
}

fn prepare_results(
    result: SimRunResult,
    recommendation_phase: &'static str,
) -> Result<PreparedResults, serde_json::Error> {
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
            "recommendation_phase": recommendation_phase,
            "result": result.decks[candidate.source_index],
        }));
    }
    let row_chunks = rows
        .chunks(500)
        .map(serde_json::to_value)
        .collect::<Result<Vec<_>, _>>()?;

    let recommendations = prepare_recommendations(
        &candidates,
        &[DEFAULT_RECOMMENDATION_DECK_COUNT],
        recommendation_phase,
    );

    Ok(PreparedResults {
        deck_result_count: result.decks.len(),
        row_chunks,
        result_ids,
        recommendations,
    })
}

fn combine_prepared_results(
    mut current: PreparedResults,
    mut void_phase: PreparedResults,
) -> PreparedResults {
    let source_index_offset = current.result_ids.len();
    for prepared in &mut void_phase.recommendations {
        for deck in &mut prepared.recommendation.decks {
            deck.source_index += source_index_offset;
        }
    }
    current.deck_result_count += void_phase.deck_result_count;
    current.row_chunks.append(&mut void_phase.row_chunks);
    current.result_ids.append(&mut void_phase.result_ids);
    current
        .recommendations
        .append(&mut void_phase.recommendations);
    current
}

pub async fn create_job(
    state: &Arc<AppState>,
    request: CreateSimulationJobRequest,
) -> Result<(Uuid, bool), AppError> {
    let internal_player_id: Uuid = sqlx::query_scalar("SELECT id FROM players WHERE player_id=$1")
        .bind(request.player_id.trim())
        .fetch_optional(state.db()?)
        .await?
        .ok_or_else(|| AppError::NotFound("Player not found".to_string()))?;
    let row: Option<(i64, serde_json::Value)> =
        sqlx::query_as("SELECT revision, stats FROM player_stats WHERE player_id = $1")
            .bind(internal_player_id)
            .fetch_optional(state.db()?)
            .await?;
    let (stats_revision, stats_json) =
        row.ok_or_else(|| AppError::NotFound("Player has no stored stats".to_string()))?;
    let player_stats: PlayerRaidData = serde_json::from_value(stats_json)?;

    let boss_row: Option<(serde_json::Value, serde_json::Value, i64)> = sqlx::query_as(
        "SELECT boss_data, attackable_parts, version FROM current_boss WHERE singleton=TRUE",
    )
    .fetch_optional(state.db()?)
    .await?;
    let (boss_json, attackable_json, boss_version) =
        boss_row.ok_or_else(|| AppError::NotFound("No sims boss data".to_string()))?;
    let boss_data: Boss = serde_json::from_value(boss_json)?;
    let attackable_part: Vec<BossPartName> = serde_json::from_value(attackable_json)?;
    let usable_card: Vec<CardName> = player_stats
        .card_list
        .iter()
        .filter(|card| card.enabled)
        .map(|card| card.card_id)
        .collect();
    let mirror_force_boost: f64 = sqlx::query_scalar(
        "SELECT COALESCE((SELECT mirror_force_boost FROM raid_cycle_state ORDER BY updated_at DESC LIMIT 1), 0::DOUBLE PRECISION)",
    )
    .fetch_one(state.db()?)
    .await?;

    let payload = SimPayLoad {
        player_raid_data: player_stats,
        boss_data,
        attackable_part,
        usable_card,
        include_body_phase: request.include_body_phase,
        mirror_force_boost,
    };
    let deduplication_key = format!(
        "{}:{}:{}:{}:{}:{:.6}",
        internal_player_id,
        stats_revision,
        boss_version,
        SIMULATOR_VERSION,
        request.include_body_phase,
        mirror_force_boost
    );
    let job_id = Uuid::new_v4();
    let inserted: Option<(Uuid,)> = sqlx::query_as(
        "INSERT INTO simulation_jobs (id, player_id, boss_version, deduplication_key, simulator_version, status, payload) VALUES ($1,$2,$3,$4,$5,'pending',$6) ON CONFLICT (deduplication_key) DO NOTHING RETURNING id",
    )
    .bind(job_id)
    .bind(internal_player_id)
    .bind(boss_version)
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

pub fn spawn_old_job_cleanup(state: Arc<AppState>, current_boss_version: i64) {
    tokio::spawn(async move {
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
        let Ok(db) = state.db() else {
            return;
        };
        let mut deleted_total = 0u64;
        loop {
            let result = sqlx::query(
                "DELETE FROM simulation_jobs WHERE id IN (SELECT id FROM simulation_jobs WHERE boss_version < $1 ORDER BY created_at LIMIT 10)",
            )
            .bind(current_boss_version)
            .execute(db)
            .await;
            match result {
                Ok(result) => {
                    let deleted = result.rows_affected();
                    deleted_total += deleted;
                    if deleted == 0 {
                        tracing::info!(
                            current_boss_version,
                            deleted_jobs = deleted_total,
                            "old simulation job cleanup complete"
                        );
                        break;
                    }
                    tokio::task::yield_now().await;
                }
                Err(error) => {
                    tracing::error!(
                        current_boss_version,
                        deleted_jobs = deleted_total,
                        ?error,
                        "old simulation job cleanup failed"
                    );
                    break;
                }
            }
        }
        if let Err(error) = sqlx::query(
            "DELETE FROM simulation_batches b WHERE NOT EXISTS (SELECT 1 FROM simulation_batch_jobs bj WHERE bj.batch_id=b.id)",
        )
        .execute(db)
        .await
        {
            tracing::warn!(?error, "empty simulation batch cleanup failed");
        }
    });
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
    let total_processing_started = Instant::now();
    let simulation_started = Instant::now();
    let (current_result, void_result) = tokio::task::spawn_blocking(move || {
        SimService::run_simulation_with_optional_body_phase(payload)
    })
    .await
    .map_err(|error| AppError::Internal(format!("Simulation worker panicked: {error}")))?;
    let simulation_duration_ms = simulation_started.elapsed().as_millis() as u64;

    sqlx::query("UPDATE simulation_jobs SET status='optimizing', updated_at=NOW() WHERE id=$1")
        .bind(job_id)
        .execute(state.db()?)
        .await?;
    let body_phase_ran = void_result.is_some();
    let recommendation_started = Instant::now();
    let prepared =
        tokio::task::spawn_blocking(move || -> Result<PreparedResults, serde_json::Error> {
            let current = prepare_results(current_result, CURRENT_RECOMMENDATION_PHASE)?;
            match void_result {
                Some(result) => Ok(combine_prepared_results(
                    current,
                    prepare_results(result, VOID_RECOMMENDATION_PHASE)?,
                )),
                None => Ok(current),
            }
        })
        .await
        .map_err(|error| {
            AppError::Internal(format!("Recommendation optimizer panicked: {error}"))
        })??;
    let recommendation_duration_ms = recommendation_started.elapsed().as_millis() as u64;
    let deck_result_count = prepared.deck_result_count;
    persist_results(state.db()?, job_id, prepared).await?;
    let total_duration_ms = total_processing_started.elapsed().as_millis() as u64;
    sqlx::query("UPDATE simulation_jobs SET status='completed', result=$2, completed_at=NOW(), updated_at=NOW() WHERE id=$1")
        .bind(job_id)
        .bind(serde_json::json!({
            "deck_result_count": deck_result_count,
            "body_phase_ran": body_phase_ran,
            "simulation_duration_ms": simulation_duration_ms,
            "recommendation_duration_ms": recommendation_duration_ms,
            "total_duration_ms": total_duration_ms
        }))
        .execute(state.db()?)
        .await?;
    Ok(())
}

async fn persist_results(
    pool: &sqlx::PgPool,
    job_id: Uuid,
    prepared: PreparedResults,
) -> Result<(), AppError> {
    let PreparedResults {
        row_chunks,
        result_ids,
        recommendations,
        ..
    } = prepared;
    let mut tx = pool.begin().await?;
    sqlx::query("DELETE FROM deck_recommendations WHERE simulation_job_id=$1")
        .bind(job_id)
        .execute(&mut *tx)
        .await?;
    sqlx::query("DELETE FROM simulation_deck_results WHERE simulation_job_id=$1")
        .bind(job_id)
        .execute(&mut *tx)
        .await?;

    let persistence_started = Instant::now();
    let total_chunks = row_chunks.len();
    let mut next_progress_percent = 10usize;
    tracing::info!(
        job_id = %job_id,
        deck_result_chunks = total_chunks,
        progress_percent = 0,
        "persisting simulation deck results"
    );
    for (index, chunk) in row_chunks.into_iter().enumerate() {
        sqlx::query(
            "INSERT INTO simulation_deck_results (id, simulation_job_id, cards, card_mask, average_damage, recommendation_phase, result) SELECT (row->>'id')::UUID, $1, row->'cards', (row->>'card_mask')::BIGINT, (row->>'average_damage')::NUMERIC, row->>'recommendation_phase', row->'result' FROM jsonb_array_elements($2::JSONB) AS row",
        )
        .bind(job_id)
        .bind(chunk)
        .execute(&mut *tx)
        .await?;

        let progress_percent = (index + 1) * 100 / total_chunks.max(1);
        while progress_percent >= next_progress_percent && next_progress_percent <= 100 {
            tracing::info!(
                job_id = %job_id,
                progress_percent = next_progress_percent,
                elapsed_ms = persistence_started.elapsed().as_millis(),
                "persisting simulation deck results"
            );
            next_progress_percent += 10;
        }
    }
    for prepared_recommendation in recommendations {
        let PreparedRecommendation {
            deck_count,
            must_include_mirror_force,
            must_include_team_tactics,
            recommendation_phase,
            recommendation,
        } = prepared_recommendation;
        let recommendation_id = Uuid::new_v4();
        sqlx::query("INSERT INTO deck_recommendations (id, simulation_job_id, deck_count, must_include_mirror_force, must_include_team_tactics, recommendation_phase, total_average_damage) VALUES ($1,$2,$3,$4,$5,$6,CAST($7 AS NUMERIC))")
                    .bind(recommendation_id)
                    .bind(job_id)
                    .bind(deck_count as i32)
                    .bind(must_include_mirror_force)
                    .bind(must_include_team_tactics)
                    .bind(recommendation_phase)
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
    tx.commit().await?;
    tracing::info!(
        job_id = %job_id,
        elapsed_ms = persistence_started.elapsed().as_millis(),
        "simulation results persistence complete"
    );
    Ok(())
}

pub async fn generate_deck_recommendations(
    state: &Arc<AppState>,
    player_id: &str,
    deck_count: usize,
    include_body_phase: bool,
) -> Result<bool, AppError> {
    if !(1..=MAX_RECOMMENDATION_DECK_COUNT).contains(&deck_count) {
        return Err(AppError::BadRequest(format!(
            "deck_count must be between 1 and {MAX_RECOMMENDATION_DECK_COUNT}"
        )));
    }

    let _permit = state
        .recommendation_slots
        .clone()
        .acquire_owned()
        .await
        .map_err(|_| AppError::Internal("Recommendation worker is shutting down".to_string()))?;

    let recommendation_phase = if include_body_phase {
        VOID_RECOMMENDATION_PHASE
    } else {
        CURRENT_RECOMMENDATION_PHASE
    };
    let job_id: Uuid = sqlx::query_scalar(
        "SELECT j.id FROM simulation_jobs j JOIN players p ON p.id=j.player_id WHERE p.player_id=$1 AND j.status='completed' AND j.boss_version=(SELECT version FROM current_boss WHERE singleton=TRUE) AND EXISTS (SELECT 1 FROM simulation_deck_results d WHERE d.simulation_job_id=j.id AND d.recommendation_phase=$2) ORDER BY j.completed_at DESC LIMIT 1",
    )
    .bind(player_id)
    .bind(recommendation_phase)
    .fetch_optional(state.db()?)
    .await?
    .ok_or_else(|| AppError::NotFound("Player has no completed simulation".to_string()))?;

    let already_generated: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM deck_recommendations WHERE simulation_job_id=$1 AND deck_count=$2 AND recommendation_phase=$3)",
    )
    .bind(job_id)
    .bind(deck_count as i32)
    .bind(recommendation_phase)
    .fetch_one(state.db()?)
    .await?;
    if already_generated {
        return Ok(false);
    }

    let stored_results: Vec<(Uuid, serde_json::Value)> = sqlx::query_as(
        "SELECT id, result FROM simulation_deck_results WHERE simulation_job_id=$1 AND recommendation_phase=$2 ORDER BY average_damage DESC",
    )
    .bind(job_id)
    .bind(recommendation_phase)
    .fetch_all(state.db()?)
    .await?;
    if stored_results.is_empty() {
        return Err(AppError::NotFound(
            "Completed simulation has no deck results".to_string(),
        ));
    }

    let (result_ids, recommendations) = tokio::task::spawn_blocking(move || {
        let mut ids = Vec::with_capacity(stored_results.len());
        let mut results = Vec::with_capacity(stored_results.len());
        for (id, result) in stored_results {
            ids.push(Some(id));
            results.push(serde_json::from_value(result)?);
        }
        let candidates =
            crate::services::taptitan::recommendation::candidates_from_results(&results);
        let recommendations =
            prepare_recommendations(&candidates, &[deck_count], recommendation_phase);
        Ok::<_, serde_json::Error>((ids, recommendations))
    })
    .await
    .map_err(|error| {
        AppError::Internal(format!("On-demand recommendation worker panicked: {error}"))
    })??;

    let mut tx = state.db()?.begin().await?;
    sqlx::query("DELETE FROM deck_recommendations WHERE simulation_job_id=$1 AND deck_count=$2 AND recommendation_phase=$3")
        .bind(job_id)
        .bind(deck_count as i32)
        .bind(recommendation_phase)
        .execute(&mut *tx)
        .await?;
    for prepared in recommendations {
        let recommendation_id = Uuid::new_v4();
        sqlx::query("INSERT INTO deck_recommendations (id, simulation_job_id, deck_count, must_include_mirror_force, must_include_team_tactics, recommendation_phase, total_average_damage) VALUES ($1,$2,$3,$4,$5,$6,CAST($7 AS NUMERIC))")
            .bind(recommendation_id)
            .bind(job_id)
            .bind(prepared.deck_count as i32)
            .bind(prepared.must_include_mirror_force)
            .bind(prepared.must_include_team_tactics)
            .bind(prepared.recommendation_phase)
            .bind(prepared.recommendation.total_average_damage.to_string())
            .execute(&mut *tx)
            .await?;
        for (position, deck) in prepared.recommendation.decks.iter().enumerate() {
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
    tx.commit().await?;
    Ok(true)
}

pub async fn get_job(state: &AppState, job_id: Uuid) -> Result<SimulationJobView, AppError> {
    sqlx::query_as("SELECT j.id, p.player_id, j.simulator_version, j.status, j.result, j.error_message, j.attempts, j.created_at, j.started_at, j.completed_at, j.updated_at FROM simulation_jobs j JOIN players p ON p.id=j.player_id WHERE j.id=$1")
        .bind(job_id)
        .fetch_optional(state.db()?)
        .await?
        .ok_or_else(|| AppError::NotFound("Simulation job not found".to_string()))
}

pub async fn list_player_jobs(
    state: &AppState,
    player_id: &str,
) -> Result<Vec<SimulationJobView>, AppError> {
    let jobs = sqlx::query_as("SELECT j.id, p.player_id, j.simulator_version, j.status, j.result, j.error_message, j.attempts, j.created_at, j.started_at, j.completed_at, j.updated_at FROM simulation_jobs j JOIN players p ON p.id=j.player_id WHERE p.player_id=$1 ORDER BY j.created_at DESC LIMIT 100")
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
