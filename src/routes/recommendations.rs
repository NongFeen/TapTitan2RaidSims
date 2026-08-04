use std::sync::Arc;

use axum::{
    Json,
    extract::{Path, Query, State},
};
use serde::Deserialize;
use serde_json::Value;
use uuid::Uuid;

use crate::{error::AppError, models::app::RecommendationView, state::AppState};

#[derive(Deserialize)]
pub struct RecommendationQuery {
    #[serde(default = "default_deck_count")]
    deck_count: i32,
    #[serde(default)]
    must_include_mirror_force: bool,
    #[serde(default)]
    must_include_team_tactics: bool,
}

#[derive(Deserialize)]
pub struct DeckResultsQuery {
    #[serde(default = "default_limit")]
    limit: i64,
    #[serde(default)]
    offset: i64,
}

fn default_limit() -> i64 {
    100
}

fn default_deck_count() -> i32 {
    6
}

#[derive(Deserialize)]
pub struct GenerateRecommendationRequest {
    deck_count: i32,
}

#[derive(serde::Serialize)]
pub struct GenerateRecommendationResponse {
    deck_count: i32,
    created: bool,
}

pub async fn generate_for_player(
    State(state): State<Arc<AppState>>,
    Path(player_id): Path<String>,
    Json(request): Json<GenerateRecommendationRequest>,
) -> Result<Json<GenerateRecommendationResponse>, AppError> {
    if request.deck_count != 9 {
        return Err(AppError::BadRequest(
            "On-demand generation currently supports only deck_count 9".to_string(),
        ));
    }

    let created =
        crate::services::job_service::generate_nine_deck_recommendations(&state, &player_id)
            .await?;
    Ok(Json(GenerateRecommendationResponse {
        deck_count: request.deck_count,
        created,
    }))
}

pub async fn current_for_player(
    State(state): State<Arc<AppState>>,
    Path(player_id): Path<String>,
    Query(query): Query<RecommendationQuery>,
) -> Result<Json<RecommendationView>, AppError> {
    if !matches!(query.deck_count, 6 | 9) {
        return Err(AppError::BadRequest(
            "deck_count must be 6 or 9".to_string(),
        ));
    }
    let player_exists: bool =
        sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM players WHERE player_id=$1)")
            .bind(&player_id)
            .fetch_one(state.db()?)
            .await?;
    if !player_exists {
        return Err(AppError::NotFound("Player not found".to_string()));
    }
    let recommendation = sqlx::query_as(
        "SELECT r.id, r.simulation_job_id, r.deck_count, r.must_include_mirror_force, r.must_include_team_tactics, r.total_average_damage::TEXT AS total_average_damage, COALESCE(jsonb_agg(jsonb_build_object('position', i.position, 'cards', d.cards, 'average_damage', d.average_damage::TEXT, 'result', d.result) ORDER BY i.position) FILTER (WHERE i.position IS NOT NULL), '[]'::jsonb) AS decks, r.created_at FROM deck_recommendations r JOIN simulation_jobs j ON j.id=r.simulation_job_id JOIN players p ON p.id=j.player_id LEFT JOIN deck_recommendation_items i ON i.recommendation_id=r.id LEFT JOIN simulation_deck_results d ON d.id=i.simulation_deck_result_id WHERE p.player_id=$1 AND j.status='completed' AND j.id=(SELECT latest.id FROM simulation_jobs latest WHERE latest.player_id=p.id AND latest.status='completed' ORDER BY latest.completed_at DESC LIMIT 1) AND r.deck_count=$2 AND r.must_include_mirror_force=$3 AND r.must_include_team_tactics=$4 GROUP BY r.id ORDER BY r.created_at DESC LIMIT 1",
    )
    .bind(player_id)
    .bind(query.deck_count)
    .bind(query.must_include_mirror_force)
    .bind(query.must_include_team_tactics)
    .fetch_optional(state.db()?)
    .await?
    .ok_or_else(|| AppError::NotFound("No completed recommendation found".to_string()))?;
    Ok(Json(recommendation))
}

#[derive(serde::Serialize, sqlx::FromRow)]
pub struct DeckResultView {
    id: Uuid,
    cards: Value,
    average_damage: String,
    result: Value,
}

pub async fn deck_results(
    State(state): State<Arc<AppState>>,
    Path(job_id): Path<Uuid>,
    Query(query): Query<DeckResultsQuery>,
) -> Result<Json<Vec<DeckResultView>>, AppError> {
    if !(1..=500).contains(&query.limit) || query.offset < 0 {
        return Err(AppError::BadRequest(
            "limit must be 1..500 and offset cannot be negative".to_string(),
        ));
    }
    let results = sqlx::query_as(
        "SELECT id, cards, average_damage::TEXT AS average_damage, result FROM simulation_deck_results WHERE simulation_job_id=$1 ORDER BY average_damage DESC LIMIT $2 OFFSET $3",
    )
    .bind(job_id)
    .bind(query.limit)
    .bind(query.offset)
    .fetch_all(state.db()?)
    .await?;
    Ok(Json(results))
}
