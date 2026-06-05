use axum::{Router, routing::get};
use std::sync::Arc;
use crate::state::AppState;
use crate::routes;

pub fn create_router(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/api/health", get(routes::health::handler))
        .with_state(state)
}