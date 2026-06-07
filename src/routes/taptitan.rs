use axum::{
    extract::Json,
    http::StatusCode,
    response::{IntoResponse, Json as ResponseJson},
};
use crate::models::player_data::PlayerData;
use crate::models::responses::{ApiResponse,ApiError};


pub async fn send_player_data_json(
    Json(payload): Json<PlayerData>,
) -> impl IntoResponse {
    if payload.player_stats.max_prestige_stage == "0" {
        let error_response: ApiResponse<PlayerData> = ApiResponse::Error {
            error: ApiError {
                code: "INVALID_STAGE".to_string(),
                message: "Max prestige stage cannot be zero".to_string(),
            },
        };
        return (StatusCode::BAD_REQUEST, ResponseJson(error_response)).into_response();
    }

    // Happy path
    let success_response = ApiResponse::Success { data: payload };
    (StatusCode::CREATED, ResponseJson(success_response)).into_response()
}