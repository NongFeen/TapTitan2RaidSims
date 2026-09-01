use std::sync::Arc;

use axum::{
    Json,
    extract::{Path, Query, State},
};
use serde::Deserialize;
use serde_json::Value;
use utoipa::{IntoParams, ToSchema};

use crate::{
    error::AppError,
    models::app::RecommendationView,
    services::{
        job_service::{DEFAULT_RECOMMENDATION_DECK_COUNT, MAX_RECOMMENDATION_DECK_COUNT},
        taptitan::recommendation::cards_from_mask,
    },
    state::AppState,
};

#[derive(Deserialize, IntoParams)]
pub struct RecommendationQuery {
    #[serde(default = "default_deck_count")]
    deck_count: i32,
    #[serde(default)]
    must_include_mirror_force: bool,
    #[serde(default)]
    must_include_team_tactics: bool,
    include_body_phase: Option<bool>,
}

fn default_deck_count() -> i32 {
    DEFAULT_RECOMMENDATION_DECK_COUNT as i32
}

fn validate_deck_count(deck_count: i32) -> Result<usize, AppError> {
    if !(1..=MAX_RECOMMENDATION_DECK_COUNT as i32).contains(&deck_count) {
        return Err(AppError::BadRequest(format!(
            "deck_count must be between 1 and {MAX_RECOMMENDATION_DECK_COUNT}"
        )));
    }
    Ok(deck_count as usize)
}

#[derive(Deserialize, ToSchema)]
pub struct GenerateRecommendationRequest {
    #[serde(default = "default_deck_count")]
    deck_count: i32,
    #[serde(default)]
    include_body_phase: bool,
}

#[derive(serde::Serialize, ToSchema)]
pub struct GenerateRecommendationResponse {
    deck_count: i32,
    created: bool,
}

/// Generate deck recommendations for a player
///
/// Queues a simulation job (or reuses an in-flight/completed one for the
/// current boss version) whose results back
/// `GET .../recommendations/current`.
#[utoipa::path(
    post,
    path = "/api/players/{player_id}/recommendations",
    tag = "recommendations",
    params(("player_id" = String, Path, description = "Player id")),
    request_body = GenerateRecommendationRequest,
    responses((status = 200, description = "Whether a new simulation job was created", body = GenerateRecommendationResponse)),
)]
pub async fn generate_for_player(
    State(state): State<Arc<AppState>>,
    Path(player_id): Path<String>,
    Json(request): Json<GenerateRecommendationRequest>,
) -> Result<Json<GenerateRecommendationResponse>, AppError> {
    let deck_count = validate_deck_count(request.deck_count)?;
    let created = crate::services::job_service::generate_deck_recommendations(
        &state,
        &player_id,
        deck_count,
        request.include_body_phase,
    )
    .await?;
    Ok(Json(GenerateRecommendationResponse {
        deck_count: request.deck_count,
        created,
    }))
}

/// Get the latest completed recommendation for a player
#[utoipa::path(
    get,
    path = "/api/players/{player_id}/recommendations/current",
    tag = "recommendations",
    params(("player_id" = String, Path, description = "Player id"), RecommendationQuery),
    responses(
        (status = 200, description = "Latest completed recommendation matching the query", body = RecommendationView),
        (status = 404, description = "Player not found, or no completed recommendation matches"),
    ),
)]
pub async fn current_for_player(
    State(state): State<Arc<AppState>>,
    Path(player_id): Path<String>,
    Query(query): Query<RecommendationQuery>,
) -> Result<Json<RecommendationView>, AppError> {
    validate_deck_count(query.deck_count)?;
    let player_exists: bool =
        sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM players WHERE player_id=$1)")
            .bind(&player_id)
            .fetch_one(state.db()?)
            .await?;
    if !player_exists {
        return Err(AppError::NotFound("Player not found".to_string()));
    }
    let mut recommendation: RecommendationView = sqlx::query_as(
        "SELECT r.id, r.simulation_job_id, r.deck_count, r.must_include_mirror_force, r.must_include_team_tactics, r.total_average_damage::TEXT AS total_average_damage, (r.recommendation_phase='void') AS body_phase_ran, COALESCE(jsonb_agg(jsonb_build_object('position', i.position, 'card_mask', d.card_mask, 'average_damage', d.average_damage::TEXT, 'result', jsonb_build_object('best_pattern', jsonb_build_object('pattern', d.pattern, 'lowest_round_damage', d.deck_lowest_damage, 'highest_round_damage', d.deck_highest_damage, 'card_damage', jsonb_build_array(jsonb_build_object('card', d.card1, 'average_damage', d.card1_damage), jsonb_build_object('card', d.card2, 'average_damage', d.card2_damage), jsonb_build_object('card', d.card3, 'average_damage', d.card3_damage))))) ORDER BY i.position) FILTER (WHERE i.position IS NOT NULL), '[]'::jsonb) AS decks, r.created_at FROM deck_recommendations r JOIN simulation_jobs j ON j.id=r.simulation_job_id LEFT JOIN deck_recommendation_items i ON i.recommendation_id=r.id LEFT JOIN simulation_deck_results d ON d.id=i.simulation_deck_result_id WHERE j.player_id=$1 AND j.status='completed' AND j.boss_version=(SELECT version FROM current_boss WHERE singleton=TRUE) AND r.deck_count=$2 AND r.must_include_mirror_force=$3 AND r.must_include_team_tactics=$4 AND r.recommendation_phase=(CASE WHEN COALESCE($5::BOOLEAN, FALSE) THEN 'void' ELSE 'current' END)::recommendation_phase GROUP BY r.id ORDER BY r.created_at DESC LIMIT 1",
    )
    .bind(player_id)
    .bind(query.deck_count)
    .bind(query.must_include_mirror_force)
    .bind(query.must_include_team_tactics)
    .bind(query.include_body_phase)
    .fetch_optional(state.db()?)
    .await?
    .ok_or_else(|| AppError::NotFound("No completed recommendation found".to_string()))?;

    // `cards` isn't stored anywhere -- card_mask (a real column) is the sole
    // source of truth for deck identity, decoded back into card ids here at
    // response time instead of persisting a redundant copy.
    if let Some(decks) = recommendation.decks.as_array_mut() {
        for deck in decks.iter_mut() {
            let mask = deck.get("card_mask").and_then(Value::as_u64).unwrap_or(0);
            if let Some(deck) = deck.as_object_mut() {
                deck.remove("card_mask");
                deck.insert("cards".to_string(), serde_json::json!(cards_from_mask(mask)));
            }
        }
    }
    Ok(Json(recommendation))
}

#[cfg(test)]
#[path = "../../tests/unit/routes/recommendations_tests.rs"]
mod tests;
