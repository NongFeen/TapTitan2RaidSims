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
