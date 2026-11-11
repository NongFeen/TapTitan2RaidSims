use serde_json::json;

use super::*;

fn part(name: &str) -> serde_json::Value {
    json!({
        "part_name": name,
        "part_state": "Body",
        "max_armor": 100,
        "max_health": 100,
        "current_armor": 0,
        "current_health": 100
    })
}

fn boss() -> Boss {
    serde_json::from_value(json!({
        "boss_name": "Jukk",
        "head": part("Head"),
        "torso": part("Torso"),
        "left_shoulder": part("LeftShoulder"),
        "right_shoulder": part("RightShoulder"),
        "left_hand": part("LeftHand"),
        "right_hand": part("RightHand"),
        "left_leg": part("LeftLeg"),
        "right_leg": part("RightLeg")
    }))
    .unwrap()
}

fn card(id: &str, cardtype: &str) -> Card {
    serde_json::from_value(json!({
        "card_id": id,
        "cardtype": cardtype,
        "level": 1
    }))
    .unwrap()
}

#[test]
fn direct_patterns_mark_only_their_candidate_parts() {
    let mask = dependency_part_mask_for(
        &boss(),
        &[BossPartName::Head, BossPartName::Torso],
        &[card("MoonBeam", "Burst")],
        &[AttackPattern::SingleHead],
    );
    assert_eq!(mask, BossPartName::Head.dependency_mask());
}

#[test]
fn global_state_cards_depend_on_every_part() {
    let mask = dependency_part_mask_for(
        &boss(),
        &[BossPartName::Head],
        &[card("FinisherAttack", "Support")],
        &[AttackPattern::SingleHead],
    );
    assert_eq!(mask, u8::MAX);
}

/// Debug dump of the PreDeterminedProc cache for one deck.
/// Edit the levels below to match your cards, then run:
///   cargo test --bins debug_proc_cache_for_deck -- --ignored --nocapture
#[test]
#[ignore = "debug output only"]
fn debug_proc_cache_for_deck() {
    const MIRROR_FORCE_LEVEL: u16 = 60;
    const CRUSHING_INSTINCT_LEVEL: u16 = 60;
    const ANCESTRAL_FAVOR_LEVEL: u16 = 60;

    let deck_for = |boss: &Boss| {
        let mut deck = vec![
            card_at("MirrorForce", "Burst", MIRROR_FORCE_LEVEL),
            card_at("ExecutionersAxe", "Support", CRUSHING_INSTINCT_LEVEL), // Crushing Instinct
            card_at("BurstBoost", "Support", ANCESTRAL_FAVOR_LEVEL),        // Ancestral Favor
        ];
        prepare_deck_for_sim(&mut deck, boss);
        deck
    };

    // (label, arena buff, amount the game sent)
    let cases: [(&str, &str, Option<f64>); 4] = [
        ("no arena buff", "None", None),
        ("+3s, no amount (default 3.0)", "AttackDuration", None),
        ("+3s, amount = 3", "AttackDuration", Some(3.0)),
        ("+3s, amount = 0.1", "AttackDuration", Some(0.1)),
    ];

    for (label, modifier, amount) in cases {
        let mut boss_json = serde_json::to_value(boss()).unwrap();
        boss_json["global_raid_modifier"] = json!(modifier);
        boss_json["global_raid_modifier_amount"] = json!(amount);
        let boss: Boss = serde_json::from_value(boss_json).unwrap();

        let deck = deck_for(&boss);
        let support = support_modifiers_for_deck(&deck, &boss);
        let ticks = SimService::deck_tick_count(&deck, &boss);
        let mirror_force = deck.iter().find(|c| c.card_id == CardName::MirrorForce).unwrap();
        let base_chance = mirror_force.get_proc_chance(&boss);

        let mut cache = PreDeterminedProc::new();
        cache.generate_proc_count(&deck, &boss, ticks);

        println!("\n=== {label} ===");
        println!("attack_duration_add_seconds = {:.2}", support.attack_duration_add_seconds);
        println!("ticks                       = {ticks}");
        println!("mirror force base chance    = {:.4}", base_chance);
        println!("burst_chance_mult (support) = {:.4}", support.burst_chance_mult);
        println!(
            "expected MF procs           = {:.2}  (chance x mult x ticks)",
            (base_chance * support.burst_chance_mult).min(1.0) * ticks as f64
        );
        cache.print_all();
    }
}

fn card_at(id: &str, cardtype: &str, level: u16) -> Card {
    serde_json::from_value(json!({
        "card_id": id,
        "cardtype": cardtype,
        "level": level
    }))
    .unwrap()
}
