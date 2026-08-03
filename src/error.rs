use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde::Serialize;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("{0}")]
    BadRequest(String),
    #[error("{0}")]
    NotFound(String),
    #[error("{0}")]
    Conflict(String),
    #[error("{0}")]
    Unauthorized(String),
    #[error("database is unavailable")]
    DatabaseUnavailable,
    #[error("database operation failed")]
    Database(#[from] sqlx::Error),
    #[error("serialization failed")]
    Serialization(#[from] serde_json::Error),
    #[error("internal operation failed: {0}")]
    Internal(String),
}

#[derive(Serialize)]
struct ErrorBody {
    code: &'static str,
    message: String,
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, code, message) = match self {
            Self::BadRequest(message) => (StatusCode::BAD_REQUEST, "BAD_REQUEST", message),
            Self::NotFound(message) => (StatusCode::NOT_FOUND, "NOT_FOUND", message),
            Self::Conflict(message) => (StatusCode::CONFLICT, "CONFLICT", message),
            Self::Unauthorized(message) => (StatusCode::UNAUTHORIZED, "UNAUTHORIZED", message),
            Self::DatabaseUnavailable => (
                StatusCode::SERVICE_UNAVAILABLE,
                "DATABASE_UNAVAILABLE",
                "This endpoint requires PostgreSQL; start the database and restart the backend"
                    .to_string(),
            ),
            Self::Database(error) => {
                tracing::error!(?error, "database operation failed");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "DATABASE_ERROR",
                    "Database operation failed".to_string(),
                )
            }
            Self::Serialization(error) => {
                tracing::error!(?error, "serialization failed");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "SERIALIZATION_ERROR",
                    "Serialization failed".to_string(),
                )
            }
            Self::Internal(message) => {
                tracing::error!(%message, "internal operation failed");
                (StatusCode::INTERNAL_SERVER_ERROR, "INTERNAL_ERROR", message)
            }
        };
        (status, Json(ErrorBody { code, message })).into_response()
    }
}
