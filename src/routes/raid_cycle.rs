use std::sync::Arc;

use axum::{Json, extract::State};
use chrono::{DateTime, Utc};
use serde::Serialize;

use crate::{error::AppError, state::AppState};

#[derive(Debug, Serialize, sqlx::FromRow)]
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

pub async fn current(State(state): State<Arc<AppState>>) -> Result<Json<RaidCycleView>, AppError> {
    let cycle = sqlx::query_as(
        "SELECT clan_code,raid_id,started_at,raid_started_at,next_reset_at,morale*100.0 AS morale_percent,team_tactics_morale_boost*100.0 AS team_tactics_morale_percent,(morale+team_tactics_morale_boost)*100.0 AS default_morale_percent,mirror_force_boost,updated_at FROM raid_cycle_state ORDER BY updated_at DESC LIMIT 1",
    )
    .fetch_optional(state.db()?)
    .await?
    .ok_or_else(|| AppError::NotFound("No current raid cycle".to_string()))?;
    Ok(Json(cycle))
}
