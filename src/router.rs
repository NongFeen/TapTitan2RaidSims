use axum::{Router, routing::get};
use std::sync::Arc;
use tower_http::services::ServeDir;
use crate::state::AppState;
use crate::routes;

pub fn create_router(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/api/health", get(routes::health::handler))
        .route("/api/taptitan/boss", get(routes::taptitan::get_boss))
        .nest_service("/assets", ServeDir::new("assets"))
        .with_state(state)
}