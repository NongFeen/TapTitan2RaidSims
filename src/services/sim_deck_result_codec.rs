//! What actually gets persisted for `simulation_deck_results`.
//!
//! The in-memory `SimDeckResult`/`SimPatternResult` carry a lot that's either
//! duplicated elsewhere or never populated by the real simulation pipeline:
//! `deck`/`deck_names` are fully reconstructible from the `card_mask` column,
//! `_display` strings are derived formatting of a number the frontend
//! already formats itself, `card_name` is derivable from `card`,
//! `average_damage` at the pattern level duplicates the row's own
//! `average_damage` column, `dependency_part_mask`/`total_attack_patterns`
//! are their own real columns, and `patterns` (plural) is always empty in
//! production -- only the live, non-persisted debug endpoint ever populates
//! it. A deck's winning pattern is always exactly 3 cards (`prepare_results`
//! already only keeps decks of len 3), so instead of a JSONB blob the
//! remaining pattern data -- attack pattern name, each card, each card's
//! damage, and the pattern's lowest/highest round damage -- gets its own
//! real column (`pattern`, `card1..3`, `card1_damage..3`,
//! `deck_lowest_damage`, `deck_highest_damage`). Only the live debug
//! endpoint needs the full shape; nothing that reads a persisted row does.

use crate::models::{cards::CardName, db_enums::RecommendationPhase};
use crate::services::taptitan::{
    recommendation::cards_from_mask,
    sim_service::{
        SimCardDamageResult, SimDeckResult, SimPatternResult, SimulationPhase, format_compact,
    },
};

/// A deck's winning pattern, narrowed to the columns actually worth storing.
pub struct NarrowedPattern {
    pub pattern: String,
    pub card1: CardName,
    pub card2: CardName,
    pub card3: CardName,
    pub card1_damage: i64,
    pub card2_damage: i64,
    pub card3_damage: i64,
    pub deck_lowest_damage: i64,
    pub deck_highest_damage: i64,
}

/// Narrows a deck's winning pattern down to what's actually worth storing.
/// Panics if `pattern.card_damage` isn't exactly 3 entries, which never
/// happens -- every caller only ever narrows a 3-card deck's pattern.
pub fn narrow_for_persist(pattern: &SimPatternResult) -> NarrowedPattern {
    let [card1, card2, card3] = <&[SimCardDamageResult; 3]>::try_from(pattern.card_damage.as_slice())
        .expect("a deck's winning pattern always has exactly 3 cards");
    NarrowedPattern {
        pattern: pattern.pattern.clone(),
        card1: card1.card,
        card2: card2.card,
        card3: card3.card,
        card1_damage: card1.average_damage as i64,
        card2_damage: card2.average_damage as i64,
        card3_damage: card3.average_damage as i64,
        deck_lowest_damage: pattern.lowest_round_damage as i64,
        deck_highest_damage: pattern.highest_round_damage as i64,
    }
}

/// Rebuilds a full `SimDeckResult` from a persisted row's flattened pattern
/// columns, for the incremental recompute path that reuses a deck's result
/// as-is. `dependency_part_mask` is deliberately left at 0 -- every caller of
/// this immediately overwrites it from the row's own column anyway.
#[allow(clippy::too_many_arguments)]
pub fn rebuild_from_persisted(
    pattern: String,
    card1: CardName,
    card2: CardName,
    card3: CardName,
    card1_damage: i64,
    card2_damage: i64,
    card3_damage: i64,
    deck_lowest_damage: i64,
    deck_highest_damage: i64,
    card_mask: u64,
    average_damage: u64,
    total_attack_patterns: i32,
    recommendation_phase: RecommendationPhase,
) -> SimDeckResult {
    let deck = cards_from_mask(card_mask);
    let deck_names = deck
        .iter()
        .map(|card| card.display_name().to_string())
        .collect();
    let card_damage = [
        (card1, card1_damage),
        (card2, card2_damage),
        (card3, card3_damage),
    ]
    .into_iter()
    .map(|(card, damage)| {
        let average_damage = damage as u64;
        SimCardDamageResult {
            card,
            card_name: card.display_name().to_string(),
            average_damage,
            average_damage_display: format_compact(average_damage),
        }
    })
    .collect();
    // tap_damage isn't its own persisted column -- approximate it from the
    // ones that are. This carries the same small joint-vs-independent
    // rounding drift `SimPatternResult::tap_damage`'s doc comment warns
    // about; only a freshly-run simulation's own tap_damage is exact.
    let tap_damage = average_damage.saturating_sub(
        card1_damage as u64 + card2_damage as u64 + card3_damage as u64,
    );
    SimDeckResult {
        deck,
        deck_names,
        total_attack_patterns: total_attack_patterns as usize,
        best_pattern: Some(SimPatternResult {
            pattern,
            average_damage,
            average_damage_display: format_compact(average_damage),
            lowest_round_damage: deck_lowest_damage as u64,
            lowest_round_damage_display: format_compact(deck_lowest_damage as u64),
            highest_round_damage: deck_highest_damage as u64,
            highest_round_damage_display: format_compact(deck_highest_damage as u64),
            tap_damage,
            tap_damage_display: format_compact(tap_damage),
            card_damage,
        }),
        simulation_phase: match recommendation_phase {
            RecommendationPhase::Current => SimulationPhase::Current,
            RecommendationPhase::Void => SimulationPhase::TargetedBody,
        },
        patterns: Vec::new(),
        dependency_part_mask: 0,
    }
}

#[cfg(test)]
#[path = "../../tests/unit/services/sim_deck_result_codec_tests.rs"]
mod tests;
