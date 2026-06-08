use axum::http::HeaderValue;
use std::sync::Arc;
use tower_http::cors::{self, CorsLayer};
use tower_http::trace::TraceLayer;

mod routes;
mod router;
mod state;
mod models;
mod services;
mod dtos;
use state::AppState;

#[tokio::main]
async fn main() {
    // ← init logging first before anything else
    tracing_subscriber::fmt()
        .with_env_filter("backend=debug,tower_http=debug")
        .init();

    dotenvy::dotenv().ok();

    let port = std::env::var("PORT").unwrap_or("3000".into());
    let state = Arc::new(AppState::new());

    let cors = CorsLayer::new()
        .allow_origin([
            "http://localhost:5173".parse::<HeaderValue>().unwrap(),
            "http://localhost:3000".parse::<HeaderValue>().unwrap(),
        ])
        .allow_methods(cors::Any)
        .allow_headers(cors::Any);

    let app = router::create_router(state)
        .layer(TraceLayer::new_for_http())  // ← log every request
        .layer(cors);

    let addr = format!("localhost:{port}");
    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    tracing::info!("Server running on http://{addr}");
    axum::serve(listener, app).await.unwrap();
}