use std::sync::Arc;

use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
};
use chrono::{DateTime, Duration, Utc};

use crate::{
    error::AppError,
    models::{
        app::{
            CreateSimulationJobRequest, PlayerDetail, PlayerStatsVersion, PlayerSummary,
            Tt2ClanFetchResult, Tt2ClanStatus, Tt2PlayerStatus, UpdateAutoSimsRequest,
            UpdatePlayerStatsRequest, UpdatePlayerTokenRequest,
        },
        player_raid_data::PlayerRaidData,
    },
    services::{player_stats_repo, taptitan::player_service::clean_data},
    state::AppState,
};

/// List all players
#[utoipa::path(
    get,
    path = "/api/players",
    tag = "players",
    responses((status = 200, description = "All known players", body = [PlayerSummary])),
)]
pub async fn list(
    State(state): State<Arc<AppState>>,
) -> Result<Json<Vec<PlayerSummary>>, AppError> {
    let players = sqlx::query_as(
        "SELECT p.player_id, p.display_name, p.auto_sims, s.revision AS stats_revision, p.player_token_ciphertext IS NOT NULL AS has_player_token, p.tt2_token_status AS player_token_status, p.tt2_last_fetched_at, p.created_at, p.updated_at FROM players p LEFT JOIN player_stats s ON s.player_id=p.player_id ORDER BY p.display_name",
    )
    .fetch_all(state.db()?)
    .await?;
    Ok(Json(players))
}

#[derive(sqlx::FromRow)]
struct PlayerDetailRow {
    player_id: String,
    display_name: String,
    auto_sims: bool,
    stats_revision: Option<i64>,
    has_player_token: bool,
    player_token_status: crate::models::db_enums::TokenStatus,
    tt2_last_fetched_at: Option<DateTime<Utc>>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

/// Get a player's detail
#[utoipa::path(
    get,
    path = "/api/players/{player_id}",
    tag = "players",
    params(("player_id" = String, Path, description = "Player id (TT2 player code)")),
    responses(
        (status = 200, description = "Player detail with current stats", body = PlayerDetail),
        (status = 404, description = "Player not found"),
    ),
)]
pub async fn get(
    State(state): State<Arc<AppState>>,
    Path(player_id): Path<String>,
) -> Result<Json<PlayerDetail>, AppError> {
    let row: PlayerDetailRow = sqlx::query_as(
        "SELECT p.player_id, p.display_name, p.auto_sims, s.revision AS stats_revision, p.player_token_ciphertext IS NOT NULL AS has_player_token, p.tt2_token_status AS player_token_status, p.tt2_last_fetched_at, p.created_at, p.updated_at FROM players p LEFT JOIN player_stats s ON s.player_id=p.player_id WHERE p.player_id=$1",
    )
    .bind(player_id)
    .fetch_optional(state.db()?)
    .await?
    .ok_or_else(|| AppError::NotFound("Player not found".to_string()))?;
    let stats = player_stats_repo::load(state.db()?, &row.player_id)
        .await?
        .map(|loaded| serde_json::to_value(&loaded.data))
        .transpose()?;
    Ok(Json(PlayerDetail {
        player_id: row.player_id,
        display_name: row.display_name,
        auto_sims: row.auto_sims,
        stats_revision: row.stats_revision,
        stats,
        has_player_token: row.has_player_token,
        player_token_status: row.player_token_status,
        tt2_last_fetched_at: row.tt2_last_fetched_at,
        created_at: row.created_at,
        updated_at: row.updated_at,
    }))
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
    Ok((StatusCode::CREATED, Json(stored)))
}

async fn store_stats(
    state: &Arc<AppState>,
    player_id: &str,
    stats: PlayerRaidData,
) -> Result<PlayerStatsVersion, AppError> {
    let mut tx = state.db()?.begin().await?;
    let locked: Option<String> =
        sqlx::query_scalar("SELECT player_id FROM players WHERE player_id=$1 FOR UPDATE")
            .bind(player_id)
            .fetch_optional(&mut *tx)
            .await?;
    locked.ok_or_else(|| AppError::NotFound("Player not found".to_string()))?;
    let stored = player_stats_repo::store(&mut tx, player_id, &stats).await?;
    sqlx::query("UPDATE players SET updated_at=NOW() WHERE player_id=$1")
        .bind(player_id)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    Ok(PlayerStatsVersion {
        revision: stored.revision,
        stats: serde_json::to_value(&stored.data)?,
        created_at: stored.created_at,
        updated_at: stored.updated_at,
    })
}

/// Get a player's current stats revision
#[utoipa::path(
    get,
    path = "/api/players/{player_id}/stats/current",
    tag = "players",
    params(("player_id" = String, Path, description = "Player id")),
    responses(
        (status = 200, description = "Current stored raid stats", body = PlayerStatsVersion),
        (status = 404, description = "Player not found, or has no current stats"),
    ),
)]
pub async fn current_stats(
    State(state): State<Arc<AppState>>,
    Path(player_id): Path<String>,
) -> Result<Json<PlayerStatsVersion>, AppError> {
    let player_exists: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM players WHERE player_id=$1)")
        .bind(&player_id)
        .fetch_one(state.db()?)
        .await?;
    if !player_exists {
        return Err(AppError::NotFound("Player not found".to_string()));
    }
    let loaded = player_stats_repo::load(state.db()?, &player_id)
        .await?
        .ok_or_else(|| AppError::NotFound("Player has no current stats".to_string()))?;
    Ok(Json(PlayerStatsVersion {
        revision: loaded.revision,
        stats: serde_json::to_value(&loaded.data)?,
        created_at: loaded.created_at,
        updated_at: loaded.updated_at,
    }))
}

pub async fn update_auto_sims(
    State(state): State<Arc<AppState>>,
    Path(player_id): Path<String>,
    Json(request): Json<UpdateAutoSimsRequest>,
) -> Result<Json<PlayerSummary>, AppError> {
    let player = sqlx::query_as(
        "UPDATE players SET auto_sims=$2, updated_at=NOW() WHERE player_id=$1 RETURNING player_id, display_name, auto_sims, (SELECT revision FROM player_stats WHERE player_id=players.player_id) AS stats_revision, player_token_ciphertext IS NOT NULL AS has_player_token, tt2_token_status AS player_token_status, tt2_last_fetched_at, created_at, updated_at",
    )
    .bind(&player_id)
    .bind(request.auto_sims)
    .fetch_optional(state.db()?)
    .await?
    .ok_or_else(|| AppError::NotFound("Player not found".to_string()))?;
    // if request.auto_sims {
    //     enqueue_for_current_boss(&state, &player_id).await?;
    // }
    Ok(Json(player))
}

/// Set a player's TT2 token (internal)
#[utoipa::path(
    put,
    path = "/internal/players/{player_id}/token",
    tag = "internal",
    params(("player_id" = String, Path, description = "Player id")),
    request_body = UpdatePlayerTokenRequest,
    responses(
        (status = 200, description = "Token stored (encrypted at rest)", body = PlayerSummary),
        (status = 404, description = "Player not found"),
    ),
    security(("internal_api_key" = [])),
)]
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
    let tt2 = state.gamehive_api.as_ref().ok_or_else(|| {
        AppError::ServiceUnavailable("TT2 integration is not configured".to_string())
    })?;
    let (ciphertext, nonce) = tt2.cipher().encrypt(token)?;
    let player = sqlx::query_as(
        "UPDATE players SET player_token_ciphertext=$2, player_token_nonce=$3, tt2_token_status='configured', tt2_last_fetched_at=NULL, updated_at=NOW() WHERE player_id=$1 RETURNING player_id, display_name, auto_sims, (SELECT revision FROM player_stats WHERE player_id=players.player_id) AS stats_revision, TRUE AS has_player_token, tt2_token_status AS player_token_status, tt2_last_fetched_at, created_at, updated_at",
    )
    .bind(&player_id).bind(ciphertext).bind(nonce)
    .fetch_optional(state.db()?).await?
    .ok_or_else(|| AppError::NotFound("Player not found".to_string()))?;
    Ok(Json(player))
}

/// Clear a player's TT2 token (internal)
#[utoipa::path(
    delete,
    path = "/internal/players/{player_id}/token",
    tag = "internal",
    params(("player_id" = String, Path, description = "Player id")),
    responses(
        (status = 200, description = "Token cleared", body = PlayerSummary),
        (status = 404, description = "Player not found"),
    ),
    security(("internal_api_key" = [])),
)]
pub async fn clear_token(
    State(state): State<Arc<AppState>>,
    Path(player_id): Path<String>,
) -> Result<Json<PlayerSummary>, AppError> {
    let player = sqlx::query_as(
        "UPDATE players SET player_token_ciphertext=NULL, player_token_nonce=NULL, tt2_token_status='missing', tt2_last_fetched_at=NULL, updated_at=NOW() WHERE player_id=$1 RETURNING player_id, display_name, auto_sims, (SELECT revision FROM player_stats WHERE player_id=players.player_id) AS stats_revision, FALSE AS has_player_token, tt2_token_status AS player_token_status, tt2_last_fetched_at, created_at, updated_at",
    )
    .bind(&player_id).fetch_optional(state.db()?).await?
    .ok_or_else(|| AppError::NotFound("Player not found".to_string()))?;
    Ok(Json(player))
}

/// TT2 player socket status (internal)
#[utoipa::path(
    get,
    path = "/internal/tt2/player-status",
    tag = "internal",
    responses((status = 200, description = "Whether the TT2 /player socket is configured/connected", body = Tt2PlayerStatus)),
    security(("internal_api_key" = [])),
)]
pub async fn tt2_status(State(state): State<Arc<AppState>>) -> Json<Tt2PlayerStatus> {
    Json(Tt2PlayerStatus {
        configured: state.gamehive_api.is_some(),
        connected: state
            .gamehive_api
            .as_ref()
            .is_some_and(|client| client.is_connected()),
        raid_connected: state
            .gamehive_api
            .as_ref()
            .is_some_and(|client| client.is_raid_connected()),
    })
}

/// TT2 clan sync status (internal)
#[utoipa::path(
    get,
    path = "/internal/tt2/clan-status",
    tag = "internal",
    responses((status = 200, description = "Last clan-wide player data fetch, and when the next one is allowed", body = Tt2ClanStatus)),
    security(("internal_api_key" = [])),
)]
pub async fn tt2_clan_status(
    State(state): State<Arc<AppState>>,
) -> Result<Json<Tt2ClanStatus>, AppError> {
    let row: (Option<String>, Option<String>, Option<DateTime<Utc>>, i32) = sqlx::query_as(
        "SELECT clan_code, clan_name, last_fetched_at, last_player_count FROM tt2_clan_sync_state WHERE singleton=TRUE",
    )
    .fetch_one(state.db()?)
    .await?;
    Ok(Json(Tt2ClanStatus {
        clan_code: row.0,
        clan_name: row.1,
        last_fetched_at: row.2,
        next_fetch_at: row.2.map(|value| value + Duration::hours(12)),
        last_player_count: row.3,
    }))
}

/// Fetch fresh clan player data from TT2 (internal)
///
/// Rate-limited to once every 12 hours; requires the TT2 `/raid` socket to
/// be connected.
#[utoipa::path(
    post,
    path = "/internal/tt2/fetch-clan-stats",
    tag = "internal",
    responses(
        (status = 201, description = "Clan player data fetched and stored", body = Tt2ClanFetchResult),
        (status = 429, description = "Called again before the 12-hour cooldown elapsed"),
        (status = 503, description = "TT2 integration not configured, or /raid socket not connected"),
    ),
    security(("internal_api_key" = [])),
)]
pub async fn fetch_tt2_clan_stats(
    State(state): State<Arc<AppState>>,
) -> Result<(StatusCode, Json<Tt2ClanFetchResult>), AppError> {
    let result = crate::services::clan_sync_service::fetch_and_store_clan_stats(&state).await?;
    Ok((StatusCode::CREATED, Json(result)))
}

/// Fetch a player's fresh stats from TT2 (internal)
///
/// Rate-limited to once every 2 minutes per player; requires a configured
/// player token and a connected TT2 `/player` socket.
#[utoipa::path(
    post,
    path = "/internal/players/{player_id}/fetch-stats",
    tag = "internal",
    params(("player_id" = String, Path, description = "Player id")),
    responses(
        (status = 201, description = "Fresh stats fetched and stored", body = PlayerStatsVersion),
        (status = 404, description = "Player not found"),
        (status = 429, description = "Called again before the 2-minute cooldown elapsed"),
    ),
    security(("internal_api_key" = [])),
)]
pub async fn fetch_tt2_stats(
    State(state): State<Arc<AppState>>,
    Path(player_id): Path<String>,
) -> Result<(StatusCode, Json<PlayerStatsVersion>), AppError> {
    let tt2 = state.gamehive_api.as_ref().ok_or_else(|| {
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
    let existing_stats = player_stats_repo::load(state.db()?, &player_id)
        .await?
        .map(|loaded| loaded.data);
    let title = existing_stats.as_ref().map_or(0.0, |stats| stats.title);
    let mut stats = public.into_raid_data(title)?;
    if let Some(existing_stats) = &existing_stats {
        preserve_card_preferences(&mut stats, existing_stats);
    }
    validate_stats(&stats)?;
    let stored = store_stats(&state, &player_id, stats).await?;
    sqlx::query("UPDATE players SET tt2_token_status='configured' WHERE player_id=$1")
        .bind(&player_id)
        .execute(state.db()?)
        .await?;
    Ok((StatusCode::CREATED, Json(stored)))
}

pub(crate) fn preserve_card_preferences(refreshed: &mut PlayerRaidData, existing: &PlayerRaidData) {
    let enabled_by_card = existing
        .card_list
        .iter()
        .map(|card| (card.card_id, card.enabled))
        .collect::<std::collections::HashMap<_, _>>();

    for card in &mut refreshed.card_list {
        if let Some(enabled) = enabled_by_card.get(&card.card_id) {
            card.enabled = *enabled;
        }
    }
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
                include_body_phase: false,
            },
        )
        .await?;
    }
    Ok(())
}

pub(crate) fn validate_stats(stats: &PlayerRaidData) -> Result<(), AppError> {
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
#[path = "../../tests/unit/routes/players_tests.rs"]
mod tests;
