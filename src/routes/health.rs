use std::sync::Arc;

use axum::{Json, extract::State};
use serde::Serialize;

use crate::state::AppState;

#[derive(Serialize)]
pub struct HealthResponse {
    pub status: String,
    pub version: String,
    pub database: bool,
}

pub async fn handler(State(state): State<Arc<AppState>>) -> Json<HealthResponse> {
    let database = match state.optional_db() {
        Some(pool) => sqlx::query_scalar::<_, i32>("SELECT 1")
            .fetch_one(pool)
            .await
            .is_ok(),
        None => false,
    };
    Json(HealthResponse {
        status: if database { "ok" } else { "degraded" }.into(),
        version: env!("CARGO_PKG_VERSION").into(),
        database,
    })
}
