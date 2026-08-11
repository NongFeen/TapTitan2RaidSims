use crate::models::cards::CardType;
use serde::Serialize; // Reference your core enum type

#[derive(Serialize)]
pub struct CardDefinitionDto {
    pub id: &'static str,   // Will output "FlakShot"
    pub name: &'static str, // Will output "Flak Shot"
    pub r#type: CardType,
    pub image: String,
    pub seasonal_level_boost: u16,
}
