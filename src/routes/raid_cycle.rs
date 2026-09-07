use std::sync::Arc;

use axum::{Json, extract::State};
use chrono::{DateTime, Utc};
use serde::Serialize;
use utoipa::ToSchema;

use crate::{error::AppError, state::AppState};

#[derive(Debug, Serialize, sqlx::FromRow, ToSchema)]
pub struct RaidCycleView {
    pub clan_code: String,
    pub raid_id: i64,
    pub started_at: Option<DateTime<Utc>>,
    pub raid_started_at: DateTime<Utc>,
    pub next_reset_at: DateTime<Utc>,
    pub morale_percent: f64,
    pub team_tactics_morale_percent: f64,
    pub default_morale_percent: f64,
    pub mirror_force_boost: f64,
    pub updated_at: DateTime<Utc>,
}

/// Get the current raid cycle
#[utoipa::path(
    get,
    path = "/api/raid-cycle/current",
    tag = "raids",
    responses(
        (status = 200, description = "Current raid cycle (morale, mirror force boost, reset time)", body = RaidCycleView),
        (status = 404, description = "No current raid cycle"),
    ),
)]
pub async fn current(State(state): State<Arc<AppState>>) -> Result<Json<RaidCycleView>, AppError> {
    let cycle = sqlx::query_as(
        "SELECT clan_code,raid_id,started_at,raid_started_at,next_reset_at,morale*100.0 AS morale_percent,team_tactics_morale_boost*100.0 AS team_tactics_morale_percent,(morale+team_tactics_morale_boost)*100.0 AS default_morale_percent,mirror_force_boost,updated_at FROM raid_cycle_state ORDER BY updated_at DESC LIMIT 1",
    )
    .fetch_optional(state.db()?)
    .await?
    .ok_or_else(|| AppError::NotFound("No current raid cycle".to_string()))?;
    Ok(Json(cycle))
}

#[derive(Debug, Serialize, sqlx::FromRow, ToSchema)]
pub struct CardUsageView {
    pub card_id: String,
    pub uses: i64,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct CycleAttackSummaryView {
    pub cycle: i32,
    pub total_damage: String,
    pub attack_count: i64,
    pub card_usage: Vec<CardUsageView>,
}

/// Get clan-wide attack totals for the current cycle
#[utoipa::path(
    get,
    path = "/api/raid-cycle/current/attack-summary",
    tag = "raids",
    responses(
        (status = 200, description = "Total damage, attack count, and card usage across the whole clan for the current cycle", body = CycleAttackSummaryView),
    ),
)]
pub async fn current_attack_summary(
    State(state): State<Arc<AppState>>,
) -> Result<Json<CycleAttackSummaryView>, AppError> {
    let raid_id: Option<i64> =
        sqlx::query_scalar("SELECT raid_id FROM raid_cycle_state ORDER BY updated_at DESC LIMIT 1")
            .fetch_optional(state.db()?)
            .await?;
    let Some(raid_id) = raid_id else {
        return Ok(Json(CycleAttackSummaryView {
            cycle: 0,
            total_damage: "0".to_string(),
            attack_count: 0,
            card_usage: Vec::new(),
        }));
    };

    let cycle: i32 =
        sqlx::query_scalar("SELECT COALESCE(MAX(cycle), 0) FROM raid_attack_logs WHERE raid_id=$1")
            .bind(raid_id)
            .fetch_one(state.db()?)
            .await?;

    let (total_damage, attack_count): (String, i64) = sqlx::query_as(
        "SELECT COALESCE(SUM(total_damage), 0)::TEXT, COUNT(*) FROM raid_attack_logs WHERE raid_id=$1 AND cycle=$2",
    )
    .bind(raid_id)
    .bind(cycle)
    .fetch_one(state.db()?)
    .await?;

    // card1/card2/card3 are separate slot columns on each attack row --
    // union them into one stream so a card counts once per attack it
    // appeared in, regardless of which slot it was played from.
    let card_usage: Vec<CardUsageView> = sqlx::query_as(
        "SELECT card_id, COUNT(*) AS uses FROM ( \
           SELECT c.card1 AS card_id FROM raid_attack_components c JOIN raid_attack_logs l ON l.raid_id=c.raid_id AND l.player_id=c.player_id AND l.attack_datetime=c.attack_datetime WHERE l.raid_id=$1 AND l.cycle=$2 AND c.card1 IS NOT NULL \
           UNION ALL \
           SELECT c.card2 FROM raid_attack_components c JOIN raid_attack_logs l ON l.raid_id=c.raid_id AND l.player_id=c.player_id AND l.attack_datetime=c.attack_datetime WHERE l.raid_id=$1 AND l.cycle=$2 AND c.card2 IS NOT NULL \
           UNION ALL \
           SELECT c.card3 FROM raid_attack_components c JOIN raid_attack_logs l ON l.raid_id=c.raid_id AND l.player_id=c.player_id AND l.attack_datetime=c.attack_datetime WHERE l.raid_id=$1 AND l.cycle=$2 AND c.card3 IS NOT NULL \
         ) AS all_cards GROUP BY card_id ORDER BY uses DESC",
    )
    .bind(raid_id)
    .bind(cycle)
    .fetch_all(state.db()?)
    .await?;

    Ok(Json(CycleAttackSummaryView {
        cycle,
        total_damage,
        attack_count,
        card_usage,
    }))
}
