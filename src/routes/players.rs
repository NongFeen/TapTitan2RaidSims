use std::sync::Arc;

use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
};
use uuid::Uuid;

use crate::{
    error::AppError,
    models::{
        app::{
            CreatePlayerRequest, CreateSimulationJobRequest, PlayerDetail, PlayerStatsVersion,
            PlayerSummary, UpdateAutoSimsRequest, UpdatePlayerStatsRequest,
        },
        player_raid_data::PlayerRaidData,
    },
    services::taptitan::player_service::clean_data,
    state::AppState,
};

pub async fn create(
    State(state): State<Arc<AppState>>,
    Json(request): Json<CreatePlayerRequest>,
) -> Result<(StatusCode, Json<PlayerSummary>), AppError> {
    let display_name = request.display_name.trim();
    if display_name.is_empty() {
        return Err(AppError::BadRequest(
            "display_name cannot be empty".to_string(),
        ));
    }
    let game_player_id = request.player_id.trim();
    if game_player_id.is_empty() {
        return Err(AppError::BadRequest(
            "player_id cannot be empty when provided".to_string(),
        ));
    }
    let id = Uuid::new_v4();
    let player = sqlx::query_as(
        "INSERT INTO players (id, player_id, display_name, auto_sims) VALUES ($1,$2,$3,$4) RETURNING player_id, display_name, auto_sims, NULL::BIGINT AS stats_revision, created_at, updated_at",
    )
    .bind(id)
    .bind(game_player_id)
    .bind(display_name)
    .bind(request.auto_sims)
    .fetch_one(state.db()?)
    .await
    .map_err(|error| {
        if error
            .as_database_error()
            .is_some_and(|database| database.is_unique_violation())
        {
            AppError::Conflict("A player with this player_id already exists".to_string())
        } else {
            AppError::Database(error)
        }
    })?;
    Ok((StatusCode::CREATED, Json(player)))
}

pub async fn list(
    State(state): State<Arc<AppState>>,
) -> Result<Json<Vec<PlayerSummary>>, AppError> {
    let players = sqlx::query_as(
        "SELECT p.player_id, p.display_name, p.auto_sims, s.revision AS stats_revision, p.created_at, p.updated_at FROM players p LEFT JOIN player_stats s ON s.player_id=p.id ORDER BY p.display_name",
    )
    .fetch_all(state.db()?)
    .await?;
    Ok(Json(players))
}

pub async fn get(
    State(state): State<Arc<AppState>>,
    Path(player_id): Path<String>,
) -> Result<Json<PlayerDetail>, AppError> {
    let player = sqlx::query_as(
        "SELECT p.player_id, p.display_name, p.auto_sims, s.revision AS stats_revision, s.stats, p.created_at, p.updated_at FROM players p LEFT JOIN player_stats s ON s.player_id=p.id WHERE p.player_id=$1",
    )
    .bind(player_id)
    .fetch_optional(state.db()?)
    .await?
    .ok_or_else(|| AppError::NotFound("Player not found".to_string()))?;
    Ok(Json(player))
}

pub async fn update_stats(
    State(state): State<Arc<AppState>>,
    Path(player_id): Path<String>,
    Json(request): Json<UpdatePlayerStatsRequest>,
) -> Result<(StatusCode, Json<PlayerStatsVersion>), AppError> {
    let stats = match request {
        UpdatePlayerStatsRequest::Cleaned(stats) => stats,
        UpdatePlayerStatsRequest::Raw(raw) => clean_data(&raw),
    };
    validate_stats(&stats)?;
    let mut tx = state.db()?.begin().await?;
    let internal_id: Option<Uuid> =
        sqlx::query_scalar("SELECT id FROM players WHERE player_id=$1 FOR UPDATE")
            .bind(&player_id)
            .fetch_optional(&mut *tx)
            .await?;
    let internal_id =
        internal_id.ok_or_else(|| AppError::NotFound("Player not found".to_string()))?;
    let stored = sqlx::query_as(
        "INSERT INTO player_stats (player_id, revision, stats) VALUES ($1,1,$2) ON CONFLICT (player_id) DO UPDATE SET revision=player_stats.revision+1, stats=EXCLUDED.stats, updated_at=NOW() RETURNING revision, stats, created_at, updated_at",
    )
    .bind(internal_id)
    .bind(serde_json::to_value(stats)?)
    .fetch_one(&mut *tx)
    .await?;
    sqlx::query("UPDATE players SET updated_at=NOW() WHERE id=$1")
        .bind(internal_id)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    enqueue_for_current_boss(&state, &player_id).await?;
    Ok((StatusCode::CREATED, Json(stored)))
}

pub async fn current_stats(
    State(state): State<Arc<AppState>>,
    Path(player_id): Path<String>,
) -> Result<Json<PlayerStatsVersion>, AppError> {
    let stats = sqlx::query_as("SELECT s.revision, s.stats, s.created_at, s.updated_at FROM player_stats s JOIN players p ON p.id=s.player_id WHERE p.player_id=$1")
        .bind(&player_id)
        .fetch_optional(state.db()?)
        .await?
        .ok_or_else(|| AppError::NotFound("Player has no current stats".to_string()))?;
    Ok(Json(stats))
}

pub async fn update_auto_sims(
    State(state): State<Arc<AppState>>,
    Path(player_id): Path<String>,
    Json(request): Json<UpdateAutoSimsRequest>,
) -> Result<Json<PlayerSummary>, AppError> {
    let player = sqlx::query_as(
        "UPDATE players SET auto_sims=$2, updated_at=NOW() WHERE player_id=$1 RETURNING player_id, display_name, auto_sims, (SELECT revision FROM player_stats WHERE player_id=players.id) AS stats_revision, created_at, updated_at",
    )
    .bind(&player_id)
    .bind(request.auto_sims)
    .fetch_optional(state.db()?)
    .await?
    .ok_or_else(|| AppError::NotFound("Player not found".to_string()))?;
    if request.auto_sims {
        enqueue_for_current_boss(&state, &player_id).await?;
    }
    Ok(Json(player))
}

async fn enqueue_for_current_boss(state: &Arc<AppState>, player_id: &str) -> Result<(), AppError> {
    let can_simulate: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM current_boss cb JOIN players p ON p.player_id=$1 WHERE cb.singleton=TRUE AND p.auto_sims=TRUE)",
    )
    .bind(player_id)
    .fetch_one(state.db()?)
    .await?;
    if can_simulate {
        crate::services::job_service::create_job(
            state,
            CreateSimulationJobRequest {
                player_id: player_id.to_string(),
            },
        )
        .await?;
    }
    Ok(())
}

fn validate_stats(stats: &PlayerRaidData) -> Result<(), AppError> {
    if stats.player_raid_level == 0 {
        return Err(AppError::BadRequest(
            "player_raid_level must be greater than zero".to_string(),
        ));
    }
    if stats.card_list.is_empty() {
        return Err(AppError::BadRequest(
            "card_list cannot be empty".to_string(),
        ));
    }
    let mut cards: Vec<_> = stats.card_list.iter().map(|card| card.card_id).collect();
    cards.sort();
    cards.dedup();
    if cards.len() != stats.card_list.len() {
        return Err(AppError::BadRequest(
            "card_list contains duplicate cards".to_string(),
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_raw_player_export_for_stats_update() {
        let request: UpdatePlayerStatsRequest =
            serde_json::from_str(include_str!("../../playerDataSample.json"))
                .expect("raw player sample should deserialize");
        let UpdatePlayerStatsRequest::Raw(raw) = request else {
            panic!("raw sample was mistaken for cleaned stats");
        };
        let cleaned = clean_data(&raw);
        validate_stats(&cleaned).expect("converted raw player stats should be valid");

        let level = |card_name| {
            cleaned
                .card_list
                .iter()
                .find(|card| card.card_id == card_name)
                .map(|card| card.level)
                .expect("sample card should exist")
        };
        assert_eq!(level(crate::models::cards::CardName::GuardBreak), 48);
        assert_eq!(level(crate::models::cards::CardName::BarbedMorningstar), 48);
        assert_eq!(level(crate::models::cards::CardName::ElectroZap), 47);
        assert_eq!(level(crate::models::cards::CardName::CorrosiveBubbles), 47);
        assert_eq!(level(crate::models::cards::CardName::BattleDrums), 49);
        assert_eq!(level(crate::models::cards::CardName::CrushingInstinct), 49);
        assert_eq!(level(crate::models::cards::CardName::SoulFire), 49);
        assert_eq!(level(crate::models::cards::CardName::RancidGas), 49);
        assert_eq!(level(crate::models::cards::CardName::MoonBeam), 48);
    }

    #[test]
    fn accepts_older_raw_export_without_boosted_cards() {
        let mut value: serde_json::Value =
            serde_json::from_str(include_str!("../../playerDataSample.json"))
                .expect("raw player sample should be valid JSON");
        value
            .as_object_mut()
            .expect("raw player sample should be an object")
            .remove("boostedCards");

        let raw: crate::models::player_data::PlayerData =
            serde_json::from_value(value).expect("older raw export should still deserialize");
        assert!(raw.boosted_cards.is_empty());
    }
}
