use std::sync::Arc;

use axum::{
    extract::{Request, State},
    middleware::Next,
    response::Response,
};

use crate::{error::AppError, state::AppState};

pub async fn require_key(
    State(state): State<Arc<AppState>>,
    request: Request,
    next: Next,
) -> Result<Response, AppError> {
    let supplied = request
        .headers()
        .get("x-internal-api-key")
        .and_then(|value| value.to_str().ok());
    if supplied != Some(state.internal_api_key.as_ref()) {
        return Err(AppError::Unauthorized(
            "A valid x-internal-api-key header is required".to_string(),
        ));
    }
    Ok(next.run(request).await)
}
