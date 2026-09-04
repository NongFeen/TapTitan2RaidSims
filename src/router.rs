use crate::routes;
use crate::state::AppState;
use axum::{Router, middleware, routing::get, routing::post, routing::put};
use std::sync::Arc;
use tower_http::services::{ServeDir, ServeFile};

pub fn create_router(state: Arc<AppState>) -> Router {
    // Serves the frontend's built assets when bundled alongside the backend
    // binary (see Dockerfile.combined). Falls back to index.html for any
    // path that isn't a static file, so client-side routing (react-router)
    // works on a hard refresh / direct link.
    let static_dir = std::env::var("STATIC_DIR").unwrap_or_else(|_| "static".to_string());
    let spa_fallback = ServeFile::new(format!("{static_dir}/index.html"));
    // `.fallback` (not `.not_found_service`) so unmatched client routes come
    // back as 200, matching how the standalone nginx setup served the SPA.
    let serve_static = ServeDir::new(&static_dir).fallback(spa_fallback);

    let internal = Router::new()
        .route("/simulation-jobs", post(routes::jobs::create))
        .route("/simulation-jobs/batch", post(routes::jobs::create_batch))
        .route(
            "/simulation-jobs/batch/{batch_id}",
            get(routes::jobs::get_batch),
        )
        .route("/simulation-jobs/{job_id}", get(routes::jobs::get))
        .route("/simulation-jobs/{job_id}/retry", post(routes::jobs::retry))
        .route("/simulation-debug", post(routes::simulation_debug::run))
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
        .route("/tt2/clan-status", get(routes::players::tt2_clan_status))
        .route(
            "/tt2/fetch-clan-stats",
            post(routes::players::fetch_tt2_clan_stats),
        )
        .route_layer(middleware::from_fn_with_state(
            Arc::clone(&state),
            routes::internal_auth::require_key,
        ));

    Router::new()
        .nest("/internal", internal)
        //GET
        .route("/api/health", get(routes::health::handler))
        .route("/api/players", get(routes::players::list))
        .route("/api/players/{player_id}", get(routes::players::get))
        .route(
            "/api/players/{player_id}/simulation-jobs",
            get(routes::jobs::list_for_player),
        )

        // UNUSED FETCH FROM GAME NO MANUAL UPDATE
        // .route(
        //     "/api/players/{player_id}/stats",
        //     put(routes::players::update_stats),
        // ) 
        
        .route(
            "/api/players/{player_id}/stats/current",
            get(routes::players::current_stats),
        )
        .route(
            "/api/players/{player_id}/attack-log",
            get(routes::players::attack_log),
        )

        // .route(
        //     "/api/players/{player_id}/auto_sims",
        //     put(routes::players::update_auto_sims),
        // )

        
        .route("/api/current-boss", get(routes::raids::current))
        .route(
            "/api/live-current-boss",
            get(routes::raids::live_from_attack),
        )
        .route(
            "/api/live-current-boss/stream",
            get(routes::raids::live_current_boss_stream),
        )
        .route(
            "/api/live-attacking-players",
            get(routes::raids::live_attacking_players),
        )
        .route(
            "/api/live-attacking-players/stream",
            get(routes::raids::live_attacking_players_stream),
        )
        .route("/api/raid-cycle/current", get(routes::raid_cycle::current))
        .route(
            "/api/players/{player_id}/recommendations/current",
            get(routes::recommendations::current_for_player),
        )
        .route(
            "/api/players/{player_id}/recommendations",
            post(routes::recommendations::generate_for_player),
        )
        .route(
            "/api/players/{player_id}/recommendations/custom",
            post(routes::recommendations::custom_for_player),
        )

        // .route("/api/taptitan/boss", get(routes::taptitan::get_boss))
        //POST UNUSED
        // .route(
        //     "/api/taptitan/player_data",
        //     post(routes::taptitan::send_player_data_json),
        // )
        .route(
            "/api/taptitan/cards",
            get(routes::taptitan::get_all_card_definitions),
        )
        // .route(
        //     "/api/taptitan/sim_data",
        //     post(routes::taptitan::send_sim_payload),
        // )
        // .route(
        //     "/api/taptitan/sim_deck",
        //     post(routes::taptitan::send_sim_deck),
        // )
        .fallback_service(serve_static)
        .with_state(state)
}
