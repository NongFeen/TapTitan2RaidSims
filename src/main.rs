use axum::{Router, http::HeaderValue, routing::get};
use std::sync::Arc;
use tower_http::cors::{self, CorsLayer};

mod routes;
mod state;

use state::AppState;

#[tokio::main]
async fn main() {
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

    let app = Router::new()
        .route("/api/health", get(routes::health::handler))
        .layer(cors)
        .with_state(state);

    let addr = format!("localhost:{port}");
    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    println!("Server running on http://{addr}");
    axum::serve(listener, app).await.unwrap();
}