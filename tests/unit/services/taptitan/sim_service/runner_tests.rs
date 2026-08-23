use serde_json::json;

use super::*;

fn part(name: &str, state: &str, armor: u64, health: u64) -> serde_json::Value {
    json!({
        "part_name": name,
        "part_state": state,
        "max_armor": 100,
        "max_health": 200,
        "current_armor": armor,
        "current_health": health
    })
}

fn boss() -> Boss {
    serde_json::from_value(json!({
        "boss_name": "Jukk",
        "head": part("Head", "Cursed", 75, 180),
        "torso": part("Torso", "Armor", 50, 170),
        "left_shoulder": part("LeftShoulder", "Body", 0, 160),
        "right_shoulder": part("RightShoulder", "Body", 0, 150),
        "left_hand": part("LeftHand", "Body", 0, 140),
        "right_hand": part("RightHand", "Skeleton", 0, 0),
        "left_leg": part("LeftLeg", "Cursed", 25, 130),
        "right_leg": part("RightLeg", "Armor", 20, 120)
    }))
    .expect("test boss should deserialize")
}

fn pattern(damage: u64) -> SimPatternResult {
    SimPatternResult {
        pattern: "test".to_string(),
        average_damage: damage,
        average_damage_display: damage.to_string(),
        lowest_round_damage: damage,
        lowest_round_damage_display: damage.to_string(),
        highest_round_damage: damage,
        highest_round_damage_display: damage.to_string(),
        card_damage: Vec::new(),
    }
}

fn deck(cards: Vec<CardName>, damage: u64, phase: SimulationPhase) -> SimDeckResult {
    SimDeckResult {
        deck_names: cards
            .iter()
            .map(|card| card.display_name().to_string())
            .collect(),
        deck: cards,
        total_attack_patterns: 1,
        best_pattern: Some(pattern(damage)),
        simulation_phase: phase,
        patterns: Vec::new(),
        dependency_part_mask: 0,
    }
}

#[test]
fn body_phase_deck_filter_only_accepts_insanity_void_decks() {
    let insanity_void: Card = serde_json::from_value(json!({
        "card_id": "CrushingVoid",
        "cardtype": "Support",
        "level": 1
    }))
    .expect("Insanity Void card should deserialize");
    let razor_wind: Card = serde_json::from_value(json!({
        "card_id": "RazorWind",
        "cardtype": "Burst",
        "level": 1
    }))
    .expect("Razor Wind card should deserialize");

    assert!(deck_matches_required_card(
        &[razor_wind.clone(), insanity_void],
        Some(CardName::InsanityVoid)
    ));
    assert!(!deck_matches_required_card(
        &[razor_wind],
        Some(CardName::InsanityVoid)
    ));
    assert!(deck_matches_required_card(&[], None));
}

#[test]
fn body_phase_requires_enabled_and_convertible_target() {
    let boss = boss();
    let single_armor_target = [BossPartName::Head];
    assert!(!should_run_targeted_body_phase(
        false,
        &boss,
        &single_armor_target
    ));
    assert!(should_run_targeted_body_phase(
        true,
        &boss,
        &single_armor_target
    ));
    assert!(!should_run_targeted_body_phase(true, &boss, &[]));

    let body_target = [BossPartName::LeftShoulder];
    assert!(!should_run_targeted_body_phase(true, &boss, &body_target));
}

#[test]
fn conversion_changes_only_targeted_armor_and_curse_parts() {
    let mut boss = boss();
    convert_targeted_armor_to_body(
        &mut boss,
        &[
            BossPartName::Head,
            BossPartName::Torso,
            BossPartName::LeftShoulder,
            BossPartName::RightHand,
            BossPartName::LeftLeg,
        ],
    );

    assert_eq!(boss.head.part_state, PartState::Body);
    assert_eq!(boss.head.current_armor, 0);
    assert_eq!(boss.head.current_health, 180);
    assert_eq!(boss.torso.part_state, PartState::Body);
    assert_eq!(boss.left_shoulder.current_health, 160);
    assert_eq!(boss.right_hand.part_state, PartState::Skeleton);
    assert_eq!(boss.right_leg.part_state, PartState::Armor);
    assert_eq!(boss.right_leg.current_armor, 20);
}

#[test]
fn void_phase_replaces_matching_current_decks_even_when_damage_is_lower() {
    let cards = vec![
        CardName::RazorWind,
        CardName::AncestralFavor,
        CardName::InsanityVoid,
    ];
    let tied_cards = vec![
        CardName::MoonBeam,
        CardName::TeamTactics,
        CardName::AcidDrench,
    ];
    let mut current = vec![
        deck(cards.clone(), 100, SimulationPhase::Current),
        deck(tied_cards.clone(), 200, SimulationPhase::Current),
    ];
    let body = vec![deck(
        cards.into_iter().rev().collect(),
        90,
        SimulationPhase::TargetedBody,
    )];

    replace_required_card_deck_results(&mut current, body, CardName::InsanityVoid);

    assert_eq!(current.len(), 2);
    let void_deck = current
        .iter()
        .find(|result| result.deck.contains(&CardName::InsanityVoid))
        .unwrap();
    let non_void_deck = current
        .iter()
        .find(|result| !result.deck.contains(&CardName::InsanityVoid))
        .unwrap();
    assert_eq!(void_deck.simulation_phase, SimulationPhase::TargetedBody);
    assert_eq!(void_deck.best_pattern.as_ref().unwrap().average_damage, 90);
    assert_eq!(non_void_deck.simulation_phase, SimulationPhase::Current);
}
