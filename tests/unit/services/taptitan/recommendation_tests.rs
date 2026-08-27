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

    let mirror_only = optimize_decks_with_required_cards(&candidates, 2, &[CardName::MirrorForce])
        .expect("Mirror Force should fit into two compatible decks");
    let mirror_used = mirror_only
        .decks
        .iter()
        .fold(0u64, |used, deck| used | deck.card_mask);
    assert_ne!(mirror_used & mask(&[CardName::MirrorForce]), 0);

    let team_only = optimize_decks_with_required_cards(&candidates, 2, &[CardName::TeamTactics])
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

#[test]
fn cards_from_mask_round_trips_with_the_encode_direction() {
    let cards = [CardName::MoonBeam, CardName::TeamTactics, CardName::BattleDrums];
    let round_tripped = cards_from_mask(mask(&cards));
    assert_eq!(round_tripped.len(), 3);
    for card in cards {
        assert!(round_tripped.contains(&card));
    }
}

#[test]
fn cards_from_mask_of_zero_is_empty() {
    assert!(cards_from_mask(0).is_empty());
}
