use serde_json::json;

use super::*;

fn part(name: &str, state: &str, armor: u64, health: u64) -> serde_json::Value {
    json!({
        "part_name": name,
        "part_state": state,
        "max_armor": 100,
        "max_health": 100,
        "current_armor": armor,
        "current_health": health
    })
}

fn boss() -> Boss {
    serde_json::from_value(json!({
        "boss_name": "Jukk",
        "head": part("Head", "Body", 0, 50),
        "torso": part("Torso", "Body", 0, 50),
        "left_shoulder": part("LeftShoulder", "Armor", 50, 100),
        "right_shoulder": part("RightShoulder", "Body", 0, 50),
        "left_hand": part("LeftHand", "Body", 0, 50),
        "right_hand": part("RightHand", "Body", 0, 50),
        "left_leg": part("LeftLeg", "Body", 0, 50),
        "right_leg": part("RightLeg", "Body", 0, 50)
    }))
    .unwrap()
}

#[test]
fn incremental_base_accepts_only_body_to_skeleton_changes() {
    let base = boss();
    let mut skeleton = base.clone();
    skeleton.head.current_health = 0;
    skeleton.head.sync_state_from_current_values();
    assert_eq!(
        incremental_boss_change_mask(&base, &skeleton),
        Some(BossPartName::Head.dependency_mask())
    );

    let mut armor_to_body = base.clone();
    armor_to_body.left_shoulder.current_armor = 0;
    armor_to_body.left_shoulder.sync_state_from_current_values();
    assert_eq!(incremental_boss_change_mask(&base, &armor_to_body), None);
}
