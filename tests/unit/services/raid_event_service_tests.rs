use super::*;

#[test]
fn supplied_attack_samples_have_expected_totals_and_transition_indices() {
    let cases = [
        (
            include_str!("../../../exampleSocketdatajson/beforebosstrans.json"),
            449_811_765,
            1,
        ),
        (
            include_str!("../../../exampleSocketdatajson/bosstrans.json"),
            835_323_242,
            1,
        ),
        (
            include_str!("../../../exampleSocketdatajson/afterbosstrans.json"),
            479_865_018,
            2,
        ),
    ];
    for (raw, expected_total, resulting_index) in cases {
        let event: AttackEvent = serde_json::from_str(raw).unwrap();
        let components = attack_components(&event).unwrap();
        assert_eq!(
            components.iter().map(|part| part.total_damage).sum::<u64>(),
            expected_total
        );
        assert_eq!(components[0].card_name, "Tap");
        assert_eq!(components[0].titan_index, 1);
        assert_eq!(event.raid_state.titan_index, resulting_index);
    }
}

#[test]
fn persisted_attack_payload_excludes_ephemeral_live_boss_hp() {
    let raw: Value = serde_json::from_str(include_str!(
        "../../../exampleSocketdatajson/afterbosstrans.json"
    ))
    .unwrap();
    let sanitized = without_live_boss_data(raw);
    assert!(sanitized.pointer("/raid_state/current").is_none());
    assert_eq!(
        sanitized.pointer("/raid_state/titan_index"),
        Some(&serde_json::json!(2))
    );
}

#[tokio::test]
async fn attack_updates_ephemeral_live_boss_without_a_database() {
    let state = Arc::new(AppState::new(None, 1, "test-key".to_string(), None));
    let raw: Value = serde_json::from_str(include_str!(
        "../../../exampleSocketdatajson/afterbosstrans.json"
    ))
    .unwrap();

    assert!(handle_event(&state, "attack", raw).await.is_err());

    let live = state.live_attack_boss.read().await.clone().unwrap();
    assert_eq!(live.titan_index, 2);
    assert_eq!(live.boss_data["enemy_id"], "Enemy8");
    assert_eq!(live.boss_data["current_hp"], 107_028_000_000.0);
}

#[test]
fn raid_snapshot_uses_part_totals_targets_and_live_modifiers() {
    let event: SubCycleEvent = serde_json::from_str(include_str!(
        "../../../exampleSocketdatajson/sub_cycle_example.json"
    ))
    .unwrap();
    let (boss, targets) =
        boss_from_raid_snapshot(&event.raid, &event.titan_target, "Enemy8", false).unwrap();
    assert_eq!(boss.boss_name, BossName::Priker);
    assert_eq!(boss.head.max_armor, 29_967_840_000);
    assert_eq!(boss.head.current_armor, 28_358_456_000);
    assert_eq!(
        boss.global_raid_modifier,
        GlobalRaidModifier::AfflictionDuration
    );
    assert_eq!(boss.global_raid_modifier_amount, Some(0.5));
    assert_eq!(boss.curse_type, CurseType::BodyDamage);
    assert_eq!(boss.curse_damage_per_curse, 0.06);
    assert!(targets.contains(&BossPartName::Head));
    assert!(!targets.contains(&BossPartName::RightLeg));
    let enemy = event
        .raid
        .titans
        .iter()
        .find(|titan| titan.enemy_id == "Enemy8")
        .unwrap();
    let game_left_shoulder_armor = enemy
        .parts
        .iter()
        .find(|part| part.part_id == "ArmorArmUpperLeft")
        .unwrap();
    assert_eq!(
        boss.right_shoulder.current_armor,
        rounded_u64(game_left_shoulder_armor.current_hp, "ArmorArmUpperLeft").unwrap()
    );
    assert_eq!(
        target_part_name("ArmUpperRight").unwrap(),
        BossPartName::LeftShoulder
    );
    assert_eq!(
        target_part_name("HandLeft").unwrap(),
        BossPartName::RightHand
    );
}

#[test]
fn sub_start_supplies_base_raid_and_sub_cycle_supplies_only_targets() {
    let sub_start: SubStartEvent = serde_json::from_str(include_str!(
        "../../../exampleSocketdatajson/sub_start_example.json"
    ))
    .unwrap();
    let sub_cycle: SubCycleEvent = serde_json::from_str(include_str!(
        "../../../exampleSocketdatajson/sub_cycle_example.json"
    ))
    .unwrap();

    assert_eq!(sub_start.raid_id, 3_318_220);
    assert_eq!(sub_start.raid.spawn_sequence.len(), 6);
    assert_eq!(sub_start.raid.titans.len(), 3);
    assert_eq!(sub_start.morale.bonus_amount, 0.39);
    assert_eq!(
        sub_start
            .start_at
            .with_timezone(&chrono::FixedOffset::east_opt(7 * 60 * 60).unwrap())
            .to_rfc3339(),
        "2026-08-20T03:31:24+07:00"
    );

    let (boss, targets) =
        boss_from_raid_snapshot(&sub_start.raid, &sub_cycle.titan_target, "Enemy3", false).unwrap();
    assert_eq!(boss.boss_name, BossName::Jukk);
    assert_eq!(boss.global_raid_modifier, GlobalRaidModifier::BurstDamage);
    assert_eq!(boss.global_raid_modifier_amount, Some(0.3));
    assert_eq!(boss.curse_type, CurseType::BodyDamage);
    assert!(targets.contains(&BossPartName::Torso));
    assert!(!targets.contains(&BossPartName::Head));
}

#[test]
fn sub_cycle_raid_is_used_only_when_sub_start_is_missing() {
    let sub_start: SubStartEvent = serde_json::from_str(include_str!(
        "../../../exampleSocketdatajson/sub_start_example.json"
    ))
    .unwrap();
    let sub_cycle: SubCycleEvent = serde_json::from_str(include_str!(
        "../../../exampleSocketdatajson/sub_cycle_example.json"
    ))
    .unwrap();

    let (fallback, used_fallback) = select_base_raid(None, false, &sub_cycle.raid).unwrap();
    assert!(used_fallback);
    assert_eq!(fallback.spawn_sequence, sub_cycle.raid.spawn_sequence);

    let stored = serde_json::to_value(&sub_start.raid).unwrap();
    let (authoritative, used_fallback) =
        select_base_raid(Some(stored), true, &sub_cycle.raid).unwrap();
    assert!(!used_fallback);
    assert_eq!(authoritative.spawn_sequence, sub_start.raid.spawn_sequence);
    assert_ne!(authoritative.spawn_sequence, sub_cycle.raid.spawn_sequence);
}

#[test]
fn attack_part_values_flip_game_left_and_right() {
    let live = AttackCurrentBoss {
        enemy_id: "Enemy8".to_string(),
        current_hp: 1_010.0,
        parts: vec![
            AttackCurrentBossPart {
                part_id: "BodyArmUpperLeft".to_string(),
                current_hp: 101.0,
            },
            AttackCurrentBossPart {
                part_id: "ArmorArmUpperLeft".to_string(),
                current_hp: 202.0,
            },
            AttackCurrentBossPart {
                part_id: "BodyArmUpperRight".to_string(),
                current_hp: 303.0,
            },
            AttackCurrentBossPart {
                part_id: "ArmorArmUpperRight".to_string(),
                current_hp: 404.0,
            },
        ],
    };

    assert_eq!(
        attack_part_values(&live, BossPartName::RightShoulder).unwrap(),
        Some((202, 101))
    );
    assert_eq!(
        attack_part_values(&live, BossPartName::LeftShoulder).unwrap(),
        Some((404, 303))
    );
}

#[test]
fn live_boss_display_combines_attack_hp_with_sub_cycle_metadata() {
    let mut attack: AttackEvent = serde_json::from_str(include_str!(
        "../../../exampleSocketdatajson/afterbosstrans.json"
    ))
    .unwrap();
    let sub_cycle: SubCycleEvent = serde_json::from_str(include_str!(
        "../../../exampleSocketdatajson/sub_cycle_example.json"
    ))
    .unwrap();

    for part in &mut attack.raid_state.current.parts {
        match part.part_id.as_str() {
            "ArmorHead" => part.current_hp = 0.0,
            "BodyHead" => part.current_hp = 123_456_789.0,
            "ArmorChestUpper" | "BodyChestUpper" => part.current_hp = 0.0,
            _ => {}
        }
    }
    let parts = live_boss_display_parts(
        &serde_json::to_value(&attack.raid_state.current).unwrap(),
        &serde_json::to_value(&sub_cycle.raid).unwrap(),
        &serde_json::to_value(&sub_cycle.titan_target).unwrap(),
    )
    .unwrap()
    .unwrap();

    assert_eq!(parts.len(), 8);
    let head = parts
        .iter()
        .find(|part| part.part_name == BossPartName::Head)
        .unwrap();
    assert_eq!(head.part_state, PartState::Body);
    assert_eq!(head.current_hp, 123_456_789);
    assert_eq!(head.max_hp, 43_667_424_000);
    assert!(head.is_targeted);
    let torso = parts
        .iter()
        .find(|part| part.part_name == BossPartName::Torso)
        .unwrap();
    assert_eq!(torso.part_state, PartState::Skeleton);
    assert_eq!(torso.current_hp, 0);
    let right_shoulder = parts
        .iter()
        .find(|part| part.part_name == BossPartName::RightShoulder)
        .unwrap();
    assert_eq!(right_shoulder.part_state, PartState::Cursed);
    assert_eq!(right_shoulder.current_hp, 5_613_024_000);
    assert_eq!(right_shoulder.max_hp, 8_027_100_000);
    let right_leg = parts
        .iter()
        .find(|part| part.part_name == BossPartName::RightLeg)
        .unwrap();
    assert!(!right_leg.is_targeted);

    let mut incomplete_raid = serde_json::to_value(&sub_cycle.raid).unwrap();
    incomplete_raid["titans"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|titan| titan["enemy_id"] == "Enemy8")
        .unwrap()["parts"]
        .as_array_mut()
        .unwrap()
        .pop();
    assert!(
        live_boss_display_parts(
            &serde_json::to_value(&attack.raid_state.current).unwrap(),
            &incomplete_raid,
            &serde_json::to_value(&sub_cycle.titan_target).unwrap(),
        )
        .unwrap()
        .is_none()
    );

    attack.raid_state.current.enemy_id = "MissingEnemy".to_string();
    assert!(
        live_boss_display_parts(
            &serde_json::to_value(&attack.raid_state.current).unwrap(),
            &serde_json::to_value(&sub_cycle.raid).unwrap(),
            &serde_json::to_value(&sub_cycle.titan_target).unwrap(),
        )
        .unwrap()
        .is_none()
    );
}

#[test]
fn attack_updates_sims_boss_once_when_every_target_becomes_body() {
    let event: SubCycleEvent = serde_json::from_str(include_str!(
        "../../../exampleSocketdatajson/sub_cycle_example.json"
    ))
    .unwrap();
    let (mut boss, _) =
        boss_from_raid_snapshot(&event.raid, &event.titan_target, "Enemy8", false).unwrap();
    let targets = [BossPartName::Head, BossPartName::Torso];
    boss.head.part_state = PartState::Armor;
    boss.torso.part_state = PartState::Armor;
    let mut incoming = boss.clone();
    incoming.head.current_armor = 0;
    incoming.head.current_health = 15_000_000_000;
    incoming.torso.current_armor = 0;
    incoming.torso.current_health = 15_000_000_000;
    incoming.sync_part_states_from_current_values();

    assert_eq!(
        classify_phase_refresh(&boss, &targets, &incoming),
        Some(PhaseRefresh::Full)
    );

    assert_eq!(classify_phase_refresh(&incoming, &targets, &incoming), None);
}

#[test]
fn attack_waits_until_every_target_is_body() {
    let event: SubCycleEvent = serde_json::from_str(include_str!(
        "../../../exampleSocketdatajson/sub_cycle_example.json"
    ))
    .unwrap();
    let (mut boss, _) =
        boss_from_raid_snapshot(&event.raid, &event.titan_target, "Enemy8", false).unwrap();
    let targets = [BossPartName::Head, BossPartName::Torso];
    boss.head.part_state = PartState::Armor;
    boss.torso.part_state = PartState::Armor;
    let mut incoming = boss.clone();
    incoming.head.current_armor = 0;
    incoming.head.current_health = 15_000_000_000;
    incoming.sync_part_states_from_current_values();

    assert_eq!(classify_phase_refresh(&boss, &targets, &incoming), None);
}

#[test]
fn selected_body_to_skeleton_is_incremental_and_non_selected_is_ignored() {
    let event: SubCycleEvent = serde_json::from_str(include_str!(
        "../../../exampleSocketdatajson/sub_cycle_example.json"
    ))
    .unwrap();
    let (mut boss, _) =
        boss_from_raid_snapshot(&event.raid, &event.titan_target, "Enemy8", false).unwrap();
    boss.head.current_armor = 0;
    boss.head.current_health = 10;
    boss.head.sync_state_from_current_values();
    boss.torso.current_armor = 0;
    boss.torso.current_health = 10;
    boss.torso.sync_state_from_current_values();
    let targets = [BossPartName::Head];

    let mut incoming = boss.clone();
    incoming.head.current_health = 0;
    incoming.head.sync_state_from_current_values();
    assert_eq!(
        classify_phase_refresh(&boss, &targets, &incoming),
        Some(PhaseRefresh::Incremental(
            BossPartName::Head.dependency_mask()
        ))
    );

    let multi_targets = [BossPartName::Head, BossPartName::Torso];
    let mut multiple = boss.clone();
    multiple.head.current_health = 0;
    multiple.torso.current_health = 0;
    multiple.sync_part_states_from_current_values();
    assert_eq!(
        classify_phase_refresh(&boss, &multi_targets, &multiple),
        Some(PhaseRefresh::Incremental(
            BossPartName::Head.dependency_mask() | BossPartName::Torso.dependency_mask()
        ))
    );

    let mut non_selected = boss.clone();
    non_selected.torso.current_health = 0;
    non_selected.torso.sync_state_from_current_values();
    assert_eq!(classify_phase_refresh(&boss, &targets, &non_selected), None);

    assert_eq!(
        classify_phase_refresh(&incoming, &targets, &boss),
        None,
        "stale events must not move skeleton back to body"
    );

    let mut hp_only = boss.clone();
    hp_only.head.current_health -= 1;
    hp_only.head.sync_state_from_current_values();
    assert_eq!(classify_phase_refresh(&boss, &targets, &hp_only), None);
}

#[test]
fn selected_armor_skipping_directly_to_skeleton_requires_full_refresh() {
    let event: SubCycleEvent = serde_json::from_str(include_str!(
        "../../../exampleSocketdatajson/sub_cycle_example.json"
    ))
    .unwrap();
    let (mut boss, _) =
        boss_from_raid_snapshot(&event.raid, &event.titan_target, "Enemy8", false).unwrap();
    boss.head.current_armor = 10;
    boss.head.current_health = 10;
    boss.head.part_state = PartState::Armor;
    let mut incoming = boss.clone();
    incoming.head.current_armor = 0;
    incoming.head.current_health = 0;
    incoming.head.sync_state_from_current_values();

    assert_eq!(
        classify_phase_refresh(&boss, &[BossPartName::Head], &incoming),
        Some(PhaseRefresh::Full)
    );
}

#[test]
fn cycle_sample_produces_expected_percentages_and_reset_boundary() {
    let event: CycleResetEvent = serde_json::from_str(include_str!(
        "../../../exampleSocketdatajson/cycle_reset_example.json"
    ))
    .unwrap();
    let team = bonus_value(&event.card_bonuses, "TeamTacticsClanMoraleBoost");
    let mirror = bonus_value(&event.card_bonuses, "MirrorForceBoost");
    assert!(((event.morale.bonus_amount + team) * 100.0 - 44.3).abs() < 1e-9);
    assert!((1.0 + mirror - 1.35).abs() < 1e-9);
    assert_eq!(
        next_reset_boundary(event.raid_started_at, event.started_at),
        event.next_reset_at
    );
}
