use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "token_status", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum TokenStatus {
    Missing,
    Configured,
    Invalid,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "job_status", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum JobStatus {
    Pending,
    Running,
    Optimizing,
    Completed,
    Failed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "recompute_mode", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum RecomputeMode {
    Full,
    PhaseAware,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "recommendation_phase", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum RecommendationPhase {
    Current,
    Void,
}

