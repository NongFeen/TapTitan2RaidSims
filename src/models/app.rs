use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct PlayerSummary {
    pub player_id: String,
    pub display_name: String,
    pub auto_sims: bool,
    pub stats_revision: Option<i64>,
    pub has_player_token: bool,
    pub player_token_status: String,
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
    pub player_token_status: String,
    pub tt2_last_fetched_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct CreatePlayerRequest {
    pub player_id: String,
    pub display_name: String,
    #[serde(default)]
    pub auto_sims: bool,
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
    pub status: String,
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

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct RecommendationView {
    pub id: Uuid,
    pub simulation_job_id: Uuid,
    pub deck_count: i32,
    pub must_include_mirror_force: bool,
    pub must_include_team_tactics: bool,
    pub total_average_damage: String,
    pub decks: Value,
    pub created_at: DateTime<Utc>,
}
