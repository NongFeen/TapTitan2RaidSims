use std::{collections::HashSet, sync::Arc};

use axum::{Json, extract::State};
use serde::{Deserialize, Serialize};

use crate::{
    error::AppError,
    models::{
        boss::{Boss, BossPartName},
        cards::CardName,
        player_raid_data::PlayerRaidData,
        sim_payload::SimPayLoad,
    },
    services::taptitan::sim_service::{SimDeckResult, SimService},
    state::AppState,
};

const DEBUG_TICKS_PER_ROUND: u32 = 600;
const MAX_TOTAL_TAPS: u32 = DEBUG_TICKS_PER_ROUND;
const MAX_DEBUG_ROUNDS: u64 = 100;

#[derive(Debug, Deserialize)]
pub struct DebugSimulationRequest {
    pub player_id: String,
    pub boss_data: Boss,
    pub attackable_parts: Vec<BossPartName>,
    pub deck: Vec<CardName>,
    pub total_taps: u32,
    pub rounds_per_pattern: u64,
}

#[derive(Debug, Serialize)]
pub struct DebugSimulationResponse {
    pub rounds_per_pattern: u64,
    pub ticks_per_round: u32,
    pub total_taps: u32,
    pub result: SimDeckResult,
}

pub async fn run(
    State(state): State<Arc<AppState>>,
    Json(request): Json<DebugSimulationRequest>,
) -> Result<Json<DebugSimulationResponse>, AppError> {
    if request.deck.len() != 3 || request.deck.iter().copied().collect::<HashSet<_>>().len() != 3 {
        return Err(AppError::BadRequest(
            "Debug simulation requires exactly three different cards".to_string(),
        ));
    }
    if !(1..=MAX_TOTAL_TAPS).contains(&request.total_taps) {
        return Err(AppError::BadRequest(format!(
            "total_taps must be between 1 and {MAX_TOTAL_TAPS}"
        )));
    }
    if !(1..=MAX_DEBUG_ROUNDS).contains(&request.rounds_per_pattern) {
        return Err(AppError::BadRequest(format!(
            "rounds_per_pattern must be between 1 and {MAX_DEBUG_ROUNDS}"
        )));
    }
    if request.attackable_parts.is_empty() {
        return Err(AppError::BadRequest(
            "Select at least one attackable boss part".to_string(),
        ));
    }
    if request
        .attackable_parts
        .iter()
        .copied()
        .collect::<HashSet<_>>()
        .len()
        != request.attackable_parts.len()
    {
        return Err(AppError::BadRequest(
            "Attackable boss parts contain duplicates".to_string(),
        ));
    }

    let stats_json: serde_json::Value = sqlx::query_scalar(
        "SELECT s.stats FROM player_stats s WHERE s.player_id=$1",
    )
    .bind(request.player_id.trim())
    .fetch_optional(state.db()?)
    .await?
    .ok_or_else(|| AppError::NotFound("Player has no current stats".to_string()))?;
    let player_raid_data: PlayerRaidData = serde_json::from_value(stats_json)?;
    for card_name in &request.deck {
        let available = player_raid_data
            .card_list
            .iter()
            .any(|card| card.card_id == *card_name && card.enabled);
        if !available {
            return Err(AppError::BadRequest(format!(
                "{} is not enabled for this player",
                card_name.display_name()
            )));
        }
    }
    let mut boss_data = request.boss_data;
    boss_data.sync_part_states_from_current_values();
    let mirror_force_boost: f64 = sqlx::query_scalar(
        "SELECT COALESCE((SELECT mirror_force_boost FROM raid_cycle_state ORDER BY updated_at DESC LIMIT 1), 0::DOUBLE PRECISION)",
    )
    .fetch_one(state.db()?)
    .await?;
    let payload = SimPayLoad {
        player_raid_data,
        boss_data,
        attackable_part: request.attackable_parts,
        usable_card: request.deck,
        include_body_phase: false,
        mirror_force_boost,
    };
    let total_taps = request.total_taps;
    let rounds_per_pattern = request.rounds_per_pattern;
    let _permit = Arc::clone(&state.simulation_slots)
        .acquire_owned()
        .await
        .map_err(|_| {
            AppError::ServiceUnavailable("Simulation service is shutting down".to_string())
        })?;
    let result = tokio::task::spawn_blocking(move || {
        SimService::run_deck_debug_simulation(payload, total_taps, rounds_per_pattern)
    })
    .await
    .map_err(|error| {
        tracing::error!(?error, "debug simulation worker panicked");
        AppError::Internal("Debug simulation worker failed".to_string())
    })?
    .ok_or_else(|| {
        AppError::BadRequest("The selected deck has no valid attack patterns".to_string())
    })?;

    Ok(Json(DebugSimulationResponse {
        rounds_per_pattern,
        ticks_per_round: DEBUG_TICKS_PER_ROUND,
        total_taps,
        result,
    }))
}
