use std::{collections::HashMap, str::FromStr, sync::OnceLock};

use super::{
    card_skill_data::card_skill_row,
    cards::{Card, CardName},
};

/// Env var listing this season's boosted cards as `CardId:levels`, comma
/// separated (e.g. `BurstBoost:10,Fragmentize:20`). Unset means no boosts.
pub const SEASONAL_CARD_BOOSTS_ENV: &str = "SEASONAL_CARD_BOOSTS";

static SEASONAL_CARD_BOOSTS: OnceLock<HashMap<CardName, u16>> = OnceLock::new();

/// Reads the seasonal boosts from the environment once, at startup, so a typo
/// fails the deploy instead of the first simulation.
pub fn load_from_env() -> Result<&'static HashMap<CardName, u16>, String> {
    if let Some(boosts) = SEASONAL_CARD_BOOSTS.get() {
        return Ok(boosts);
    }
    let configured = std::env::var(SEASONAL_CARD_BOOSTS_ENV).unwrap_or_default();
    let boosts = parse_seasonal_card_boosts(&configured)?;
    Ok(SEASONAL_CARD_BOOSTS.get_or_init(|| boosts))
}

pub fn parse_seasonal_card_boosts(value: &str) -> Result<HashMap<CardName, u16>, String> {
    value
        .split(',')
        .map(str::trim)
        .filter(|entry| !entry.is_empty())
        .map(|entry| {
            let (card_id, levels) = entry
                .split_once(':')
                .ok_or_else(|| format!("{SEASONAL_CARD_BOOSTS_ENV} entry `{entry}` must be CardId:levels"))?;
            let card = CardName::from_str(card_id.trim())
                .map_err(|_| format!("{SEASONAL_CARD_BOOSTS_ENV} has unknown card ID `{}`", card_id.trim()))?;
            let levels = levels
                .trim()
                .parse()
                .map_err(|_| format!("{SEASONAL_CARD_BOOSTS_ENV} boost for `{card_id}` must be a whole number"))?;
            Ok((card, levels))
        })
        .collect()
}

pub fn seasonal_level_boost(card: CardName) -> u16 {
    let boosts = load_from_env().unwrap_or_else(|error| panic!("{error}"));
    boosts.get(&card).copied().unwrap_or(0)
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

#[cfg(test)]
#[path = "../../tests/unit/models/seasonal_card_boosts_tests.rs"]
mod tests;
