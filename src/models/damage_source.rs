use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::models::cards::CardName;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(tag = "kind", content = "value")]
pub enum DamageSource {
    Tap,
    Card(CardName),
}

impl DamageSource {}
