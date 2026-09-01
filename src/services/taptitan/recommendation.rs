use std::collections::HashMap;
use std::time::Instant;

use serde::{Deserialize, Serialize};
use strum::IntoEnumIterator;

use crate::models::cards::CardName;
use crate::services::taptitan::sim_service::SimDeckResult;

const GREEDY_SEED_LIMIT: usize = 2_048;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CandidateDeck {
    pub source_index: usize,
    pub cards: Vec<CardName>,
    pub card_mask: u64,
    pub average_damage: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DeckRecommendation {
    pub deck_count: usize,
    pub total_average_damage: u64,
    pub decks: Vec<CandidateDeck>,
}

pub fn candidates_from_results(results: &[SimDeckResult]) -> Vec<CandidateDeck> {
    let indices: HashMap<CardName, usize> = CardName::iter()
        .enumerate()
        .map(|(index, card)| (card, index))
        .collect();

    results
        .iter()
        .enumerate()
        .filter_map(|(source_index, result)| {
            let average_damage = result.best_pattern.as_ref()?.average_damage;
            if result.deck.len() != 3 {
                return None;
            }
            let card_mask = result.deck.iter().try_fold(0u64, |mask, card| {
                let index = *indices.get(card)?;
                Some(mask | (1u64 << index))
            })?;
            Some(CandidateDeck {
                source_index,
                cards: result.deck.clone(),
                card_mask,
                average_damage,
            })
        })
        .collect()
}

/// The inverse of `candidates_from_results`'s mask computation -- recovers a
/// deck's card list from its `card_mask` alone. Safe as long as `CardName`
/// stays append-only (never reordered), which `card_name_declaration_order_is_pinned`
/// guards against.
pub fn cards_from_mask(mask: u64) -> Vec<CardName> {
    CardName::iter()
        .enumerate()
        .filter_map(|(index, card)| (mask & (1u64 << index) != 0).then_some(card))
        .collect()
}

pub fn optimize_decks(
    candidates: &[CandidateDeck],
    deck_count: usize,
) -> Option<DeckRecommendation> {
    optimize_decks_with_required_cards(candidates, deck_count, &[])
}

pub fn optimize_decks_with_required_cards(
    candidates: &[CandidateDeck],
    deck_count: usize,
    required_cards: &[CardName],
) -> Option<DeckRecommendation> {
    let required_card_mask = required_cards.iter().try_fold(0u64, |mask, card| {
        CardName::iter()
            .position(|candidate| candidate == *card)
            .map(|index| mask | (1u64 << index))
    })?;

    if deck_count == 0 {
        return (required_card_mask == 0).then_some(DeckRecommendation {
            deck_count,
            total_average_damage: 0,
            decks: Vec::new(),
        });
    }

    let mut sorted = candidates.to_vec();
    sorted.sort_by_key(|candidate| std::cmp::Reverse(candidate.average_damage));
    let search_started = Instant::now();
    tracing::info!(
        deck_count,
        ?required_cards,
        progress_percent = 0,
        phase = "greedy_seed",
        "finding top deck recommendation"
    );

    let mut best_total = 0u64;
    let mut best_indices = Vec::new();
    seed_with_greedy(
        &sorted,
        deck_count,
        required_card_mask,
        &mut best_total,
        &mut best_indices,
    );
    tracing::info!(
        deck_count,
        ?required_cards,
        phase = "greedy_seed_complete",
        elapsed_ms = search_started.elapsed().as_millis(),
        "top deck recommendation phase complete"
    );
    let mut selected = Vec::with_capacity(deck_count);
    let root_candidate_count = sorted.len().saturating_sub(deck_count.saturating_sub(1));
    for index in 0..root_candidate_count {
        let candidate = &sorted[index];
        selected.push(index);
        search(
            &sorted,
            deck_count,
            index + 1,
            candidate.card_mask,
            candidate.average_damage,
            required_card_mask,
            &mut selected,
            &mut best_total,
            &mut best_indices,
        );
        selected.pop();

        // let progress_percent = (index + 1) * 100 / root_candidate_count.max(1);
        // while progress_percent >= next_progress_percent && next_progress_percent <= 100 {
        //     tracing::info!(
        //         deck_count,
        //         ?required_cards,
        //         progress_percent = next_progress_percent,
        //         phase = "exact_search",
        //         progress_basis = "root_candidates",
        //         elapsed_ms = search_started.elapsed().as_millis(),
        //         "finding top deck recommendation"
        //     );
        //     next_progress_percent += 10;
        // }
    }
    tracing::info!(
        deck_count,
        ?required_cards,
        phase = "exact_search_complete",
        elapsed_ms = search_started.elapsed().as_millis(),
        "top deck recommendation phase complete"
    );

    if best_indices.len() != deck_count {
        return None;
    }

    Some(DeckRecommendation {
        deck_count,
        total_average_damage: best_total,
        decks: best_indices
            .into_iter()
            .map(|index| sorted[index].clone())
            .collect(),
    })
}

fn seed_with_greedy(
    candidates: &[CandidateDeck],
    target_count: usize,
    required_card_mask: u64,
    best_total: &mut u64,
    best_indices: &mut Vec<usize>,
) {
    // Multiple starting points produce a useful lower bound before exact search.
    // A strong incumbent makes branch-and-bound practical for a full card pool.
    for forced in 0..candidates.len().min(GREEDY_SEED_LIMIT) {
        let mut used_cards = candidates[forced].card_mask;
        let mut total = candidates[forced].average_damage;
        let mut selected = vec![forced];
        for (index, candidate) in candidates.iter().enumerate() {
            if selected.len() == target_count {
                break;
            }
            if index != forced && candidate.card_mask & used_cards == 0 {
                used_cards |= candidate.card_mask;
                total = total.saturating_add(candidate.average_damage);
                selected.push(index);
            }
        }
        if selected.len() == target_count
            && used_cards & required_card_mask == required_card_mask
            && (best_indices.is_empty() || total > *best_total)
        {
            *best_total = total;
            best_indices.clone_from(&selected);
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn search(
    candidates: &[CandidateDeck],
    target_count: usize,
    start: usize,
    used_cards: u64,
    total: u64,
    required_card_mask: u64,
    selected: &mut Vec<usize>,
    best_total: &mut u64,
    best_indices: &mut Vec<usize>,
) {
    if selected.len() == target_count {
        if used_cards & required_card_mask == required_card_mask
            && (total > *best_total || best_indices.is_empty())
        {
            *best_total = total;
            best_indices.clone_from(selected);
        }
        return;
    }

    let remaining = target_count - selected.len();
    if candidates.len().saturating_sub(start) < remaining {
        return;
    }

    let optimistic = candidates[start..]
        .iter()
        .take(remaining)
        .fold(total, |sum, candidate| {
            sum.saturating_add(candidate.average_damage)
        });
    if !best_indices.is_empty() && optimistic <= *best_total {
        return;
    }

    for index in start..candidates.len() {
        if candidates.len() - index < remaining {
            break;
        }
        let candidate = &candidates[index];
        if candidate.card_mask & used_cards != 0 {
            continue;
        }
        selected.push(index);
        search(
            candidates,
            target_count,
            index + 1,
            used_cards | candidate.card_mask,
            total.saturating_add(candidate.average_damage),
            required_card_mask,
            selected,
            best_total,
            best_indices,
        );
        selected.pop();
    }
}

#[cfg(test)]
#[path = "../../../tests/unit/services/taptitan/recommendation_tests.rs"]
mod tests;
