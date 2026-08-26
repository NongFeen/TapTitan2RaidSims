use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};

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

pub async fn fetch_tt2_clan_stats(
    State(state): State<Arc<AppState>>,
) -> Result<(StatusCode, Json<Tt2ClanFetchResult>), AppError> {
    let _fetch_guard = state.clan_fetch_lock.lock().await;
    let tt2 = state.gamehive_api.as_ref().ok_or_else(|| {
        AppError::ServiceUnavailable("TT2 integration is not configured".to_string())
    })?;
    if !tt2.is_raid_connected() {
        return Err(AppError::ServiceUnavailable(
            "TT2 /raid socket is not connected".to_string(),
        ));
    }

    let last_fetched_at: Option<DateTime<Utc>> =
        sqlx::query_scalar("SELECT last_fetched_at FROM tt2_clan_sync_state WHERE singleton=TRUE")
            .fetch_one(state.db()?)
            .await?;
    if let Some(last_fetched_at) = last_fetched_at {
        let next_fetch_at = last_fetched_at + Duration::hours(12);
        if Utc::now() < next_fetch_at {
            return Err(AppError::TooManyRequests(format!(
                "Clan player data can be fetched again at {}",
                next_fetch_at.to_rfc3339()
            )));
        }
    }

    let clan = tt2.fetch_clan().await?;
    if clan.clan_code.trim().is_empty() || clan.clan_name.trim().is_empty() {
        return Err(AppError::BadRequest(
            "TT2 returned clan data without a clan code or name".to_string(),
        ));
    }

    let existing_codes: HashSet<String> = sqlx::query_scalar("SELECT player_id FROM players")
        .fetch_all(state.db()?)
        .await?
        .into_iter()
        .collect();
    let mut existing_stats = HashMap::new();
    for player_code in &existing_codes {
        if let Some(loaded) = player_stats_repo::load(state.db()?, player_code).await? {
            existing_stats.insert(player_code.clone(), loaded.data);
        }
    }

    let mut seen_codes = HashSet::new();
    let mut prepared_players = Vec::with_capacity(clan.players_data.len());
    for clan_player in clan.players_data {
        let player_code = clan_player.player_code.trim().to_string();
        let display_name = clan_player.name.trim().to_string();
        if player_code.is_empty() || display_name.is_empty() {
            return Err(AppError::BadRequest(
                "TT2 returned a clan player without a player code or name".to_string(),
            ));
        }
        if !seen_codes.insert(player_code.clone()) {
            return Err(AppError::BadRequest(format!(
                "TT2 returned duplicate clan player {player_code}"
            )));
        }
        let previous = existing_stats.get(&player_code);
        let title = previous.map_or(0.0, |stats| stats.title);
        let mut stats = clan_player.into_raid_data(title)?;
        if let Some(previous) = previous {
            preserve_card_preferences(&mut stats, previous);
        }
        validate_stats(&stats)?;
        prepared_players.push((player_code, display_name, stats));
    }

    let player_count = prepared_players.len();
    let created_players = prepared_players
        .iter()
        .filter(|(player_code, _, _)| !existing_codes.contains(player_code))
        .count();
    let updated_players = player_count - created_players;
    let fetched_at = Utc::now();
    let clan_code = clan.clan_code.trim().to_string();
    let clan_name = clan.clan_name.trim().to_string();
    let mut tx = state.db()?.begin().await?;
    for (player_code, display_name, stats) in prepared_players {
        sqlx::query(
            "INSERT INTO players (player_id, display_name, auto_sims) VALUES ($1,$2,FALSE) ON CONFLICT (player_id) DO UPDATE SET display_name=EXCLUDED.display_name, updated_at=NOW()",
        )
        .bind(&player_code)
        .bind(display_name)
        .execute(&mut *tx)
        .await?;
        player_stats_repo::store(&mut tx, &player_code, &stats).await?;
    }
    sqlx::query(
        "UPDATE tt2_clan_sync_state SET clan_code=$1, clan_name=$2, last_fetched_at=$3, last_player_count=$4, updated_at=NOW() WHERE singleton=TRUE",
    )
    .bind(&clan_code)
    .bind(&clan_name)
    .bind(fetched_at)
    .bind(player_count as i32)
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;

    Ok((
        StatusCode::CREATED,
        Json(Tt2ClanFetchResult {
            clan_code,
            clan_name,
            created_players,
            updated_players,
            player_count,
            last_fetched_at: fetched_at,
            next_fetch_at: fetched_at + Duration::hours(12),
        }),
    ))
}

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

fn preserve_card_preferences(refreshed: &mut PlayerRaidData, existing: &PlayerRaidData) {
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
#[path = "../../tests/unit/routes/players_tests.rs"]
mod tests;
