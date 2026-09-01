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
        assert!(components[0].card_id.is_none());
        assert_eq!(components[0].titan_index, 1);
        assert_eq!(event.raid_state.titan_index, resulting_index);
    }
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
fn first_titan_enemy_id_follows_spawn_order_not_titan_list_order() {
    let sub_start: SubStartEvent = serde_json::from_str(include_str!(
        "../../../exampleSocketdatajson/sub_start_example.json"
    ))
    .unwrap();

    // spawn_sequence[0] is "Jukk", which is Enemy3 in the titans list -- not
    // Enemy2 (Takedar), which happens to be listed first in `titans`.
    assert_eq!(sub_start.raid.spawn_sequence[0], "Jukk");
    assert_eq!(sub_start.raid.titans[0].enemy_id, "Enemy2");
    assert_eq!(first_titan_enemy_id(&sub_start.raid).unwrap(), "Enemy3");
}

#[test]
fn first_titan_enemy_id_falls_back_to_first_titan_when_spawn_sequence_is_empty() {
    let sub_start: SubStartEvent = serde_json::from_str(include_str!(
        "../../../exampleSocketdatajson/sub_start_example.json"
    ))
    .unwrap();
    let mut raid = sub_start.raid;
    raid.spawn_sequence.clear();

    assert_eq!(first_titan_enemy_id(&raid).unwrap(), "Enemy2");
}

#[test]
fn a_brand_new_raids_first_sub_start_has_no_locked_in_target_yet() {
    // The real sample: every part reports state "0" on the very first
    // sub_start of a raid -- nothing has been targeted yet in-game.
    let sub_start: SubStartEvent = serde_json::from_str(include_str!(
        "../../../exampleSocketdatajson/sub_start_example.json"
    ))
    .unwrap();
    assert!(
        sub_start
            .titan_target
            .iter()
            .all(|target| target.state.iter().all(|part| part.state == "0"))
    );

    // Previously this would fail with "sub_cycle selected no attackable
    // titan parts"; now it should default to every part being attackable so
    // a boss can still be set up immediately from sub_start alone.
    let enemy_id = first_titan_enemy_id(&sub_start.raid).unwrap().to_string();
    let (_, attackable_parts) =
        boss_from_raid_snapshot(&sub_start.raid, &sub_start.titan_target, &enemy_id, false)
            .expect("should default to all parts attackable, not error");
    assert_eq!(attackable_parts.len(), 8);
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

#[test]
fn attack_duration_applies_battle_drums_and_global_modifier_adjustments() {
    assert_eq!(
        attack_duration_seconds(&[], GlobalRaidModifier::None),
        30.0
    );
    assert_eq!(
        attack_duration_seconds(&[CardName::BattleDrums], GlobalRaidModifier::None),
        20.0
    );
    assert_eq!(
        attack_duration_seconds(&[], GlobalRaidModifier::AttackDuration),
        33.0
    );
    assert!(
        (attack_duration_seconds(&[], GlobalRaidModifier::SupportEffect) - 18.5).abs() < 1e-9
    );
    // Both adjustments stack: Battle Drums plus a global modifier.
    assert!(
        (attack_duration_seconds(
            &[CardName::BattleDrums],
            GlobalRaidModifier::SupportEffect
        ) - 8.5)
            .abs()
            < 1e-9
    );
    // A modifier unrelated to duration leaves the base window untouched.
    assert_eq!(
        attack_duration_seconds(&[], GlobalRaidModifier::BurstDamage),
        30.0
    );
}

#[test]
fn start_attack_sample_parses_into_expected_player_and_cards() {
    let event: StartAttackEvent = serde_json::from_str(include_str!(
        "../../../exampleSocketdatajson/start_attack.json"
    ))
    .unwrap();
    assert_eq!(event.player.player_code, "933qd64");
    assert_eq!(event.player.name, "12 Kero -> Feen");
    assert_eq!(
        event.cards,
        vec!["CosmicBarb", "MentalFocus", "InnerTruth"]
    );
}

#[tokio::test]
async fn start_attack_registers_a_live_attacking_player_without_a_database() {
    let state = Arc::new(AppState::new(None, 1, "test-key".to_string(), None));
    let raw: Value = serde_json::from_str(include_str!(
        "../../../exampleSocketdatajson/start_attack.json"
    ))
    .unwrap();

    handle_event(&state, "start_attack", raw).await.unwrap();

    let players = state.live_attacking_players.read().await;
    let player = players.get("933qd64").unwrap();
    assert_eq!(player.name, "12 Kero -> Feen");
    assert_eq!(player.cards.len(), 3);
    assert_eq!(player.cards[0].card_id, "CosmicBarb");
    assert_eq!(player.cards[0].display_name, "Electro Zap");
    // No database available, so the global raid modifier can't be looked up
    // and only Battle Drums (absent from this deck) would change the base window.
    assert_eq!(player.duration_seconds, 30.0);
}

#[tokio::test]
async fn repeated_start_attack_for_the_same_player_refreshes_started_at() {
    let state = Arc::new(AppState::new(None, 1, "test-key".to_string(), None));
    let raw: Value = serde_json::from_str(include_str!(
        "../../../exampleSocketdatajson/start_attack.json"
    ))
    .unwrap();

    handle_event(&state, "start_attack", raw.clone()).await.unwrap();
    let first_started_at = state
        .live_attacking_players
        .read()
        .await
        .get("933qd64")
        .unwrap()
        .started_at;

    // A later attack from the same player carries a newer `started_at`; the
    // frontend uses that (not an artificial counter) to tell "still the same
    // attack" apart from "a fresh one just began, restart the countdown".
    let mut later: Value = raw.clone();
    later["started_at"] = serde_json::json!("2026-08-25T15:00:00Z");

    handle_event(&state, "start_attack", later).await.unwrap();
    let second_started_at = state
        .live_attacking_players
        .read()
        .await
        .get("933qd64")
        .unwrap()
        .started_at;

    assert!(second_started_at > first_started_at);
    assert_eq!(state.live_attacking_players.read().await.len(), 1);
}

fn enemy8_boss_with_four_cursed_parts() -> (Boss, [BossPartName; 4]) {
    let sub_cycle: SubCycleEvent = serde_json::from_str(include_str!(
        "../../../exampleSocketdatajson/sub_cycle_example.json"
    ))
    .unwrap();
    // Enemy8 in this sample has exactly 4 cursed parts: LeftShoulder,
    // RightShoulder, LeftHand, RightHand (see ArmorArmUpperRight/Left and
    // ArmorHandRight/Left, all cursed:true).
    let (boss, _) =
        boss_from_raid_snapshot(&sub_cycle.raid, &sub_cycle.titan_target, "Enemy8", false)
            .unwrap();
    let cursed_parts = [
        BossPartName::LeftShoulder,
        BossPartName::RightShoulder,
        BossPartName::LeftHand,
        BossPartName::RightHand,
    ];
    for part_name in cursed_parts {
        assert_eq!(boss.part(part_name).part_state, PartState::Cursed);
    }
    (boss, cursed_parts)
}

/// Live "attack" payload where the armor on every part in `broken_parts`
/// drops to 0 (body survives) and nothing else on `boss` changes. Mirrors
/// TT2's convention of omitting the armor entry once it's destroyed (see
/// `attack_part_values`).
fn attack_breaking_parts(boss: &Boss, broken_parts: &[BossPartName]) -> AttackCurrentBoss {
    let mut live_parts = Vec::new();
    for part_name in BossPartName::all() {
        let (body_id, armor_id) = match part_name {
            BossPartName::Head => ("BodyHead", "ArmorHead"),
            BossPartName::Torso => ("BodyChestUpper", "ArmorChestUpper"),
            BossPartName::RightShoulder => ("BodyArmUpperLeft", "ArmorArmUpperLeft"),
            BossPartName::LeftShoulder => ("BodyArmUpperRight", "ArmorArmUpperRight"),
            BossPartName::RightHand => ("BodyHandLeft", "ArmorHandLeft"),
            BossPartName::LeftHand => ("BodyHandRight", "ArmorHandRight"),
            BossPartName::RightLeg => ("BodyLegUpperLeft", "ArmorLegUpperLeft"),
            BossPartName::LeftLeg => ("BodyLegUpperRight", "ArmorLegUpperRight"),
        };
        let part = boss.part(part_name);
        live_parts.push(AttackCurrentBossPart {
            part_id: body_id.to_string(),
            current_hp: part.current_health as f64,
        });
        let armor_hp = if broken_parts.contains(&part_name) {
            0.0
        } else {
            part.current_armor as f64
        };
        if armor_hp > 0.0 {
            live_parts.push(AttackCurrentBossPart {
                part_id: armor_id.to_string(),
                current_hp: armor_hp,
            });
        }
    }
    AttackCurrentBoss {
        enemy_id: "Enemy8".to_string(),
        current_hp: 0.0,
        parts: live_parts,
    }
}

#[test]
fn every_targeted_curse_part_breaking_at_once_triggers_full_refresh() {
    let (boss, cursed_parts) = enemy8_boss_with_four_cursed_parts();
    let targets = cursed_parts.to_vec();
    let live = attack_breaking_parts(&boss, &cursed_parts);

    let incoming = boss_from_attack_snapshot(&boss, &live).unwrap();
    for part_name in cursed_parts {
        assert_eq!(incoming.part(part_name).part_state, PartState::Body);
    }

    assert_eq!(
        classify_phase_refresh(&boss, &targets, &incoming),
        Some(PhaseRefresh::Full)
    );
}

#[test]
fn boss_from_attack_snapshot_applies_present_parts_even_when_one_is_missing() {
    let (boss, cursed_parts) = enemy8_boss_with_four_cursed_parts();
    let mut live = attack_breaking_parts(&boss, &cursed_parts);
    // Drop every entry for Head (both body and armor) as if TT2 omitted it
    // from this attack's payload entirely.
    live.parts
        .retain(|part| part.part_id != "BodyHead" && part.part_id != "ArmorHead");

    let incoming = boss_from_attack_snapshot(&boss, &live).unwrap();

    // The 4 cursed parts this attack actually reported still update...
    for part_name in cursed_parts {
        assert_eq!(incoming.part(part_name).part_state, PartState::Body);
    }
    // ...and Head, missing from the payload, is left exactly as it was
    // rather than the whole update being dropped.
    assert_eq!(incoming.head.current_armor, boss.head.current_armor);
    assert_eq!(incoming.head.current_health, boss.head.current_health);
    assert_eq!(incoming.head.part_state, boss.head.part_state);
}

#[test]
fn stale_reading_on_an_untargeted_part_does_not_suppress_a_targeted_curse_break() {
    let (boss, cursed_parts) = enemy8_boss_with_four_cursed_parts();
    let targets = cursed_parts.to_vec();
    let live = attack_breaking_parts(&boss, &cursed_parts);
    let incoming = boss_from_attack_snapshot(&boss, &live).unwrap();

    // Head (untargeted) is already dead going into this attack...
    let mut current = boss.clone();
    current.head.current_armor = 0;
    current.head.current_health = 0;
    current.head.sync_state_from_current_values();
    assert_eq!(current.head.part_state, PartState::Skeleton);

    // ...but this snapshot reports it alive again -- a stale/out-of-order
    // read from another clan member's overlapping attack. That must not
    // suppress the real phase change on the targeted curse parts.
    let mut incoming = incoming;
    incoming.head.current_health = 5;

    assert_eq!(
        classify_phase_refresh(&current, &targets, &incoming),
        Some(PhaseRefresh::Full)
    );
}

#[test]
fn stale_reading_on_an_untargeted_part_does_not_suppress_a_plain_armor_break() {
    // Same shape as the curse-break case above, but the targeted part that
    // breaks carries no curse at all -- confirms the fix isn't curse-specific.
    let (base_boss, _) = enemy8_boss_with_four_cursed_parts();
    let mut boss = base_boss.clone();
    boss.torso.part_state = PartState::Armor; // plain armor, not cursed
    let targets = vec![BossPartName::Torso];
    let live = attack_breaking_parts(&boss, &[BossPartName::Torso]);

    let incoming = boss_from_attack_snapshot(&boss, &live).unwrap();
    assert_eq!(incoming.torso.part_state, PartState::Body);

    let mut current = boss.clone();
    current.head.current_armor = 0;
    current.head.current_health = 0;
    current.head.sync_state_from_current_values();

    let mut incoming = incoming;
    incoming.head.current_health = 5; // stale: untargeted Head looks alive again

    assert_eq!(
        classify_phase_refresh(&current, &targets, &incoming),
        Some(PhaseRefresh::Full)
    );
}

#[test]
fn aggregate_card_components_folds_split_hits_and_skips_pure_taps() {
    let components = vec![
        // A pure tap (no card) contributes to tap_damage elsewhere but never
        // appears in the per-card breakdown.
        AttackComponent {
            titan_index: 1,
            card_id: None,
            card_level: None,
            total_damage: 500,
        },
        AttackComponent {
            titan_index: 1,
            card_id: Some("SkullBash".to_string()),
            card_level: Some(52),
            total_damage: 1000,
        },
        AttackComponent {
            titan_index: 1,
            card_id: Some("MirrorForce".to_string()),
            card_level: Some(50),
            total_damage: 2000,
        },
        // SkullBash lands a second time as a separate Body/Armor-split
        // component mid-attack -- must fold into the same card total, not a
        // second entry.
        AttackComponent {
            titan_index: 1,
            card_id: Some("SkullBash".to_string()),
            card_level: Some(52),
            total_damage: 300,
        },
    ];

    let cards = aggregate_card_components(&components);

    assert_eq!(cards.len(), 2, "pure tap must not become a card entry");
    assert_eq!(cards[0].card_id, "SkullBash");
    assert_eq!(cards[0].card_level, Some(52));
    assert_eq!(cards[0].damage, 1300, "split hits for the same card must sum");
    assert_eq!(cards[1].card_id, "MirrorForce");
    assert_eq!(cards[1].damage, 2000);
}

#[test]
fn boss_name_maps_known_enemy_ids_and_validates_matching_name() {
    assert_eq!(boss_name("Enemy1", "").unwrap(), BossName::Lojak);
    assert_eq!(boss_name("Enemy3", "").unwrap(), BossName::Jukk);
    assert_eq!(boss_name("Enemy8", "Priker").unwrap(), BossName::Priker);
    assert!(
        boss_name("Enemy8", "Lojak").is_err(),
        "a mismatched enemy_name must be rejected"
    );
    assert!(
        boss_name("Enemy99", "").is_err(),
        "an unrecognized enemy_id must be rejected"
    );
}

#[test]
fn global_modifier_accepts_both_known_spellings_and_rejects_unknown() {
    assert_eq!(
        global_modifier("BurstDamage").unwrap(),
        GlobalRaidModifier::BurstDamage
    );
    // TT2 uses inconsistent spellings across events for the same modifier --
    // both must map to the same variant.
    assert_eq!(
        global_modifier("AfflictedChance").unwrap(),
        GlobalRaidModifier::AfflictionChance
    );
    assert_eq!(
        global_modifier("AfflictionChance").unwrap(),
        GlobalRaidModifier::AfflictionChance
    );
    assert_eq!(
        global_modifier("AfflictedDamage").unwrap(),
        GlobalRaidModifier::AfflictionDamage
    );
    assert_eq!(
        global_modifier("AfflictedDuration").unwrap(),
        GlobalRaidModifier::AfflictionDuration
    );
    assert!(global_modifier("NotARealModifier").is_err());
}

#[test]
fn curse_type_accepts_both_known_spellings_and_rejects_unknown() {
    assert_eq!(
        curse_type("BodyDamagePerCurse").unwrap(),
        CurseType::BodyDamage
    );
    assert_eq!(
        curse_type("BurstDamagePerCurse").unwrap(),
        CurseType::BurstDamage
    );
    assert_eq!(
        curse_type("AfflictedDamagePerCurse").unwrap(),
        CurseType::AfflictionDamage
    );
    assert_eq!(
        curse_type("AfflictionDamagePerCurse").unwrap(),
        CurseType::AfflictionDamage
    );
    assert!(curse_type("NotARealCurse").is_err());
}

#[test]
fn next_reset_boundary_rolls_forward_when_exactly_on_a_boundary() {
    let start = Utc::now();
    assert_eq!(
        next_reset_boundary(start, start),
        start + chrono::Duration::hours(12)
    );

    // Landing exactly on a boundary must roll forward to the *next* one, not
    // report the boundary already reached.
    let one_boundary = start + chrono::Duration::hours(12);
    assert_eq!(
        next_reset_boundary(start, one_boundary),
        start + chrono::Duration::hours(24)
    );

    let just_before = one_boundary - chrono::Duration::seconds(1);
    assert_eq!(next_reset_boundary(start, just_before), one_boundary);
}

#[test]
fn bonus_value_defaults_to_zero_for_a_missing_id() {
    let bonuses = vec![RaidBonus {
        id: "MirrorForceBoost".to_string(),
        value: 0.35,
    }];
    assert_eq!(bonus_value(&bonuses, "MirrorForceBoost"), 0.35);
    assert_eq!(bonus_value(&bonuses, "SomethingElse"), 0.0);
    assert_eq!(bonus_value(&[], "MirrorForceBoost"), 0.0);
}

fn loaded_enemy8_boss(
    attackable_parts: Vec<BossPartName>,
    source_raid_id: Option<i64>,
) -> boss_repo::LoadedBoss {
    let (mut boss, _) = enemy8_boss_with_four_cursed_parts();
    // Exercise every branch live_boss_from_persisted has to handle: an Armor
    // part, a Skeleton part, and the Cursed parts the fixture already gives us.
    boss.head.part_state = PartState::Armor;
    boss.head.current_armor = 111;
    boss.head.current_health = 222;
    boss.torso.part_state = PartState::Skeleton;
    boss.torso.current_armor = 0;
    boss.torso.current_health = 0;

    boss_repo::LoadedBoss {
        version: 7,
        boss,
        attackable_parts,
        source_raid_id,
        source_titan_index: Some(2),
        source_enemy_id: Some("Enemy8".to_string()),
        created_at: Utc::now(),
        updated_at: Utc::now(),
    }
}

#[test]
fn live_boss_from_persisted_returns_none_without_a_source_raid_id() {
    let loaded = loaded_enemy8_boss(vec![BossPartName::Head], None);
    assert!(live_boss_from_persisted(&loaded, "clan123".to_string(), 5).is_none());
}

#[test]
fn live_boss_from_persisted_reconstructs_a_live_view_from_the_stored_boss() {
    let loaded = loaded_enemy8_boss(
        vec![BossPartName::Head, BossPartName::Torso],
        Some(42),
    );
    let expected_current_hp_total = BossPartName::all()
        .iter()
        .map(|part_name| loaded.boss.part(*part_name).current_health as f64)
        .sum::<f64>();
    let expected_updated_at = loaded.updated_at;

    let view = live_boss_from_persisted(&loaded, "clan123".to_string(), 5)
        .expect("a boss with a source_raid_id must produce a view");

    assert_eq!(view.clan_code, "clan123");
    assert_eq!(view.raid_id, 42);
    assert_eq!(view.titan_index, 2);
    assert_eq!(view.cycle, 5);
    assert_eq!(view.received_at, expected_updated_at);
    assert_eq!(view.boss_data["enemy_id"], "Enemy8");
    assert_eq!(view.boss_data["current_hp"], expected_current_hp_total);

    let parts = view.boss_data["parts"].as_array().unwrap();
    let has_part_id = |id: &str| parts.iter().any(|part| part["part_id"] == id);
    // Head is Armor -- both its body and armor entries are present.
    assert!(has_part_id("BodyHead"));
    assert!(has_part_id("ArmorHead"));
    // Torso is Skeleton -- TT2's own convention omits a destroyed armor layer.
    assert!(has_part_id("BodyChestUpper"));
    assert!(!has_part_id("ArmorChestUpper"));

    let display_parts = view.display_parts.expect("display_parts must be populated");
    assert_eq!(display_parts.len(), 8);
    let head = display_parts
        .iter()
        .find(|part| part.part_name == BossPartName::Head)
        .unwrap();
    assert_eq!(head.part_state, PartState::Armor);
    assert_eq!(head.current_hp, 111);
    assert_eq!(head.max_hp, loaded.boss.head.max_armor);
    assert!(head.is_targeted);

    let torso = display_parts
        .iter()
        .find(|part| part.part_name == BossPartName::Torso)
        .unwrap();
    assert_eq!(torso.part_state, PartState::Skeleton);
    assert_eq!(torso.current_hp, 0);
    // A Skeleton part reports max_hp from max_health, not max_armor.
    assert_eq!(torso.max_hp, loaded.boss.torso.max_health);
    assert!(torso.is_targeted);

    let left_shoulder = display_parts
        .iter()
        .find(|part| part.part_name == BossPartName::LeftShoulder)
        .unwrap();
    assert_eq!(left_shoulder.part_state, PartState::Cursed);
    assert!(
        !left_shoulder.is_targeted,
        "a cursed part outside attackable_parts must not be reported as targeted"
    );
}

#[tokio::test]
async fn store_sub_start_cycle_state_rejects_invalid_morale_without_touching_the_database() {
    let state = Arc::new(AppState::new(None, 1, "test-key".to_string(), None));

    // If validation didn't run first, the very next thing this function does
    // is `state.db()?`, which would fail with DatabaseUnavailable instead --
    // asserting BadRequest specifically proves validation short-circuits before that.
    let negative = store_sub_start_cycle_state(&state, "clan", 1, Utc::now(), -1.0).await;
    assert!(matches!(negative, Err(AppError::BadRequest(_))));

    let not_finite = store_sub_start_cycle_state(&state, "clan", 1, Utc::now(), f64::NAN).await;
    assert!(matches!(not_finite, Err(AppError::BadRequest(_))));
}

#[tokio::test]
async fn store_cycle_state_rejects_invalid_boost_values_without_touching_the_database() {
    let state = Arc::new(AppState::new(None, 1, "test-key".to_string(), None));
    let now = Utc::now();

    let negative_morale =
        store_cycle_state(&state, "clan", 1, None, now, now, -0.1, 0.0, 0.0).await;
    assert!(matches!(negative_morale, Err(AppError::BadRequest(_))));

    let negative_team_tactics =
        store_cycle_state(&state, "clan", 1, None, now, now, 0.0, -0.1, 0.0).await;
    assert!(matches!(negative_team_tactics, Err(AppError::BadRequest(_))));

    let negative_mirror_force =
        store_cycle_state(&state, "clan", 1, None, now, now, 0.0, 0.0, -0.1).await;
    assert!(matches!(negative_mirror_force, Err(AppError::BadRequest(_))));

    let not_finite = store_cycle_state(&state, "clan", 1, None, now, now, f64::INFINITY, 0.0, 0.0)
        .await;
    assert!(matches!(not_finite, Err(AppError::BadRequest(_))));
}

#[tokio::test]
async fn handle_event_ignores_unrecognized_events_without_parsing_the_payload() {
    let state = Arc::new(AppState::new(None, 1, "test-key".to_string(), None));
    // Not shaped like any real event -- if the catch-all tried to parse it
    // against a known event type, this would fail instead of returning Ok.
    let bogus = serde_json::json!({"not": "a recognized shape"});
    assert!(
        handle_event(&state, "some_unknown_event", bogus)
            .await
            .is_ok()
    );
}

#[tokio::test]
async fn handle_cycle_reset_rejects_invalid_morale_before_touching_the_database() {
    let state = Arc::new(AppState::new(None, 1, "test-key".to_string(), None));
    let mut raw: Value = serde_json::from_str(include_str!(
        "../../../exampleSocketdatajson/cycle_reset_example.json"
    ))
    .unwrap();
    raw["morale"]["bonus_amount"] = serde_json::json!(-1.0);

    // No database is configured on this state at all, so if validation
    // didn't run before the first DB call, this would fail with
    // DatabaseUnavailable rather than BadRequest.
    let result = handle_event(&state, "cycle_reset", raw).await;
    assert!(matches!(result, Err(AppError::BadRequest(_))));
}
