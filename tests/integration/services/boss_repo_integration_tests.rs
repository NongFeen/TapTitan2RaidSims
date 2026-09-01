//! DB-backed coverage for `boss_repo` -- there is no pure logic here to
//! unit-test; `load`/`load_for_update`/`store` are each a thin wrapper
//! directly around one SQL statement against the singleton `current_boss`
//! row, so everything worth checking requires a real Postgres.
//!
//! CRITICAL: same warning as
//! `raid_event_service_integration_tests.rs` -- never run this file with a
//! bare `cargo test`. `#[sqlx::test]` falls back to `.env`'s `DATABASE_URL`
//! (the real dev Postgres) when the shell doesn't already have one set.
//! Always use `scripts/test-integration.ps1`.

use super::*;

fn sample_boss(head_current_armor: u64) -> Boss {
    let part = |name: &str, current_armor: u64| {
        serde_json::json!({
            "part_name": name,
            "part_state": "Armor",
            "max_armor": 2_000_000,
            "max_health": 1_000_000,
            "current_armor": current_armor,
            "current_health": 1_000_000,
        })
    };
    serde_json::from_value(serde_json::json!({
        "boss_name": "Lojak",
        "global_raid_modifier": "BurstDamage",
        "global_raid_modifier_amount": 0.3,
        "curse_type": "BodyDamage",
        "curse_damage_per_curse": 0.06,
        "recommend_1_to_2_part_patterns_only": false,
        "head": part("Head", head_current_armor),
        "torso": part("Torso", 500_000),
        "left_shoulder": part("LeftShoulder", 500_000),
        "right_shoulder": part("RightShoulder", 500_000),
        "left_hand": part("LeftHand", 500_000),
        "right_hand": part("RightHand", 500_000),
        "left_leg": part("LeftLeg", 500_000),
        "right_leg": part("RightLeg", 500_000),
        "damage_results": [],
    }))
    .unwrap()
}

fn sample_write<'a>(
    boss: &'a Boss,
    attackable_parts: Option<&'a [BossPartName]>,
    bump_version: bool,
) -> BossWrite<'a> {
    BossWrite {
        boss,
        attackable_parts,
        source_raid_id: Some(42),
        source_titan_index: Some(0),
        source_enemy_id: Some("Enemy1"),
        bump_version,
    }
}

#[sqlx::test]
async fn load_and_load_for_update_return_none_before_any_row_exists(pool: sqlx::PgPool) {
    assert!(load(&pool).await.unwrap().is_none());

    let mut tx = pool.begin().await.unwrap();
    assert!(load_for_update(&mut tx).await.unwrap().is_none());
    tx.commit().await.unwrap();
}

#[sqlx::test]
async fn store_round_trips_every_field(pool: sqlx::PgPool) {
    let boss = sample_boss(1_500_000);
    let mut tx = pool.begin().await.unwrap();
    store(
        &mut tx,
        sample_write(&boss, Some(&[BossPartName::Head, BossPartName::Torso]), true),
    )
    .await
    .unwrap();
    tx.commit().await.unwrap();

    let loaded = load(&pool).await.unwrap().expect("row should exist after store");
    assert_eq!(loaded.boss.boss_name, BossName::Lojak);
    assert_eq!(
        loaded.boss.global_raid_modifier,
        GlobalRaidModifier::BurstDamage
    );
    assert_eq!(loaded.boss.global_raid_modifier_amount, Some(0.3));
    assert_eq!(loaded.boss.curse_type, CurseType::BodyDamage);
    assert!((loaded.boss.curse_damage_per_curse - 0.06).abs() < 1e-9);
    assert_eq!(loaded.boss.head.part_state, PartState::Armor);
    assert_eq!(loaded.boss.head.current_armor, 1_500_000);
    assert_eq!(loaded.boss.head.max_armor, 2_000_000);
    assert_eq!(loaded.boss.torso.current_armor, 500_000);
    assert_eq!(loaded.source_raid_id, Some(42));
    assert_eq!(loaded.source_titan_index, Some(0));
    assert_eq!(loaded.source_enemy_id, Some("Enemy1".to_string()));
    assert_eq!(
        loaded.attackable_parts,
        vec![BossPartName::Head, BossPartName::Torso]
    );

    // A `None` global_raid_modifier_amount and source fields must round-trip
    // as `None`, not e.g. silently become 0 or an empty string.
    let mut no_modifier_boss = sample_boss(1_500_000);
    no_modifier_boss.global_raid_modifier_amount = None;
    let mut tx = pool.begin().await.unwrap();
    store(
        &mut tx,
        BossWrite {
            boss: &no_modifier_boss,
            attackable_parts: Some(&[]),
            source_raid_id: None,
            source_titan_index: None,
            source_enemy_id: None,
            bump_version: true,
        },
    )
    .await
    .unwrap();
    tx.commit().await.unwrap();
    let loaded = load(&pool).await.unwrap().unwrap();
    assert_eq!(loaded.boss.global_raid_modifier_amount, None);
    assert_eq!(loaded.source_raid_id, None);
    assert_eq!(loaded.source_titan_index, None);
    assert_eq!(loaded.source_enemy_id, None);
    assert!(loaded.attackable_parts.is_empty());
}

#[sqlx::test]
async fn store_bumps_version_only_when_asked(pool: sqlx::PgPool) {
    let boss = sample_boss(1_500_000);

    let mut tx = pool.begin().await.unwrap();
    let v1 = store(&mut tx, sample_write(&boss, Some(&[BossPartName::Head]), true))
        .await
        .unwrap();
    tx.commit().await.unwrap();
    assert_eq!(v1, 1, "the very first store should start at version 1");

    // bump_version: false -- the row still updates (HP sync), but the
    // version must stay exactly where it was.
    let mut tx = pool.begin().await.unwrap();
    let v2 = store(&mut tx, sample_write(&boss, None, false)).await.unwrap();
    tx.commit().await.unwrap();
    assert_eq!(v2, v1);

    // bump_version: true again -- exactly +1, not reset or skipped.
    let mut tx = pool.begin().await.unwrap();
    let v3 = store(&mut tx, sample_write(&boss, None, true)).await.unwrap();
    tx.commit().await.unwrap();
    assert_eq!(v3, v1 + 1);

    let mut tx = pool.begin().await.unwrap();
    let v4 = store(&mut tx, sample_write(&boss, None, true)).await.unwrap();
    tx.commit().await.unwrap();
    assert_eq!(v4, v1 + 2, "each bump_version:true store increments by exactly 1");
}

#[sqlx::test]
async fn store_only_overwrites_attackable_parts_when_provided(pool: sqlx::PgPool) {
    let boss = sample_boss(1_500_000);

    let mut tx = pool.begin().await.unwrap();
    store(
        &mut tx,
        sample_write(&boss, Some(&[BossPartName::Head, BossPartName::Torso]), true),
    )
    .await
    .unwrap();
    tx.commit().await.unwrap();
    let after_first = load(&pool).await.unwrap().unwrap();
    assert_eq!(
        after_first.attackable_parts,
        vec![BossPartName::Head, BossPartName::Torso]
    );

    // A later HP-only sync (attackable_parts: None) must leave targeting
    // completely untouched, even though every other column still updates.
    let mut hp_only_boss = sample_boss(999_999);
    hp_only_boss.head.current_armor = 999_999;
    let mut tx = pool.begin().await.unwrap();
    store(&mut tx, sample_write(&hp_only_boss, None, false))
        .await
        .unwrap();
    tx.commit().await.unwrap();
    let after_hp_sync = load(&pool).await.unwrap().unwrap();
    assert_eq!(
        after_hp_sync.attackable_parts,
        vec![BossPartName::Head, BossPartName::Torso],
        "attackable_parts: None must leave existing targeting alone"
    );
    assert_eq!(after_hp_sync.boss.head.current_armor, 999_999);

    // Now explicitly narrow targeting down to just LeftHand -- this must
    // actually change it, proving None vs Some behave differently rather
    // than targeting just never being touched at all.
    let mut tx = pool.begin().await.unwrap();
    store(&mut tx, sample_write(&boss, Some(&[BossPartName::LeftHand]), false))
        .await
        .unwrap();
    tx.commit().await.unwrap();
    let after_retarget = load(&pool).await.unwrap().unwrap();
    assert_eq!(after_retarget.attackable_parts, vec![BossPartName::LeftHand]);
}

#[sqlx::test]
async fn load_for_update_serializes_concurrent_writers(pool: sqlx::PgPool) {
    let boss = sample_boss(1_500_000);
    let mut tx = pool.begin().await.unwrap();
    store(&mut tx, sample_write(&boss, Some(&[BossPartName::Head]), true))
        .await
        .unwrap();
    tx.commit().await.unwrap();

    let (lock_acquired_tx, lock_acquired_rx) = tokio::sync::oneshot::channel();
    let holder_pool = pool.clone();
    let holder = tokio::spawn(async move {
        let mut tx = holder_pool.begin().await.unwrap();
        load_for_update(&mut tx).await.unwrap();
        lock_acquired_tx.send(()).unwrap();
        // Hold the row lock well past the waiter's attempt below.
        tokio::time::sleep(std::time::Duration::from_millis(400)).await;
        tx.commit().await.unwrap();
    });

    lock_acquired_rx.await.unwrap();
    let waiter_start = tokio::time::Instant::now();
    let mut waiter_tx = pool.begin().await.unwrap();
    load_for_update(&mut waiter_tx).await.unwrap();
    let waited_for = waiter_start.elapsed();
    waiter_tx.commit().await.unwrap();
    holder.await.unwrap();

    assert!(
        waited_for >= std::time::Duration::from_millis(300),
        "a second load_for_update must block until the first transaction \
         commits, not read past the still-held row lock (waited only {waited_for:?})"
    );
}
