use axum::{Json, extract::State};
use std::sync::Arc;
use crate::state::AppState;
use crate::models::ttboss::Boss;

pub async fn get_boss(State(state): State<Arc<AppState>>) -> Json<Boss> {
    Json(state.boss.clone())
}