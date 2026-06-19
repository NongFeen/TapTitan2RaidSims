use serde::{Deserialize, Serialize};

use crate::models::cards::CardName;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "value")]
pub enum DamageSource {
    Tap,
    Card(CardName),
}

impl DamageSource {
    pub fn label(&self) -> &'static str {
        match self {
            DamageSource::Tap => "Tap",
            DamageSource::Card(card_name) => match card_name {
                CardName::MoonBeam => "MoonBeam",
                CardName::GuardBreak => "Weaken",
                CardName::RuinousRain => "Rain",
                _ => card_name.id(),
            },
        }
    }
}
