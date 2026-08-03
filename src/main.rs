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

#[tokio::main]
async fn main() {
    // ← init logging first before anything else
    tracing_subscriber::fmt()
        .with_env_filter("backend=debug,tower_http=debug")
        .init();

    dotenvy::dotenv().ok();

    let config = config::Config::from_env().expect("invalid application configuration");
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
    ));

    let cors = CorsLayer::new()
        .allow_origin([
            "http://localhost:5173".parse::<HeaderValue>().unwrap(),
            "http://localhost:3000".parse::<HeaderValue>().unwrap(),
        ])
        .allow_methods(cors::Any)
        .allow_headers(cors::Any);

    let app = router::create_router(Arc::clone(&state))
        .layer(TraceLayer::new_for_http()) // ← log every request
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
