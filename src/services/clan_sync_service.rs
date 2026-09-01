use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
    time::Duration as StdDuration,
};

use chrono::{DateTime, Duration, FixedOffset, Utc};

use crate::{
    error::AppError,
    models::app::Tt2ClanFetchResult,
    routes::players::{preserve_card_preferences, validate_stats},
    services::player_stats_repo,
    state::AppState,
};

/// Thailand doesn't observe DST, so a fixed UTC+7 offset is exact -- no
/// chrono-tz/timezone-database dependency needed.
const THAILAND_OFFSET_SECONDS: i32 = 7 * 3600;
const CLAN_FETCH_INTERVAL: StdDuration = StdDuration::from_secs(24 * 3600);

/// Fetches fresh clan player data from TT2 and upserts every player's
/// stats. Shared by the manual `POST /internal/tt2/fetch-clan-stats` route
/// and `spawn_scheduled_clan_fetch`'s daily timer -- same 12-hour cooldown
/// applies either way, so a manual fetch shortly before the scheduled tick
/// simply makes that tick a no-op (returns `TooManyRequests`).
pub async fn fetch_and_store_clan_stats(state: &Arc<AppState>) -> Result<Tt2ClanFetchResult, AppError> {
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

    Ok(Tt2ClanFetchResult {
        clan_code,
        clan_name,
        created_players,
        updated_players,
        player_count,
        last_fetched_at: fetched_at,
        next_fetch_at: fetched_at + Duration::hours(12),
    })
}

fn next_thai_midnight_utc(now: DateTime<Utc>) -> DateTime<Utc> {
    let thai_offset = FixedOffset::east_opt(THAILAND_OFFSET_SECONDS).expect("valid fixed offset");
    let now_thai = now.with_timezone(&thai_offset);
    let next_midnight_thai = (now_thai.date_naive() + Duration::days(1))
        .and_hms_opt(0, 0, 0)
        .expect("00:00:00 is always a valid time")
        .and_local_timezone(thai_offset)
        .single()
        .expect("a fixed offset is never ambiguous");
    next_midnight_thai.with_timezone(&Utc)
}

/// Runs `fetch_and_store_clan_stats` once at the next Thai (UTC+7) midnight,
/// then every 24 hours after that for as long as the process runs. Errors
/// (TT2 not configured, socket down, still in cooldown) are logged and
/// skipped rather than treated as fatal -- this loop never stops on its
/// own, so the next day's tick just tries again.
pub fn spawn_scheduled_clan_fetch(state: Arc<AppState>) {
    tokio::spawn(async move {
        let now = Utc::now();
        let first_run_at = next_thai_midnight_utc(now);
        let initial_delay = (first_run_at - now).to_std().unwrap_or(StdDuration::ZERO);
        tracing::info!(
            first_run_at = %first_run_at.to_rfc3339(),
            "scheduled clan stats fetch armed for the next Thai midnight"
        );
        tokio::time::sleep(initial_delay).await;
        loop {
            match fetch_and_store_clan_stats(&state).await {
                Ok(result) => tracing::info!(
                    clan_code = %result.clan_code,
                    player_count = result.player_count,
                    created_players = result.created_players,
                    updated_players = result.updated_players,
                    "scheduled clan stats fetch completed"
                ),
                Err(error) => tracing::warn!(?error, "scheduled clan stats fetch skipped"),
            }
            tokio::time::sleep(CLAN_FETCH_INTERVAL).await;
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    fn utc(y: i32, m: u32, d: u32, h: u32, min: u32, s: u32) -> DateTime<Utc> {
        Utc.with_ymd_and_hms(y, m, d, h, min, s).unwrap()
    }

    #[test]
    fn thai_midnight_is_17_00_utc_the_day_before() {
        // 00:00 ICT (UTC+7) is 17:00 UTC the previous calendar day.
        let now = utc(2026, 1, 15, 10, 0, 0); // 17:00 ICT Jan 15
        assert_eq!(next_thai_midnight_utc(now), utc(2026, 1, 15, 17, 0, 0));
    }

    #[test]
    fn just_before_thai_midnight_still_targets_the_upcoming_one() {
        let now = utc(2026, 1, 15, 16, 59, 0); // 23:59 ICT Jan 15
        assert_eq!(next_thai_midnight_utc(now), utc(2026, 1, 15, 17, 0, 0));
    }

    #[test]
    fn exactly_at_thai_midnight_schedules_the_following_day() {
        let now = utc(2026, 1, 15, 17, 0, 0); // exactly 00:00 ICT Jan 16
        assert_eq!(next_thai_midnight_utc(now), utc(2026, 1, 16, 17, 0, 0));
    }

    #[test]
    fn consecutive_runs_are_exactly_24_hours_apart() {
        let now = utc(2026, 3, 1, 5, 30, 0);
        let first = next_thai_midnight_utc(now);
        let second = next_thai_midnight_utc(first);
        assert_eq!(second - first, Duration::hours(24));
    }
}
