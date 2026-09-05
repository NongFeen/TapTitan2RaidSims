use crate::models::cards::CardType;
use serde::Serialize; // Reference your core enum type
use utoipa::ToSchema;

#[derive(Serialize, ToSchema)]
pub struct CardDefinitionDto {
    pub id: &'static str,   // Will output "FlakShot"
    pub name: &'static str, // Will output "Flak Shot"
    pub r#type: CardType,
    pub seasonal_level_boost: u16,
}
