use super::*;
use strum::IntoEnumIterator;

fn sample_pattern() -> SimPatternResult {
    SimPatternResult {
        pattern: "1-2-3".to_string(),
        average_damage: 1_000,
        average_damage_display: "1.000K".to_string(),
        lowest_round_damage: 800,
        lowest_round_damage_display: "800".to_string(),
        highest_round_damage: 1_200,
        highest_round_damage_display: "1.200K".to_string(),
        card_damage: vec![
            SimCardDamageResult {
                card: CardName::MoonBeam,
                card_name: "Moon Beam".to_string(),
                average_damage: 400,
                average_damage_display: "400".to_string(),
            },
            SimCardDamageResult {
                card: CardName::TeamTactics,
                card_name: "Team Tactics".to_string(),
                average_damage: 0,
                average_damage_display: "0".to_string(),
            },
            SimCardDamageResult {
                card: CardName::AstralEcho,
                card_name: "Astral Echo".to_string(),
                average_damage: 250,
                average_damage_display: "250".to_string(),
            },
        ],
    }
}

#[test]
fn narrow_drops_deck_level_average_damage_and_display_fields() {
    let narrowed = narrow_for_persist(&sample_pattern());
    assert_eq!(narrowed.pattern, "1-2-3");
    assert_eq!(narrowed.deck_lowest_damage, 800);
    assert_eq!(narrowed.deck_highest_damage, 1_200);
    assert_eq!(narrowed.card1, CardName::MoonBeam);
    assert_eq!(narrowed.card2, CardName::TeamTactics);
    assert_eq!(narrowed.card3, CardName::AstralEcho);
    assert_eq!(narrowed.card1_damage, 400);
    assert_eq!(narrowed.card2_damage, 0);
    assert_eq!(narrowed.card3_damage, 250);
}

fn mask_for(cards: &[CardName]) -> u64 {
    cards.iter().fold(0u64, |mask, card| {
        let index = CardName::iter()
            .position(|candidate| candidate == *card)
            .expect("test card should have a mask index");
        mask | (1u64 << index)
    })
}

#[test]
fn rebuild_regenerates_display_fields_and_deck_from_card_mask() {
    let narrowed = narrow_for_persist(&sample_pattern());
    let mask = mask_for(&[CardName::MoonBeam, CardName::TeamTactics, CardName::AstralEcho]);

    let rebuilt = rebuild_from_persisted(
        narrowed.pattern,
        narrowed.card1,
        narrowed.card2,
        narrowed.card3,
        narrowed.card1_damage,
        narrowed.card2_damage,
        narrowed.card3_damage,
        narrowed.deck_lowest_damage,
        narrowed.deck_highest_damage,
        mask,
        1_000,
        7,
        RecommendationPhase::Void,
    );

    assert_eq!(rebuilt.total_attack_patterns, 7);
    assert_eq!(rebuilt.simulation_phase, SimulationPhase::TargetedBody);
    assert!(rebuilt.deck.contains(&CardName::MoonBeam));
    assert!(rebuilt.deck.contains(&CardName::TeamTactics));
    assert!(rebuilt.deck.contains(&CardName::AstralEcho));
    assert_eq!(rebuilt.deck.len(), rebuilt.deck_names.len());

    let best_pattern = rebuilt.best_pattern.expect("best_pattern should rebuild");
    assert_eq!(best_pattern.pattern, "1-2-3");
    assert_eq!(best_pattern.average_damage, 1_000);
    assert_eq!(best_pattern.average_damage_display, format_compact(1_000));
    assert_eq!(best_pattern.lowest_round_damage, 800);
    assert_eq!(best_pattern.highest_round_damage, 1_200);
    let moon_beam = best_pattern
        .card_damage
        .iter()
        .find(|card| card.card == CardName::MoonBeam)
        .expect("MoonBeam card_damage should rebuild");
    assert_eq!(moon_beam.card_name, "Moon Beam");
    assert_eq!(moon_beam.average_damage, 400);
    assert_eq!(moon_beam.average_damage_display, format_compact(400));
}
