use serde_json::json;

use super::*;

fn part(part_name: &str, current_armor: u64, current_health: u64) -> serde_json::Value {
    json!({
        "part_name": part_name,
        "part_state": "Cursed",
        "max_armor": 100,
        "max_health": 100,
        "current_armor": current_armor,
        "current_health": current_health
    })
}

#[test]
fn syncs_part_states_from_durability_without_erasing_curses() {
    let mut boss: Boss = serde_json::from_value(json!({
        "boss_name": "Jukk",
        "head": part("Head", 10, 20),
        "torso": part("Torso", 1, 0),
        "left_shoulder": part("LeftShoulder", 0, 5),
        "right_shoulder": part("RightShoulder", 0, 1),
        "left_hand": part("LeftHand", 0, 0),
        "right_hand": part("RightHand", 0, 0),
        "left_leg": part("LeftLeg", 99, 0),
        "right_leg": part("RightLeg", 0, 99)
    }))
    .expect("test boss should deserialize");

    boss.sync_part_states_from_current_values();

    let actual = boss.parts().map(|part| part.part_state);
    assert_eq!(
        actual,
        [
            PartState::Cursed,
            PartState::Cursed,
            PartState::Body,
            PartState::Body,
            PartState::Skeleton,
            PartState::Skeleton,
            PartState::Cursed,
            PartState::Body,
        ]
    );
}

#[test]
fn curse_count_is_frozen_and_applies_by_damage_type() {
    let mut boss: Boss = serde_json::from_value(json!({
        "boss_name": "Jukk",
        "curse_type": "BurstDamage",
        "head": part("Head", 100, 100),
        "torso": part("Torso", 100, 100),
        "left_shoulder": part("LeftShoulder", 0, 100),
        "right_shoulder": part("RightShoulder", 0, 100),
        "left_hand": part("LeftHand", 0, 0),
        "right_hand": part("RightHand", 0, 0),
        "left_leg": part("LeftLeg", 0, 0),
        "right_leg": part("RightLeg", 0, 0)
    }))
    .expect("boss should deserialize");
    boss.sync_part_states_from_current_values();
    boss.snapshot_initial_curse_parts();

    assert!(
        (boss.curse_damage_multiplier(PartState::Armor, Some(CardType::Burst)) - 0.88).abs()
            < f32::EPSILON
    );
    assert_eq!(
        boss.curse_damage_multiplier(PartState::Armor, Some(CardType::Affliction)),
        1.0
    );

    boss.head.on_hit(100);
    assert_eq!(boss.head.part_state, PartState::Body);
    assert!(
        (boss.curse_damage_multiplier(PartState::Body, Some(CardType::Burst)) - 0.88).abs()
            < f32::EPSILON
    );
}

#[test]
fn boss_accumulates_fractional_damage_before_integer_reporting() {
    let mut boss: Boss = serde_json::from_value(json!({
        "boss_name": "Jukk",
        "head": part("Head", 100, 100),
        "torso": part("Torso", 100, 100),
        "left_shoulder": part("LeftShoulder", 100, 100),
        "right_shoulder": part("RightShoulder", 100, 100),
        "left_hand": part("LeftHand", 100, 100),
        "right_hand": part("RightHand", 100, 100),
        "left_leg": part("LeftLeg", 100, 100),
        "right_leg": part("RightLeg", 100, 100)
    }))
    .expect("boss should deserialize");
    boss.head.max_armor = 100_000;
    boss.head.current_armor = 100_000;

    boss.on_hit_with_source(BossPartName::Head, 4_993.5, DamageSource::Tap);
    assert_eq!(boss.get_total_damage(), 4_993);

    boss.on_hit_with_source(BossPartName::Head, 4_993.5, DamageSource::Tap);
    assert_eq!(boss.get_total_damage(), 9_987);
    assert_eq!(boss.head.current_armor, 90_013);
}
