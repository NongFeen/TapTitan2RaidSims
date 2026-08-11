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
    let mut next_progress_percent = 10usize;
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
mod tests {
    use super::*;

    fn mask(cards: &[CardName]) -> u64 {
        cards.iter().fold(0u64, |mask, card| {
            let index = CardName::iter()
                .position(|candidate| candidate == *card)
                .expect("test card should have a mask index");
            mask | (1u64 << index)
        })
    }

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
    fn supports_the_maximum_deck_count() {
        let candidates = (0..14)
            .map(|index| {
                let first_card = index * 3;
                deck(
                    index,
                    (1u64 << first_card) | (1u64 << (first_card + 1)) | (1u64 << (first_card + 2)),
                    1_000 - index as u64,
                )
            })
            .collect::<Vec<_>>();

        let result = optimize_decks(&candidates, 14).expect("fourteen compatible decks");
        let used_cards = result
            .decks
            .iter()
            .fold(0u64, |used, candidate| used | candidate.card_mask);

        assert_eq!(result.decks.len(), 14);
        assert_eq!(used_cards.count_ones(), 42);
    }

    #[test]
    fn required_cards_are_present_in_the_combined_lineup() {
        let candidates = vec![
            deck(0, mask(&[CardName::MirrorForce, CardName::MoonBeam]), 100),
            deck(1, mask(&[CardName::TeamTactics, CardName::Fragmentize]), 90),
            deck(2, mask(&[CardName::SkullBash, CardName::RazorWind]), 200),
            deck(
                3,
                mask(&[CardName::PsychicShackles, CardName::FlakShot]),
                180,
            ),
        ];

        let mirror_only =
            optimize_decks_with_required_cards(&candidates, 2, &[CardName::MirrorForce])
                .expect("Mirror Force should fit into two compatible decks");
        let mirror_used = mirror_only
            .decks
            .iter()
            .fold(0u64, |used, deck| used | deck.card_mask);
        assert_ne!(mirror_used & mask(&[CardName::MirrorForce]), 0);

        let team_only =
            optimize_decks_with_required_cards(&candidates, 2, &[CardName::TeamTactics])
                .expect("Team Tactics should fit into two compatible decks");
        let team_used = team_only
            .decks
            .iter()
            .fold(0u64, |used, deck| used | deck.card_mask);
        assert_ne!(team_used & mask(&[CardName::TeamTactics]), 0);

        let both = optimize_decks_with_required_cards(
            &candidates,
            2,
            &[CardName::MirrorForce, CardName::TeamTactics],
        )
        .expect("required cards should fit into two compatible decks");
        let used_cards = both
            .decks
            .iter()
            .fold(0u64, |used, deck| used | deck.card_mask);

        assert_eq!(both.total_average_damage, 190);
        assert_eq!(
            used_cards & mask(&[CardName::MirrorForce, CardName::TeamTactics]),
            mask(&[CardName::MirrorForce, CardName::TeamTactics]),
        );
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

        let required = optimize_decks_with_required_cards(
            &candidates,
            9,
            &[CardName::MirrorForce, CardName::TeamTactics],
        )
        .expect("nine compatible decks containing the required cards");
        let required_used = required
            .decks
            .iter()
            .fold(0u64, |used, candidate| used | candidate.card_mask);
        assert_eq!(
            required_used & mask(&[CardName::MirrorForce, CardName::TeamTactics]),
            mask(&[CardName::MirrorForce, CardName::TeamTactics]),
        );
    }
}
