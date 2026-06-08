use axum::{
    extract::Json,
    http::StatusCode,
    response::{IntoResponse, Json as ResponseJson},
};
use crate::{models::{cards::CardName, player_data::PlayerData}, services::taptitan::player_service::clean_data};
use crate::models::responses::{ApiResponse,ApiError};
use crate::dtos::cards::CardDefinitionDto;
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
    
    // resolve for clean data
    let cleaned = clean_data(&payload);
    // return clean data
    let success_response = ApiResponse::Success { data: cleaned };
    (StatusCode::CREATED, ResponseJson(success_response)).into_response()
}

pub async fn get_all_card_definitions() -> impl IntoResponse {
    let definitions: Vec<CardDefinitionDto> = CardName::all_variants()
        .iter()
        .map(|variant| CardDefinitionDto {
            id: variant.id(),
            name: variant.display_name(),
            r#type: variant.card_type(), // Calls your existing .card_type() matching logic
            image: variant.image_url(),
        })
        .collect();

    Json(definitions)
}