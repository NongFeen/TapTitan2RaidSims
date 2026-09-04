use axum::http::HeaderValue;
use std::sync::Arc;
use tower_http::cors::{self, CorsLayer};
use tower_http::trace::TraceLayer;

mod config;
mod database;
mod docs;
mod dtos;
mod error;
mod models;
mod request_logging;
mod router;
mod routes;
mod services;
mod state;
use request_logging::FilteredHttpTrace;
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
    tracing::info!(role = ?config.role, "service role configured");
    services::taptitan::sim_service::configure_sim_worker_count(config.simulation_worker_count);
    tracing::info!(
        simulation_worker_count = config.simulation_worker_count,
        "simulation worker allocation configured (0 uses all available CPUs)"
    );

    // The worker role runs simulation jobs pulled from the shared queue; job
    // payloads already carry everything a simulation needs by the time
    // they're enqueued, so it has no use for the TT2 socket or an HTTP server.
    let gamehive_api = if config.role.serves_http() {
        config.tt2.clone().map(|tt2_config| {
            services::gamehive_api_client::GameHiveApiClient::new(tt2_config)
                .expect("invalid TT2 player-token encryption configuration")
        })
    } else {
        None
    };
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
    if config.role.serves_http() && !config.internal_api_enabled {
        tracing::warn!("INTERNAL_API_ENABLED=false; all /internal/* routes will return 503");
    }
    let state = Arc::new(AppState::new(
        pool,
        config.simulation_concurrency,
        config.internal_api_key,
        config.internal_api_enabled,
        gamehive_api.clone(),
    ));

    if let Some(gamehive_api) = gamehive_api {
        let socket_state = Arc::clone(&state);
        tokio::spawn(async move {
            gamehive_api.connect(socket_state).await;
        });
        services::clan_sync_service::spawn_scheduled_clan_fetch(Arc::clone(&state));
    } else if config.role.serves_http() {
        tracing::warn!("TT2 integration is not configured; player fetching is unavailable");
    }

    if config.role.runs_jobs() {
        state.reset_stuck_jobs().await;
        tokio::spawn(services::job_service::run_dispatch_loop(Arc::clone(&state)));
    }

    if !config.role.serves_http() {
        tracing::info!("worker role: no HTTP server; running simulation jobs only");
        shutdown_signal().await;
        return;
    }

    let allowed_origins: Vec<HeaderValue> = config
        .cors_allowed_origins
        .iter()
        .map(|origin| {
            origin
                .parse::<HeaderValue>()
                .unwrap_or_else(|_| panic!("invalid CORS_ALLOWED_ORIGINS entry: {origin}"))
        })
        .collect();
    tracing::info!(origins = ?config.cors_allowed_origins, "CORS allowed origins configured");
    let cors = CorsLayer::new()
        .allow_origin(allowed_origins)
        .allow_methods(cors::Any)
        .allow_headers(cors::Any);

    let request_trace = TraceLayer::new_for_http()
        .make_span_with(FilteredHttpTrace)
        .on_request(FilteredHttpTrace)
        .on_response(FilteredHttpTrace)
        .on_failure(FilteredHttpTrace);
    let mut app = router::create_router(Arc::clone(&state));
    if config.swagger_ui_enabled {
        tracing::info!("Swagger UI mounted at /docs");
        app = app.merge(docs::swagger_router());
    }
    let app = app.layer(request_trace).layer(cors);

    let addr = format!("0.0.0.0:{}", config.port);
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
