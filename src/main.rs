use axum::http::HeaderValue;
use std::sync::Arc;
use tower_http::cors::{self, CorsLayer};
use tower_http::trace::TraceLayer;

mod config;
mod database;
mod dtos;
mod error;
mod models;
mod router;
mod routes;
mod services;
mod state;
use state::AppState;

fn main() {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .max_blocking_threads(1)
        .thread_name("calculation-worker")
        .build()
        .expect("failed to build the API runtime");

    runtime.block_on(run());
}

async fn run() {
    // Initialize logging before connecting to external services.
    tracing_subscriber::fmt()
        .with_env_filter("backend=debug,tower_http=debug")
        .init();
    tracing::info!(
        api_worker_threads = 1,
        blocking_worker_threads = 1,
        "runtime thread allocation configured"
    );

    dotenvy::dotenv().ok();

    let config = config::Config::from_env().expect("invalid application configuration");
    services::taptitan::sim_service::configure_sim_worker_count(config.simulation_worker_count);
    tracing::info!(
        simulation_worker_count = config.simulation_worker_count,
        "simulation worker allocation configured (0 uses all available CPUs)"
    );
    let gamehive_api = config.tt2.clone().map(|tt2_config| {
        services::gamehive_api_client::GameHiveApiClient::new(tt2_config)
            .expect("invalid TT2 player-token encryption configuration")
    });
    let pool = match config.database_url.as_deref() {
        Some(database_url) => match database::connect(database_url).await {
            Ok(pool) => {
                tracing::info!("database connected and migrations applied");
                Some(pool)
            }
            Err(error) => {
                tracing::warn!(?error, "database unavailable; starting in degraded mode");
                None
            }
        },
        None => {
            tracing::warn!("DATABASE_URL is not configured; starting in degraded mode");
            None
        }
    };
    let state = Arc::new(AppState::new(
        pool,
        config.simulation_concurrency,
        config.internal_api_key,
        gamehive_api.clone(),
    ));

    if let Some(gamehive_api) = gamehive_api {
        let socket_state = Arc::clone(&state);
        tokio::spawn(async move {
            gamehive_api.connect(socket_state).await;
        });
    } else {
        tracing::warn!("TT2 integration is not configured; player fetching is unavailable");
    }

    let cors = CorsLayer::new()
        .allow_origin([
            "http://localhost:5173".parse::<HeaderValue>().unwrap(),
            "http://localhost:3000".parse::<HeaderValue>().unwrap(),
        ])
        .allow_methods(cors::Any)
        .allow_headers(cors::Any);

    let app = router::create_router(Arc::clone(&state))
        .layer(TraceLayer::new_for_http())
        .layer(cors);

    state.recover_pending_jobs().await;

    let addr = format!("localhost:{}", config.port);
    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    tracing::info!("Server running on http://{addr}");
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await
        .unwrap();
}

async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };
    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("failed to install termination handler")
            .recv()
            .await;
    };
    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();
    tokio::select! { _ = ctrl_c => {}, _ = terminate => {} }
    tracing::info!("shutdown signal received");
}
