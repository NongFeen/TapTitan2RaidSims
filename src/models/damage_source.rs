use serde::{Deserialize, Serialize};

use crate::models::cards::CardName;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "value")]
pub enum DamageSource {
    Tap,
    Card(CardName),
}

impl DamageSource {}
