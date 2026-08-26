use std::{sync::Arc, time::Instant};

use uuid::Uuid;

use crate::{
    error::AppError,
    models::{
        app::{CreateSimulationJobRequest, SimulationJobView},
        boss::{Boss, BossPartName, PartState},
        cards::CardName,
        db_enums::{RecommendationPhase, RecomputeMode},
        player_raid_data::PlayerRaidData,
        sim_payload::SimPayLoad,
    },
    services::taptitan::{
        recommendation::{
            CandidateDeck, DeckRecommendation, optimize_decks, optimize_decks_with_required_cards,
        },
        sim_service::{SIMS_ROUNDS, SimRunResult, SimService},
    },
    state::AppState,
};

const SIMULATOR_VERSION: &str = concat!(env!("CARGO_PKG_VERSION"), "-raid-cycle-v2");
pub const DEFAULT_RECOMMENDATION_DECK_COUNT: usize = 6;
pub const MAX_RECOMMENDATION_DECK_COUNT: usize = 14;

/// The exact boss snapshot that justified queuing a resim, passed straight
/// through instead of re-reading `current_boss` afterward -- a concurrent
/// attack event can otherwise clear a target's armor in the gap between
/// "we decided this needs a resim" and "the job captured its own snapshot",
/// silently losing the one shot at a void-phase (Insanity Void) simulation.
pub struct PreloadedBossSnapshot {
    pub boss: Boss,
    pub attackable_parts: Vec<BossPartName>,
    pub version: i64,
}

struct PreparedRecommendation {
    deck_count: usize,
    must_include_mirror_force: bool,
    must_include_team_tactics: bool,
    recommendation_phase: RecommendationPhase,
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
    recommendation_phase: RecommendationPhase,
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
    recommendation_phase: RecommendationPhase,
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
            "dependency_part_mask": result.decks[candidate.source_index].dependency_part_mask,
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
    create_job_with_mode(state, request, None, None).await
}

pub async fn create_phase_aware_job(
    state: &Arc<AppState>,
    request: CreateSimulationJobRequest,
    phase_change_mask: u8,
) -> Result<(Uuid, bool), AppError> {
    create_job_with_mode(state, request, Some(phase_change_mask), None).await
}

/// Same as `create_job`, but reuses `preloaded_boss` instead of re-reading
/// `current_boss` -- see `PreloadedBossSnapshot`.
pub async fn create_job_with_snapshot(
    state: &Arc<AppState>,
    request: CreateSimulationJobRequest,
    preloaded_boss: &PreloadedBossSnapshot,
) -> Result<(Uuid, bool), AppError> {
    create_job_with_mode(state, request, None, Some(preloaded_boss)).await
}

/// Same as `create_phase_aware_job`, but reuses `preloaded_boss` instead of
/// re-reading `current_boss` -- see `PreloadedBossSnapshot`.
pub async fn create_phase_aware_job_with_snapshot(
    state: &Arc<AppState>,
    request: CreateSimulationJobRequest,
    phase_change_mask: u8,
    preloaded_boss: &PreloadedBossSnapshot,
) -> Result<(Uuid, bool), AppError> {
    create_job_with_mode(state, request, Some(phase_change_mask), Some(preloaded_boss)).await
}

async fn create_job_with_mode(
    state: &Arc<AppState>,
    request: CreateSimulationJobRequest,
    phase_change_mask: Option<u8>,
    preloaded_boss: Option<&PreloadedBossSnapshot>,
) -> Result<(Uuid, bool), AppError> {
    let internal_player_id: Uuid = sqlx::query_scalar("SELECT id FROM players WHERE player_id=$1")
        .bind(request.player_id.trim())
        .fetch_optional(state.db()?)
        .await?
        .ok_or_else(|| AppError::NotFound("Player not found".to_string()))?;
    let loaded_stats = crate::services::player_stats_repo::load(state.db()?, internal_player_id)
        .await?
        .ok_or_else(|| AppError::NotFound("Player has no stored stats".to_string()))?;
    let stats_revision = loaded_stats.revision;
    let player_stats: PlayerRaidData = loaded_stats.data;

    let (boss_version, boss_data, attackable_part) = match preloaded_boss {
        Some(snapshot) => (
            snapshot.version,
            snapshot.boss.clone(),
            snapshot.attackable_parts.clone(),
        ),
        None => {
            let loaded_boss = crate::services::boss_repo::load(state.db()?)
                .await?
                .ok_or_else(|| AppError::NotFound("No sims boss data".to_string()))?;
            (
                loaded_boss.version,
                loaded_boss.boss,
                loaded_boss.attackable_parts,
            )
        }
    };
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
        "INSERT INTO simulation_jobs (id, player_id, boss_version, deduplication_key, simulator_version, status, payload, recompute_mode, phase_change_mask) VALUES ($1,$2,$3,$4,$5,'pending',$6,$7,$8) ON CONFLICT (deduplication_key) DO NOTHING RETURNING id",
    )
    .bind(job_id)
    .bind(internal_player_id)
    .bind(boss_version)
    .bind(&deduplication_key)
    .bind(SIMULATOR_VERSION)
    .bind(serde_json::to_value(payload)?)
    .bind(if phase_change_mask.is_some() {
        RecomputeMode::PhaseAware
    } else {
        RecomputeMode::Full
    })
    .bind(i16::from(phase_change_mask.unwrap_or_default()))
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
                "DELETE FROM simulation_jobs old WHERE old.id IN (SELECT candidate.id FROM simulation_jobs candidate WHERE candidate.boss_version < $1 AND candidate.status NOT IN ('pending','running','optimizing') AND (EXISTS (SELECT 1 FROM simulation_jobs current WHERE current.player_id=candidate.player_id AND current.boss_version >= $1 AND current.status='completed') OR candidate.id NOT IN (SELECT DISTINCT ON (keep.player_id) keep.id FROM simulation_jobs keep WHERE keep.boss_version < $1 AND keep.status='completed' ORDER BY keep.player_id,keep.boss_version DESC,keep.completed_at DESC NULLS LAST)) ORDER BY candidate.created_at LIMIT 10)",
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

    let payload: Option<(serde_json::Value, RecomputeMode, i16, Uuid, i64)> = sqlx::query_as(
        "UPDATE simulation_jobs SET status='running', attempts=attempts+1, started_at=NOW(), error_message=NULL, updated_at=NOW() WHERE id=$1 AND status IN ('pending','failed') RETURNING payload,recompute_mode,phase_change_mask,player_id,boss_version",
    )
    .bind(job_id)
    .fetch_optional(state.db()?)
    .await?;
    let Some((payload_json, recompute_mode, phase_change_mask, player_id, boss_version)) = payload
    else {
        return Ok(());
    };
    let payload: SimPayLoad = serde_json::from_value(payload_json)?;
    let total_processing_started = Instant::now();
    let simulation_started = Instant::now();
    let incremental = if recompute_mode == RecomputeMode::PhaseAware {
        try_incremental_run(
            state,
            player_id,
            boss_version,
            phase_change_mask as u8,
            &payload,
        )
        .await?
    } else {
        None
    };
    let (current_result, void_result, base_job_id, reused_decks, rerun_decks, execution_mode) =
        if let Some(run) = incremental {
            sqlx::query("UPDATE simulation_jobs SET base_job_id=$2 WHERE id=$1")
                .bind(job_id)
                .bind(run.base_job_id)
                .execute(state.db()?)
                .await?;
            (
                run.current,
                run.void_result,
                Some(run.base_job_id),
                run.reused_decks,
                run.rerun_decks,
                "incremental",
            )
        } else {
            if recompute_mode == RecomputeMode::PhaseAware {
                tracing::info!(
                    %job_id,
                    phase_change_mask,
                    "phase-aware simulation has no compatible reusable base; running full simulation"
                );
            }
            let (current, void_result) = tokio::task::spawn_blocking(move || {
                SimService::run_simulation_with_optional_body_phase(payload)
            })
            .await
            .map_err(|error| AppError::Internal(format!("Simulation worker panicked: {error}")))?;
            let rerun_decks =
                current.decks.len() + void_result.as_ref().map_or(0, |result| result.decks.len());
            (current, void_result, None, 0, rerun_decks, "full")
        };
    let simulation_duration_ms = simulation_started.elapsed().as_millis() as u64;

    sqlx::query("UPDATE simulation_jobs SET status='optimizing', updated_at=NOW() WHERE id=$1")
        .bind(job_id)
        .execute(state.db()?)
        .await?;
    let body_phase_ran = void_result.is_some();
    let recommendation_started = Instant::now();
    let prepared =
        tokio::task::spawn_blocking(move || -> Result<PreparedResults, serde_json::Error> {
            let current = prepare_results(current_result, RecommendationPhase::Current)?;
            match void_result {
                Some(result) => Ok(combine_prepared_results(
                    current,
                    prepare_results(result, RecommendationPhase::Void)?,
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
            "total_duration_ms": total_duration_ms,
            "execution_mode": execution_mode,
            "base_job_id": base_job_id,
            "reused_decks": reused_decks,
            "rerun_decks": rerun_decks
        }))
        .execute(state.db()?)
        .await?;
    tracing::info!(
        %job_id,
        execution_mode,
        reused_decks,
        rerun_decks,
        phase_change_mask,
        "simulation job completed"
    );
    spawn_old_job_cleanup(Arc::clone(state), boss_version);
    Ok(())
}

struct IncrementalRun {
    current: SimRunResult,
    void_result: Option<SimRunResult>,
    base_job_id: Uuid,
    reused_decks: usize,
    rerun_decks: usize,
}

async fn try_incremental_run(
    state: &Arc<AppState>,
    player_id: Uuid,
    boss_version: i64,
    requested_mask: u8,
    payload: &SimPayLoad,
) -> Result<Option<IncrementalRun>, AppError> {
    let candidates: Vec<(Uuid, serde_json::Value)> = sqlx::query_as(
        "SELECT id,payload FROM simulation_jobs WHERE player_id=$1 AND status='completed' AND boss_version < $2 AND simulator_version=$3 ORDER BY boss_version DESC,completed_at DESC LIMIT 10",
    )
    .bind(player_id)
    .bind(boss_version)
    .bind(SIMULATOR_VERSION)
    .fetch_all(state.db()?)
    .await?;

    let mut selected = None;
    for (candidate_id, candidate_payload) in candidates {
        let Ok(candidate_payload) = serde_json::from_value::<SimPayLoad>(candidate_payload) else {
            continue;
        };
        if let Some(changed_mask) = compatible_incremental_mask(&candidate_payload, payload) {
            if changed_mask & requested_mask != 0 {
                selected = Some((candidate_id, changed_mask));
                break;
            }
        }
    }
    let Some((base_job_id, changed_mask)) = selected else {
        return Ok(None);
    };

    let rows: Vec<(serde_json::Value, Option<i16>, RecommendationPhase)> = sqlx::query_as(
        "SELECT result,dependency_part_mask,recommendation_phase FROM simulation_deck_results WHERE simulation_job_id=$1 ORDER BY recommendation_phase,card_mask",
    )
    .bind(base_job_id)
    .fetch_all(state.db()?)
    .await?;
    if rows.is_empty() || rows.iter().any(|(_, mask, _)| mask.is_none()) {
        return Ok(None);
    }

    let current_rows = rows
        .iter()
        .filter(|(_, _, phase)| *phase == RecommendationPhase::Current)
        .cloned()
        .collect::<Vec<_>>();
    let void_rows = rows
        .iter()
        .filter(|(_, _, phase)| *phase == RecommendationPhase::Void)
        .cloned()
        .collect::<Vec<_>>();
    let should_have_void = payload.include_body_phase
        && payload.attackable_part.iter().any(|part_name| {
            matches!(
                payload.boss_data.part(*part_name).part_state,
                PartState::Armor | PartState::Cursed
            )
        });
    if current_rows.is_empty() || (should_have_void && void_rows.is_empty()) {
        return Ok(None);
    }

    let payload = payload.clone();
    let rebuilt = tokio::task::spawn_blocking(move || {
        let (current, current_reused, current_rerun) =
            rebuild_incremental_phase(&payload, current_rows, changed_mask, false)?;
        let (void_result, void_reused, void_rerun) = if should_have_void {
            let (result, reused, rerun) =
                rebuild_incremental_void_phase(&payload, &current, void_rows)?;
            (Some(result), reused, rerun)
        } else {
            (None, 0, 0)
        };
        Some((
            current,
            void_result,
            current_reused + void_reused,
            current_rerun + void_rerun,
        ))
    })
    .await
    .map_err(|error| {
        AppError::Internal(format!("Incremental simulation worker panicked: {error}"))
    })?;
    let Some((current, void_result, reused_decks, rerun_decks)) = rebuilt else {
        return Ok(None);
    };
    tracing::info!(
        %base_job_id,
        changed_mask,
        reused_decks,
        rerun_decks,
        "prepared incremental simulation results"
    );
    Ok(Some(IncrementalRun {
        current,
        void_result,
        base_job_id,
        reused_decks,
        rerun_decks,
    }))
}

fn rebuild_incremental_void_phase(
    payload: &SimPayLoad,
    current: &SimRunResult,
    base_void_rows: Vec<(serde_json::Value, Option<i16>, RecommendationPhase)>,
) -> Option<(SimRunResult, usize, usize)> {
    let mut decks = current
        .decks
        .iter()
        .filter(|result| !result.deck.contains(&CardName::InsanityVoid))
        .cloned()
        .collect::<Vec<_>>();
    let reused = decks.len();
    let mut rerun = 0usize;

    for (result, dependency_mask, _) in base_void_rows {
        dependency_mask?;
        let result: crate::services::taptitan::sim_service::SimDeckResult =
            serde_json::from_value(result).ok()?;
        if !result.deck.contains(&CardName::InsanityVoid) {
            continue;
        }
        rerun += 1;
        let mut deck_payload = payload.clone();
        deck_payload.usable_card = result.deck;
        if let Some(result) = SimService::run_exact_deck_for_phase(deck_payload, true, SIMS_ROUNDS)
        {
            decks.push(result);
        }
    }

    let total_attack_patterns = decks.iter().map(|deck| deck.total_attack_patterns).sum();
    Some((
        SimRunResult {
            total_decks: decks.len(),
            total_attack_patterns,
            rounds_per_pattern: SIMS_ROUNDS,
            ticks_per_round: 600,
            decks,
        },
        reused,
        rerun,
    ))
}

fn compatible_incremental_mask(base: &SimPayLoad, current: &SimPayLoad) -> Option<u8> {
    if base.include_body_phase != current.include_body_phase
        || base.attackable_part != current.attackable_part
        || base.usable_card != current.usable_card
        || (base.mirror_force_boost - current.mirror_force_boost).abs() > 1e-9
        || serde_json::to_value(&base.player_raid_data).ok()?
            != serde_json::to_value(&current.player_raid_data).ok()?
        || base.boss_data.boss_name != current.boss_data.boss_name
        || base.boss_data.global_raid_modifier != current.boss_data.global_raid_modifier
        || base.boss_data.global_raid_modifier_amount
            != current.boss_data.global_raid_modifier_amount
        || base.boss_data.curse_type != current.boss_data.curse_type
        || (base.boss_data.curse_damage_per_curse - current.boss_data.curse_damage_per_curse).abs()
            > 1e-9
        || base.boss_data.recommend_1_to_2_part_patterns_only
            != current.boss_data.recommend_1_to_2_part_patterns_only
    {
        return None;
    }

    incremental_boss_change_mask(&base.boss_data, &current.boss_data)
}

fn incremental_boss_change_mask(base: &Boss, current: &Boss) -> Option<u8> {
    let mut changed_mask = 0u8;
    for part_name in BossPartName::all() {
        let before = base.part(part_name).part_state;
        let after = current.part(part_name).part_state;
        if before == after {
            continue;
        }
        if before == PartState::Body && after == PartState::Skeleton {
            changed_mask |= part_name.dependency_mask();
        } else {
            return None;
        }
    }
    (changed_mask != 0).then_some(changed_mask)
}

fn rebuild_incremental_phase(
    payload: &SimPayLoad,
    rows: Vec<(serde_json::Value, Option<i16>, RecommendationPhase)>,
    changed_mask: u8,
    body_phase: bool,
) -> Option<(SimRunResult, usize, usize)> {
    let mut decks = Vec::with_capacity(rows.len());
    let mut reused = 0usize;
    let mut rerun = 0usize;
    for (result, dependency_mask, _) in rows {
        let dependency_mask = dependency_mask? as u8;
        let mut result: crate::services::taptitan::sim_service::SimDeckResult =
            serde_json::from_value(result).ok()?;
        result.dependency_part_mask = dependency_mask;
        if dependency_mask & changed_mask == 0 {
            reused += 1;
            decks.push(result);
            continue;
        }

        rerun += 1;
        let mut deck_payload = payload.clone();
        deck_payload.usable_card = result.deck;
        if let Some(result) =
            SimService::run_exact_deck_for_phase(deck_payload, body_phase, SIMS_ROUNDS)
        {
            decks.push(result);
        }
    }
    let total_attack_patterns = decks.iter().map(|deck| deck.total_attack_patterns).sum();
    Some((
        SimRunResult {
            total_decks: decks.len(),
            total_attack_patterns,
            rounds_per_pattern: SIMS_ROUNDS,
            ticks_per_round: 600,
            decks,
        },
        reused,
        rerun,
    ))
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
            "INSERT INTO simulation_deck_results (id, simulation_job_id, cards, card_mask, average_damage, recommendation_phase, dependency_part_mask, result) SELECT (row->>'id')::UUID, $1, row->'cards', (row->>'card_mask')::BIGINT, (row->>'average_damage')::NUMERIC, (row->>'recommendation_phase')::recommendation_phase, (row->>'dependency_part_mask')::SMALLINT, row->'result' FROM jsonb_array_elements($2::JSONB) AS row",
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
        RecommendationPhase::Void
    } else {
        RecommendationPhase::Current
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

#[cfg(test)]
#[path = "../../tests/unit/services/job_service_tests.rs"]
mod phase_aware_tests;
