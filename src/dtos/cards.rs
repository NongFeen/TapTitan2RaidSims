use serde::Serialize;
use crate::models::cards::CardType; // Reference your core enum type

#[derive(Serialize)]
pub struct CardDefinitionDto {
    pub id: &'static str,
    pub name: &'static str,
    pub r#type: CardType, 
    pub image: String,
}