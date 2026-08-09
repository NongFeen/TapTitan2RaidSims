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
            PlayerSummary, Tt2PlayerStatus, UpdateAutoSimsRequest, UpdatePlayerStatsRequest,
            UpdatePlayerTokenRequest,
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
        "INSERT INTO players (id, player_id, display_name, auto_sims) VALUES ($1,$2,$3,$4) RETURNING player_id, display_name, auto_sims, NULL::BIGINT AS stats_revision, FALSE AS has_player_token, tt2_token_status AS player_token_status, tt2_last_fetched_at, created_at, updated_at",
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
        "SELECT p.player_id, p.display_name, p.auto_sims, s.revision AS stats_revision, p.player_token_ciphertext IS NOT NULL AS has_player_token, p.tt2_token_status AS player_token_status, p.tt2_last_fetched_at, p.created_at, p.updated_at FROM players p LEFT JOIN player_stats s ON s.player_id=p.id ORDER BY p.display_name",
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
        "SELECT p.player_id, p.display_name, p.auto_sims, s.revision AS stats_revision, s.stats, p.player_token_ciphertext IS NOT NULL AS has_player_token, p.tt2_token_status AS player_token_status, p.tt2_last_fetched_at, p.created_at, p.updated_at FROM players p LEFT JOIN player_stats s ON s.player_id=p.id WHERE p.player_id=$1",
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
    let stored = store_stats(&state, &player_id, stats).await?;
    enqueue_for_current_boss(&state, &player_id).await?;
    Ok((StatusCode::CREATED, Json(stored)))
}

async fn store_stats(
    state: &Arc<AppState>,
    player_id: &str,
    stats: PlayerRaidData,
) -> Result<PlayerStatsVersion, AppError> {
    let mut tx = state.db()?.begin().await?;
    let internal_id: Option<Uuid> =
        sqlx::query_scalar("SELECT id FROM players WHERE player_id=$1 FOR UPDATE")
            .bind(player_id)
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
    Ok(stored)
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
        "UPDATE players SET auto_sims=$2, updated_at=NOW() WHERE player_id=$1 RETURNING player_id, display_name, auto_sims, (SELECT revision FROM player_stats WHERE player_id=players.id) AS stats_revision, player_token_ciphertext IS NOT NULL AS has_player_token, tt2_token_status AS player_token_status, tt2_last_fetched_at, created_at, updated_at",
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

pub async fn update_token(
    State(state): State<Arc<AppState>>,
    Path(player_id): Path<String>,
    Json(request): Json<UpdatePlayerTokenRequest>,
) -> Result<Json<PlayerSummary>, AppError> {
    let token = request.player_token.trim();
    if token.is_empty() {
        return Err(AppError::BadRequest(
            "player_token cannot be empty".to_string(),
        ));
    }
    let tt2 = state.tt2_player.as_ref().ok_or_else(|| {
        AppError::ServiceUnavailable("TT2 integration is not configured".to_string())
    })?;
    let (ciphertext, nonce) = tt2.cipher().encrypt(token)?;
    let player = sqlx::query_as(
        "UPDATE players SET player_token_ciphertext=$2, player_token_nonce=$3, tt2_token_status='configured', tt2_last_fetched_at=NULL, updated_at=NOW() WHERE player_id=$1 RETURNING player_id, display_name, auto_sims, (SELECT revision FROM player_stats WHERE player_id=players.id) AS stats_revision, TRUE AS has_player_token, tt2_token_status AS player_token_status, tt2_last_fetched_at, created_at, updated_at",
    )
    .bind(&player_id).bind(ciphertext).bind(nonce)
    .fetch_optional(state.db()?).await?
    .ok_or_else(|| AppError::NotFound("Player not found".to_string()))?;
    Ok(Json(player))
}

pub async fn clear_token(
    State(state): State<Arc<AppState>>,
    Path(player_id): Path<String>,
) -> Result<Json<PlayerSummary>, AppError> {
    let player = sqlx::query_as(
        "UPDATE players SET player_token_ciphertext=NULL, player_token_nonce=NULL, tt2_token_status='missing', tt2_last_fetched_at=NULL, updated_at=NOW() WHERE player_id=$1 RETURNING player_id, display_name, auto_sims, (SELECT revision FROM player_stats WHERE player_id=players.id) AS stats_revision, FALSE AS has_player_token, tt2_token_status AS player_token_status, tt2_last_fetched_at, created_at, updated_at",
    )
    .bind(&player_id).fetch_optional(state.db()?).await?
    .ok_or_else(|| AppError::NotFound("Player not found".to_string()))?;
    Ok(Json(player))
}

pub async fn tt2_status(State(state): State<Arc<AppState>>) -> Json<Tt2PlayerStatus> {
    Json(Tt2PlayerStatus {
        configured: state.tt2_player.is_some(),
        connected: state
            .tt2_player
            .as_ref()
            .is_some_and(|client| client.is_connected()),
    })
}

pub async fn fetch_tt2_stats(
    State(state): State<Arc<AppState>>,
    Path(player_id): Path<String>,
) -> Result<(StatusCode, Json<PlayerStatsVersion>), AppError> {
    let tt2 = state.tt2_player.as_ref().ok_or_else(|| {
        AppError::ServiceUnavailable("TT2 integration is not configured".to_string())
    })?;
    if !tt2.is_connected() {
        return Err(AppError::ServiceUnavailable(
            "TT2 /player socket is not connected".to_string(),
        ));
    }
    let token_row: Option<(Vec<u8>, Vec<u8>)> = sqlx::query_as(
        "SELECT player_token_ciphertext, player_token_nonce FROM players WHERE player_id=$1 AND player_token_ciphertext IS NOT NULL AND player_token_nonce IS NOT NULL",
    ).bind(&player_id).fetch_optional(state.db()?).await?;
    let (ciphertext, nonce) = match token_row {
        Some(token) => token,
        None => {
            let exists: bool =
                sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM players WHERE player_id=$1)")
                    .bind(&player_id)
                    .fetch_one(state.db()?)
                    .await?;
            return Err(if exists {
                AppError::BadRequest("Player has no configured TT2 token".to_string())
            } else {
                AppError::NotFound("Player not found".to_string())
            });
        }
    };
    let reserved = sqlx::query(
        "UPDATE players SET tt2_last_fetched_at=NOW() WHERE player_id=$1 AND (tt2_last_fetched_at IS NULL OR tt2_last_fetched_at <= NOW() - INTERVAL '2 minutes')",
    ).bind(&player_id).execute(state.db()?).await?;
    if reserved.rows_affected() == 0 {
        return Err(AppError::TooManyRequests(
            "Player data can be fetched once every two minutes; try again later".to_string(),
        ));
    }
    let player_token = tt2.cipher().decrypt(&ciphertext, &nonce)?;
    let public = match tt2.fetch_player(&player_token).await {
        Ok(data) => data,
        Err(error) => {
            if matches!(&error, AppError::BadRequest(message) if message.contains("rejected the application or player token"))
            {
                sqlx::query("UPDATE players SET tt2_token_status='invalid' WHERE player_id=$1")
                    .bind(&player_id)
                    .execute(state.db()?)
                    .await?;
            }
            return Err(error);
        }
    };
    if public.player_code != player_id {
        return Err(AppError::Conflict(format!(
            "TT2 token belongs to player {}, not selected player {}",
            public.player_code, player_id
        )));
    }
    let title: Option<f32> = sqlx::query_scalar(
        "SELECT COALESCE((stats->>'title')::REAL, 0) FROM player_stats s JOIN players p ON p.id=s.player_id WHERE p.player_id=$1",
    ).bind(&player_id).fetch_optional(state.db()?).await?;
    let stats = public.into_raid_data(title.unwrap_or(0.0))?;
    validate_stats(&stats)?;
    let stored = store_stats(&state, &player_id, stats).await?;
    sqlx::query("UPDATE players SET tt2_token_status='configured' WHERE player_id=$1")
        .bind(&player_id)
        .execute(state.db()?)
        .await?;
    enqueue_for_current_boss(&state, &player_id).await?;
    Ok((StatusCode::CREATED, Json(stored)))
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
