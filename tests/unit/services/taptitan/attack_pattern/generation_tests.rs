use super::*;

fn card(card_id: CardName) -> Card {
    Card {
        card_id,
        cardtype: card_id.card_type(),
        level: 1,
        enabled: true,
        tap_count: 0,
        chained_parts: Vec::new(),
        celestial_stacks: 0,
        skill: Default::default(),
        proc_chance_cache: 0.0,
    }
}

#[test]
fn cursed_patterns_require_a_curse_target_card() {
    let ordinary_deck = [card(CardName::MoonBeam)];
    let fragmentize_deck = [card(CardName::Fragmentize)];
    let ruinous_rain_deck = [card(CardName::RuinousRain)];

    for pattern in [AttackPattern::SingleCursed, AttackPattern::CycleCursed] {
        assert!(!pattern_is_available_for_deck(&pattern, &ordinary_deck));
        assert!(pattern_is_available_for_deck(&pattern, &fragmentize_deck));
        assert!(pattern_is_available_for_deck(&pattern, &ruinous_rain_deck));
    }
}

#[test]
fn narrow_filter_counts_single_patterns_as_one_attacked_part() {
    let candidates = vec![BossPartName::Head, BossPartName::Torso];
    let single = AttackPatternInfo {
        pattern: AttackPattern::SingleAny,
        candidates: candidates.clone(),
        source_count: candidates.len(),
        priority: (0, 0, 0),
    };
    let cycle = AttackPatternInfo {
        pattern: AttackPattern::CycleAllActive,
        candidates,
        source_count: 2,
        priority: (0, 0, 0),
    };

    assert_eq!(attacked_part_count(&single), 1);
    assert_eq!(attacked_part_count(&cycle), 2);
}
