use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct PlayerSummary {
    pub player_id: String,
    pub display_name: String,
    pub auto_sims: bool,
    pub latest_stats_version: Option<i64>,
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

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct PlayerStatsVersion {
    pub version: i64,
    pub stats: Value,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct SimulationJobView {
    pub id: Uuid,
    pub player_id: String,
    pub player_stat_version_id: Uuid,
    pub raid_boss_id: Option<Uuid>,
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
    pub raid_boss_id: Uuid,
}

#[derive(Debug, Deserialize)]
pub struct RaidBossEventRequest {
    pub raid_external_id: String,
    pub raid_name: String,
    pub event_id: String,
    pub boss_data: crate::models::boss::Boss,
    pub attackable_parts: Vec<crate::models::boss::BossPartName>,
}

#[derive(Debug, Serialize)]
pub struct RaidEventAccepted {
    pub raid_id: Uuid,
    pub boss_id: Uuid,
    pub created_jobs: Vec<Uuid>,
    pub duplicate: bool,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct CurrentBossView {
    pub raid_id: Uuid,
    pub raid_external_id: String,
    pub raid_name: String,
    pub boss_id: Uuid,
    pub event_id: String,
    pub version: i64,
    pub boss_data: Value,
    pub attackable_parts: Value,
    pub spawned_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct RecommendationView {
    pub id: Uuid,
    pub simulation_job_id: Uuid,
    pub deck_count: i32,
    pub total_average_damage: String,
    pub decks: Value,
    pub created_at: DateTime<Utc>,
}
