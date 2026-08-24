use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

use crate::models::db_enums::{JobStatus, TokenStatus};

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct PlayerSummary {
    pub player_id: String,
    pub display_name: String,
    pub auto_sims: bool,
    pub stats_revision: Option<i64>,
    pub has_player_token: bool,
    pub player_token_status: TokenStatus,
    pub tt2_last_fetched_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct PlayerDetail {
    pub player_id: String,
    pub display_name: String,
    pub auto_sims: bool,
    pub stats_revision: Option<i64>,
    pub stats: Option<Value>,
    pub has_player_token: bool,
    pub player_token_status: TokenStatus,
    pub tt2_last_fetched_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateAutoSimsRequest {
    pub auto_sims: bool,
}

#[derive(Debug, Deserialize)]
pub struct UpdatePlayerTokenRequest {
    pub player_token: String,
}

#[derive(Debug, Serialize)]
pub struct Tt2PlayerStatus {
    pub configured: bool,
    pub connected: bool,
    pub raid_connected: bool,
}

#[derive(Debug, Serialize)]
pub struct Tt2ClanStatus {
    pub clan_code: Option<String>,
    pub clan_name: Option<String>,
    pub last_fetched_at: Option<DateTime<Utc>>,
    pub next_fetch_at: Option<DateTime<Utc>>,
    pub last_player_count: i32,
}

#[derive(Debug, Serialize)]
pub struct Tt2ClanFetchResult {
    pub clan_code: String,
    pub clan_name: String,
    pub created_players: usize,
    pub updated_players: usize,
    pub player_count: usize,
    pub last_fetched_at: DateTime<Utc>,
    pub next_fetch_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum UpdatePlayerStatsRequest {
    Cleaned(crate::models::player_raid_data::PlayerRaidData),
    Raw(crate::models::player_data::PlayerData),
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct PlayerStatsVersion {
    pub revision: i64,
    pub stats: Value,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct SimulationJobView {
    pub id: Uuid,
    pub player_id: String,
    pub simulator_version: String,
    pub status: JobStatus,
    pub result: Option<Value>,
    pub error_message: Option<String>,
    pub attempts: i32,
    pub created_at: DateTime<Utc>,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct CreateSimulationJobRequest {
    pub player_id: String,
    #[serde(default)]
    pub include_body_phase: bool,
}

#[derive(Debug, Deserialize)]
pub struct CurrentBossUpdateRequest {
    pub boss_data: crate::models::boss::Boss,
    pub attackable_parts: Vec<crate::models::boss::BossPartName>,
    #[serde(default)]
    pub run_sims: bool,
}

#[derive(Debug, Serialize)]
pub struct RaidEventAccepted {
    pub status: &'static str,
    pub message: String,
    pub simulations_triggered: bool,
    pub deleted_jobs: u64,
    pub created_jobs: Vec<Uuid>,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct CurrentBossView {
    pub boss_data: Value,
    pub attackable_parts: Value,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize)]
pub struct LiveAttackBossView {
    pub clan_code: String,
    pub raid_id: i64,
    pub cycle: i32,
    pub titan_index: i32,
    pub boss_data: Value,
    pub received_at: DateTime<Utc>,
    pub display_parts: Option<Vec<LiveBossDisplayPart>>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct LiveBossDisplayPart {
    pub part_name: crate::models::boss::BossPartName,
    pub part_state: crate::models::boss::PartState,
    pub current_hp: u64,
    pub max_hp: u64,
    pub is_targeted: bool,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct RecommendationView {
    pub id: Uuid,
    pub simulation_job_id: Uuid,
    pub deck_count: i32,
    pub must_include_mirror_force: bool,
    pub must_include_team_tactics: bool,
    pub total_average_damage: String,
    pub body_phase_ran: bool,
    pub decks: Value,
    pub created_at: DateTime<Utc>,
}
