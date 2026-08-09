use crate::routes;
use crate::state::AppState;
use axum::{Router, middleware, routing::get, routing::post, routing::put};
use std::sync::Arc;
use tower_http::services::ServeDir;

pub fn create_router(state: Arc<AppState>) -> Router {
    let assets = Router::new()
        .fallback_service(ServeDir::new("assets"))
        .layer(middleware::from_fn(
            routes::asset_security::prevent_hotlinking,
        ));

    let internal = Router::new()
        .route("/simulation-jobs", post(routes::jobs::create))
        .route("/simulation-jobs/{job_id}", get(routes::jobs::get))
        .route("/simulation-jobs/{job_id}/retry", post(routes::jobs::retry))
        .route("/current-boss", put(routes::raids::update_current_boss))
        .route(
            "/players/{player_id}/token",
            put(routes::players::update_token).delete(routes::players::clear_token),
        )
        .route(
            "/players/{player_id}/fetch-stats",
            post(routes::players::fetch_tt2_stats),
        )
        .route("/tt2/player-status", get(routes::players::tt2_status))
        .route_layer(middleware::from_fn_with_state(
            Arc::clone(&state),
            routes::internal_auth::require_key,
        ));

    Router::new()
        .nest("/internal", internal)
        //GET
        .route("/api/health", get(routes::health::handler))
        .route(
            "/api/players",
            post(routes::players::create).get(routes::players::list),
        )
        .route("/api/players/{player_id}", get(routes::players::get))
        .route(
            "/api/players/{player_id}/simulation-jobs",
            get(routes::jobs::list_for_player),
        )
        .route(
            "/api/players/{player_id}/stats",
            put(routes::players::update_stats),
        )
        .route(
            "/api/players/{player_id}/stats/current",
            get(routes::players::current_stats),
        )
        .route(
            "/api/players/{player_id}/auto_sims",
            put(routes::players::update_auto_sims),
        )
        .route("/api/current-boss", get(routes::raids::current))
        .route(
            "/api/players/{player_id}/recommendations/current",
            get(routes::recommendations::current_for_player),
        )
        .route(
            "/api/players/{player_id}/recommendations",
            post(routes::recommendations::generate_for_player),
        )
        .route(
            "/api/simulation-jobs/{job_id}/deck-results",
            get(routes::recommendations::deck_results),
        )
        // .route("/api/taptitan/boss", get(routes::taptitan::get_boss))
        //POST
        .route(
            "/api/taptitan/player_data",
            post(routes::taptitan::send_player_data_json),
        )
        .route(
            "/api/taptitan/cards",
            get(routes::taptitan::get_all_card_definitions),
        )
        .route(
            "/api/taptitan/sim_data",
            post(routes::taptitan::send_sim_payload),
        )
        .route(
            "/api/taptitan/sim_deck",
            post(routes::taptitan::send_sim_deck),
        )
        .nest("/assets", assets)
        .with_state(state)
}
