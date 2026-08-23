use std::{collections::HashMap, str::FromStr, sync::OnceLock};

use super::{
    card_skill_data::card_skill_row,
    cards::{Card, CardName},
};

static SEASONAL_CARD_BOOSTS: OnceLock<HashMap<CardName, u16>> = OnceLock::new();

pub fn seasonal_level_boost(card: CardName) -> u16 {
    *SEASONAL_CARD_BOOSTS
        .get_or_init(load_seasonal_card_boosts)
        .get(&card)
        .unwrap_or(&0)
}

pub fn seasonal_effective_level(card: CardName, saved_level: u16) -> u16 {
    let max_level = card_skill_row(card)
        .map(|skill| skill.max_level)
        .unwrap_or(u16::MAX);
    saved_level
        .saturating_add(seasonal_level_boost(card))
        .min(max_level)
}

pub fn apply_seasonal_level_boost(card: &mut Card) {
    card.level = seasonal_effective_level(card.card_id, card.level);
}

fn load_seasonal_card_boosts() -> HashMap<CardName, u16> {
    let configured: HashMap<String, u16> = serde_json::from_str(include_str!(
        "../../assets/taptitan/config/seasonal_card_boosts.json"
    ))
    .expect("seasonal_card_boosts.json must contain a card-ID to level-boost object");

    configured
        .into_iter()
        .map(|(card_id, boost)| {
            let card = CardName::from_str(&card_id)
                .unwrap_or_else(|_| panic!("unknown seasonal boosted card ID: {card_id}"));
            (card, boost)
        })
        .collect()
}

#[cfg(test)]
#[path = "../../tests/unit/models/seasonal_card_boosts_tests.rs"]
mod tests;
