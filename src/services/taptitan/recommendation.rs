use std::collections::HashMap;

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

pub fn optimize_decks(
    candidates: &[CandidateDeck],
    deck_count: usize,
) -> Option<DeckRecommendation> {
    if deck_count == 0 {
        return Some(DeckRecommendation {
            deck_count,
            total_average_damage: 0,
            decks: Vec::new(),
        });
    }

    let mut sorted = candidates.to_vec();
    sorted.sort_by_key(|candidate| std::cmp::Reverse(candidate.average_damage));

    let mut best_total = 0u64;
    let mut best_indices = Vec::new();
    seed_with_greedy(&sorted, deck_count, &mut best_total, &mut best_indices);
    let mut selected = Vec::with_capacity(deck_count);
    search(
        &sorted,
        deck_count,
        0,
        0,
        0,
        &mut selected,
        &mut best_total,
        &mut best_indices,
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
        if selected.len() == target_count && (best_indices.is_empty() || total > *best_total) {
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
    selected: &mut Vec<usize>,
    best_total: &mut u64,
    best_indices: &mut Vec<usize>,
) {
    if selected.len() == target_count {
        if total > *best_total || best_indices.is_empty() {
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
            selected,
            best_total,
            best_indices,
        );
        selected.pop();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn deck(source_index: usize, mask: u64, damage: u64) -> CandidateDeck {
        CandidateDeck {
            source_index,
            cards: Vec::new(),
            card_mask: mask,
            average_damage: damage,
        }
    }

    #[test]
    fn finds_better_combination_than_greedy() {
        let candidates = vec![
            deck(0, 0b000111, 100),
            deck(1, 0b011001, 70),
            deck(2, 0b100110, 70),
        ];

        let result = optimize_decks(&candidates, 2).expect("two compatible decks");
        assert_eq!(result.total_average_damage, 140);
        assert_eq!(result.decks.len(), 2);
    }

    #[test]
    fn returns_none_when_not_enough_disjoint_decks() {
        let candidates = vec![deck(0, 0b111, 100), deck(1, 0b1011, 90)];
        assert!(optimize_decks(&candidates, 2).is_none());
    }

    #[test]
    fn handles_a_full_card_pool() {
        let mut candidates = Vec::new();
        for first in 0..44 {
            for second in (first + 1)..44 {
                for third in (second + 1)..44 {
                    let mask = (1u64 << first) | (1u64 << second) | (1u64 << third);
                    let damage = 1_000_000
                        + ((first as u64 * 73_856_093
                            + second as u64 * 19_349_663
                            + third as u64 * 83_492_791)
                            % 500_000);
                    candidates.push(deck(candidates.len(), mask, damage));
                }
            }
        }

        let result = optimize_decks(&candidates, 9).expect("nine compatible decks");
        assert_eq!(result.decks.len(), 9);
        let used = result
            .decks
            .iter()
            .fold(0u64, |used, candidate| used | candidate.card_mask);
        assert_eq!(used.count_ones(), 27);
    }
}
