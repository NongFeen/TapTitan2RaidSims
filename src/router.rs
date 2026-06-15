use axum::{Router, routing::get,routing::post};
use std::sync::Arc;
use tower_http::services::ServeDir;
use crate::state::AppState;
use crate::routes;

pub fn create_router(state: Arc<AppState>) -> Router {
    Router::new()
        //GET
        .route("/api/health", get(routes::health::handler))
        // .route("/api/taptitan/boss", get(routes::taptitan::get_boss))
        //POST
        .route("/api/taptitan/player_data", post(routes::taptitan::send_player_data_json))
        .route("/api/taptitan/cards", get(routes::taptitan::get_all_card_definitions))
        .route("/api/taptitan/sim_data", post(routes::taptitan::send_sim_payload))
        .nest_service("/assets", ServeDir::new("assets"))
        .with_state(state)
}