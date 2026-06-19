use axum::{
    extract::Json,
    http::StatusCode,
    response::{IntoResponse, Json as ResponseJson},
};
use crate::{models::{cards::CardName, player_data::PlayerData, sim_payload::SimPayLoad}, services::taptitan::{player_service::clean_data, sim_service::{self, SimService}}};
use crate::models::responses::{ApiResponse,ApiError};
use crate::dtos::cards::CardDefinitionDto;
use strum::IntoEnumIterator; 
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
    // 2. CardName::iter() automatically knows how to traverse all 42 items!
    let list: Vec<CardDefinitionDto> = CardName::iter()
        .map(|v| CardDefinitionDto {
            id: v.id(), // Will be "moon_beam"
            name: v.display_name(), // Will be "Moon Beam"
            r#type: v.card_type(),
            image: v.image_url(),
        })
        .collect();

    Json(list)
}

pub async fn send_sim_payload(
    Json(simdata): Json<SimPayLoad>
) -> impl IntoResponse {

    // 1. Verify that at least one attackable part is selected
    if simdata.attackable_part.is_empty() {
        let error_response: ApiResponse<SimPayLoad> = ApiResponse::Error {
            error: ApiError {
                code: "ATTACKABLE_PART_EMPTY".to_string(),
                message: "Attackable part cannot be empty. Must select at least 1 part.".to_string(),
            },
        };
        return (StatusCode::BAD_REQUEST, ResponseJson(error_response)).into_response();
    }

    // 2. Verify that the usable cards do not exceed the 42 max allowed game cards
    if simdata.usable_card.len() > 42 {
        let error_response: ApiResponse<SimPayLoad> = ApiResponse::Error {
            error: ApiError {
                code: "USABLE_CARDS_EXCEEDED".to_string(),
                message: format!(
                    "Usable cards list exceeds the maximum game limit of 42 cards (Received: {}).", 
                    simdata.usable_card.len()
                ),
            },
        };
        return (StatusCode::BAD_REQUEST, ResponseJson(error_response)).into_response();
    }

    // 3. Optional Bonus Check: Make sure they aren't passing duplicates of the same card 
    // if your simulation engine requires unique choices per request. Otherwise, skip this block!
    let mut unique_cards = simdata.usable_card.clone();
    unique_cards.sort();
    unique_cards.dedup();
    if unique_cards.len() != simdata.usable_card.len() {
        let error_response: ApiResponse<SimPayLoad> = ApiResponse::Error {
            error: ApiError {
                code: "DUPLICATE_CARDS_FOUND".to_string(),
                message: "Usable cards list contains duplicate card entries.".to_string(),
            },
        };
        return (StatusCode::BAD_REQUEST, ResponseJson(error_response)).into_response();
    }

    //call sim services
    SimService::run_simulation(simdata.clone());

    let success_response = ApiResponse::Success { data: simdata };
    (StatusCode::ACCEPTED, ResponseJson(success_response)).into_response()
}

pub async fn send_sim_deck(
    Json(simdata): Json<SimPayLoad>
)-> impl IntoResponse{
        // 1. Verify that at least one attackable part is selected
    if simdata.attackable_part.is_empty() {
        let error_response: ApiResponse<SimPayLoad> = ApiResponse::Error {
            error: ApiError {
                code: "ATTACKABLE_PART_EMPTY".to_string(),
                message: "Attackable part cannot be empty. Must select at least 1 part.".to_string(),
            },
        };
        return (StatusCode::BAD_REQUEST, ResponseJson(error_response)).into_response();
    }
    if simdata.usable_card.len() > 3 {
        let error_response: ApiResponse<SimPayLoad> = ApiResponse::Error {
            error: ApiError {
                code: "DECK_EXCEED_LIMIT".to_string(),
                message: "Deck cannot contain more than 3 card".to_string(),
            },
        };
        return (StatusCode::BAD_REQUEST, ResponseJson(error_response)).into_response();
    }
    println!("run deck sim");
    SimService::run_deck_simulation(simdata.clone());
    (StatusCode::ACCEPTED, ResponseJson(simdata)).into_response()
}