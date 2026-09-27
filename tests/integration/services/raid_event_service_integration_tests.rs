//! DB-backed coverage for `raid_event_service`'s actual read/write behavior
//! -- the parts `tests/unit/services/raid_event_service_tests.rs` can't
//! reach, since those only exercise pure functions and early-return
//! validation. Each `#[sqlx::test]` gets its own disposable Postgres
//! database (migrations applied automatically from `../../../migrations`).
//!
//! CRITICAL: `#[sqlx::test]` reads `DATABASE_URL` from the environment, and
//! -- independently of this app's own startup code -- falls back to loading
//! it from `.env` (via `dotenvy`) whenever it isn't already set in the
//! shell. `.env` points at the real dev Postgres. Never run this file with
//! a bare `cargo test`; always set `DATABASE_URL` explicitly first, e.g.
//! `scripts/test-integration.ps1`, which points it at a disposable
//! container instead (see that script for how to (re)create one). Forgetting
//! this doesn't corrupt real data -- `sqlx::test` still only creates and
//! drops its own throwaway database, never writing into `.env`'s database
//! directly -- but it does mean briefly issuing CREATE DATABASE/DROP
//! DATABASE against whatever server `.env` names, without asking.

use super::*;
use std::collections::HashMap;

use crate::models::player_data::PlayerData;
use crate::services::player_stats_repo;
use crate::services::taptitan::player_service::clean_data;

const HEAD_ARMOR: u64 = 1_000_000;
const HEAD_MAX_ARMOR: u64 = 2_000_000;
const OTHER_ARMOR: u64 = 500_000;
const OTHER_MAX: u64 = 1_000_000;
const HEALTH: u64 = 1_000_000;

/// A titan with every part at a known, stable Armor state except Head,
/// whose current armor is the one value tests actually vary.
fn simple_titan(enemy_id: &str, enemy_name: &str, head_current_armor: u64) -> RaidTitan {
    let mut parts = Vec::with_capacity(16);
    for part_name in BossPartName::all() {
        let (body_id, armor_id) = part_ids(part_name);
        let (current_armor, max_armor) = if part_name == BossPartName::Head {
            (head_current_armor, HEAD_MAX_ARMOR)
        } else {
            (OTHER_ARMOR, OTHER_MAX)
        };
        parts.push(RaidTitanPart {
            part_id: body_id.to_string(),
            current_hp: HEALTH as f64,
            total_hp: HEALTH as f64,
            cursed: false,
        });
        parts.push(RaidTitanPart {
            part_id: armor_id.to_string(),
            current_hp: current_armor as f64,
            total_hp: max_armor as f64,
            cursed: false,
        });
    }
    RaidTitan {
        enemy_id: enemy_id.to_string(),
        enemy_name: enemy_name.to_string(),
        parts,
        cursed_debuffs: vec![],
        extra: HashMap::new(),
    }
}

fn simple_raid(titan: RaidTitan) -> RaidSnapshot {
    RaidSnapshot {
        spawn_sequence: vec![titan.enemy_name.clone()],
        titans: vec![titan],
        area_buffs: vec![],
        extra: HashMap::new(),
    }
}

/// A live "attack" snapshot for the same titan shape `simple_titan` builds,
/// with Head's armor/health set to whatever this attack reports -- every
/// other part unchanged, all 16 entries present (armor omitted once broken),
/// matching TT2's own convention.
fn attack_snapshot(enemy_id: &str, head_current_armor: u64, head_current_health: u64) -> AttackCurrentBoss {
    let mut parts = Vec::with_capacity(16);
    for part_name in BossPartName::all() {
        let (body_id, armor_id) = part_ids(part_name);
        let (current_armor, current_health) = if part_name == BossPartName::Head {
            (head_current_armor, head_current_health)
        } else {
            (OTHER_ARMOR, HEALTH)
        };
        parts.push(AttackCurrentBossPart {
            part_id: body_id.to_string(),
            current_hp: current_health as f64,
        });
        if current_armor > 0 {
            parts.push(AttackCurrentBossPart {
                part_id: armor_id.to_string(),
                current_hp: current_armor as f64,
            });
        }
    }
    AttackCurrentBoss {
        enemy_id: enemy_id.to_string(),
        current_hp: 0.0,
        parts,
    }
}

fn attack_event(
    raid_id: i64,
    clan_code: &str,
    player_code: &str,
    titan_index: i32,
    current: AttackCurrentBoss,
    tap_damage: u64,
) -> AttackEvent {
    AttackEvent {
        attack_log: AttackLog {
            attack_datetime: Utc::now(),
            cards_damage: vec![AttackCardDamage {
                titan_index,
                card_id: None,
                damage_log: vec![AttackPartDamage {
                    value: tap_damage as f64,
                }],
            }],
            cards_level: vec![],
        },
        clan_code: clan_code.to_string(),
        raid_id,
        player: AttackPlayer {
            player_code: player_code.to_string(),
            name: "Test Player".to_string(),
        },
        raid_state: AttackRaidState {
            current,
            titan_index,
        },
        cycle: 1,
    }
}

/// Registers a player with real, valid stats (reusing the sim-to-real
/// fixture's player export -- `player_stats` has ~100 NOT NULL columns with
/// no defaults, so a real deserialized value is far more reliable than
/// hand-building one) and `auto_sims` on, so `queue_auto_simulations` can
/// find and queue a job for them.
async fn insert_auto_sims_player(pool: &sqlx::PgPool, player_id: &str) {
    sqlx::query("INSERT INTO players (player_id, display_name, auto_sims) VALUES ($1, $1, TRUE)")
        .bind(player_id)
        .execute(pool)
        .await
        .unwrap();

    #[derive(serde::Deserialize)]
    struct RawFixture {
        player_raw_data: PlayerData,
    }
    let fixture: RawFixture = serde_json::from_str(include_str!(
        "../../fixtures/sim_to_real/player_boss_sample.json"
    ))
    .unwrap();
    let player_raid_data = clean_data(&fixture.player_raw_data);

    let mut tx = pool.begin().await.unwrap();
    player_stats_repo::store(&mut tx, player_id, &player_raid_data)
        .await
        .unwrap();
    tx.commit().await.unwrap();
}

#[sqlx::test]
async fn migrations_apply_and_the_database_starts_empty(pool: sqlx::PgPool) {
    let raid_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM raid_current_state")
        .fetch_one(&pool)
        .await
        .expect("raid_current_state table should exist after migrations");
    assert_eq!(raid_count, 0);
}

#[sqlx::test]
async fn handle_sub_start_creates_raid_cycle_and_boss_rows_for_a_new_raid(pool: sqlx::PgPool) {
    let state = Arc::new(AppState::new(Some(pool.clone()), 1, "test-key".to_string(), true, None));
    let raid_id = 9001;
    let titan = simple_titan("Enemy1", "Lojak", HEAD_ARMOR);
    let raid = simple_raid(titan);
    let event = SubStartEvent {
        clan_code: "clanA".to_string(),
        raid_id,
        morale: Some(RaidMorale { bonus_amount: 0.4 }),
        raid,
        start_at: Some(Utc::now()),
        titan_target: vec![],
    };

    handle_sub_start(&state, event, serde_json::json!({}), true)
        .await
        .expect("a brand new raid's sub_start should succeed");

    let (resulting_titan_index, current_enemy_id): (Option<i32>, Option<String>) = sqlx::query_as(
        "SELECT resulting_titan_index, current_enemy_id FROM raid_current_state WHERE raid_id=$1",
    )
    .bind(raid_id)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(resulting_titan_index, Some(0));
    assert_eq!(current_enemy_id, Some("Enemy1".to_string()));

    let morale: f64 = sqlx::query_scalar("SELECT morale FROM raid_cycle_state WHERE raid_id=$1")
        .bind(raid_id)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert!((morale - 0.4).abs() < 1e-9);

    let loaded = boss_repo::load(&pool)
        .await
        .unwrap()
        .expect("sub_start should establish the sims boss");
    assert_eq!(loaded.boss.boss_name, BossName::Lojak);
    assert_eq!(loaded.source_raid_id, Some(raid_id));
    assert_eq!(loaded.boss.head.current_armor, HEAD_ARMOR);
    // No real target selection was reported -- every part defaults to attackable.
    assert_eq!(loaded.attackable_parts.len(), 8);
}

#[sqlx::test]
async fn repeated_sub_start_for_the_same_raid_only_refreshes_raid_data(pool: sqlx::PgPool) {
    let state = Arc::new(AppState::new(Some(pool.clone()), 1, "test-key".to_string(), true, None));
    let raid_id = 9002;
    let first_event = SubStartEvent {
        clan_code: "clanA".to_string(),
        raid_id,
        morale: Some(RaidMorale { bonus_amount: 0.1 }),
        raid: simple_raid(simple_titan("Enemy1", "Lojak", HEAD_ARMOR)),
        start_at: Some(Utc::now()),
        titan_target: vec![],
    };
    handle_sub_start(&state, first_event, serde_json::json!({}), true)
        .await
        .unwrap();
    let first_version = boss_repo::load(&pool).await.unwrap().unwrap().version;

    // A later sub_start for the SAME raid -- carries a real (but unreliable,
    // per the doc comment on handle_sub_start) titan_target selection and
    // different current_armor. Neither should touch the boss row at all.
    let second_event = SubStartEvent {
        clan_code: "clanA".to_string(),
        raid_id,
        morale: Some(RaidMorale { bonus_amount: 0.5 }),
        raid: simple_raid(simple_titan("Enemy1", "Lojak", 42)),
        start_at: Some(Utc::now()),
        titan_target: vec![TitanTarget {
            enemy_id: "Enemy1".to_string(),
            state: vec![TitanTargetPart {
                id: "Head".to_string(),
                state: "2".to_string(),
            }],
        }],
    };
    handle_sub_start(&state, second_event, serde_json::json!({}), true)
        .await
        .expect("a later sub_start for an already-established raid should succeed");

    let loaded = boss_repo::load(&pool).await.unwrap().unwrap();
    assert_eq!(
        loaded.version, first_version,
        "the boss row must be completely untouched by a repeat sub_start"
    );
    assert_eq!(
        loaded.boss.head.current_armor, HEAD_ARMOR,
        "the stale current_armor=42 from the second sub_start must never be applied"
    );
    assert_eq!(
        loaded.attackable_parts.len(),
        8,
        "sub_start's titan_target must never narrow targeting -- that's sub_cycle's job"
    );

    // raid_data itself, however, is refreshed to the newer snapshot.
    let raid_data: serde_json::Value =
        sqlx::query_scalar("SELECT raid_data FROM raid_current_state WHERE raid_id=$1")
            .bind(raid_id)
            .fetch_one(&pool)
            .await
            .unwrap();
    let refreshed: RaidSnapshot = serde_json::from_value(raid_data).unwrap();
    let head_armor = refreshed.titans[0]
        .parts
        .iter()
        .find(|part| part.part_id == "ArmorHead")
        .unwrap();
    assert_eq!(head_armor.current_hp, 42.0);
}

#[sqlx::test]
async fn handle_attack_updates_hp_without_bumping_version_when_nothing_changed(pool: sqlx::PgPool) {
    let state = Arc::new(AppState::new(Some(pool.clone()), 1, "test-key".to_string(), true, None));
    let raid_id = 9003;
    handle_sub_start(
        &state,
        SubStartEvent {
            clan_code: "clanA".to_string(),
            raid_id,
            morale: Some(RaidMorale { bonus_amount: 0.0 }),
            raid: simple_raid(simple_titan("Enemy1", "Lojak", HEAD_ARMOR)),
            start_at: Some(Utc::now()),
            titan_target: vec![],
        },
        serde_json::json!({}),
        true,
    )
    .await
    .unwrap();
    insert_auto_sims_player(&pool, "player1").await;
    let version_before = boss_repo::load(&pool).await.unwrap().unwrap().version;

    let reduced_armor = HEAD_ARMOR - 100_000;
    let attack = attack_event(
        raid_id,
        "clanA",
        "player1",
        0,
        attack_snapshot("Enemy1", reduced_armor, HEALTH),
        4610,
    );
    handle_attack(&state, attack).await.unwrap();

    let loaded = boss_repo::load(&pool).await.unwrap().unwrap();
    assert_eq!(loaded.boss.head.current_armor, reduced_armor);
    assert_eq!(loaded.boss.head.part_state, PartState::Armor);
    assert_eq!(
        loaded.version, version_before,
        "HP-only changes with no phase transition must not bump the simulation version"
    );

    let logged_count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM raid_attack_logs WHERE raid_id=$1 AND player_id=$2")
            .bind(raid_id)
            .bind("player1")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(logged_count, 1, "a known player's attack must be logged");
}

#[sqlx::test]
async fn handle_attack_triggers_full_refresh_and_queues_auto_simulations_when_a_target_breaks(
    pool: sqlx::PgPool,
) {
    let state = Arc::new(AppState::new(Some(pool.clone()), 1, "test-key".to_string(), true, None));
    let raid_id = 9004;
    let raid = simple_raid(simple_titan("Enemy1", "Lojak", 1));
    // Head starts with only a sliver of armor left. sub_start's own
    // titan_target is unreliable and is never used to set attackable_parts
    // (it always defaults to every part) -- narrowing the target selection
    // down to Head alone is sub_cycle's job. classify_phase_refresh only
    // calls this a full phase change once every *targeted* armor part is
    // gone, not just any one part, so the selection has to actually be
    // narrowed for this attack to qualify.
    handle_sub_start(
        &state,
        SubStartEvent {
            clan_code: "clanA".to_string(),
            raid_id,
            morale: Some(RaidMorale { bonus_amount: 0.0 }),
            raid: raid.clone(),
            start_at: Some(Utc::now()),
            titan_target: vec![],
        },
        serde_json::json!({}),
        true,
    )
    .await
    .unwrap();
    handle_sub_cycle(
        &state,
        SubCycleEvent {
            clan_code: "clanA".to_string(),
            raid_id,
            next_reset_at: Utc::now(),
            card_bonuses: vec![],
            morale: RaidMorale { bonus_amount: 0.0 },
            raid_started_at: Utc::now(),
            raid,
            titan_target: vec![TitanTarget {
                enemy_id: "Enemy1".to_string(),
                state: vec![TitanTargetPart {
                    id: "Head".to_string(),
                    state: "2".to_string(),
                }],
            }],
        },
        serde_json::json!({}),
    )
    .await
    .unwrap();
    insert_auto_sims_player(&pool, "player1").await;
    let version_before = boss_repo::load(&pool).await.unwrap().unwrap().version;
    assert_eq!(
        boss_repo::load(&pool).await.unwrap().unwrap().attackable_parts,
        vec![BossPartName::Head]
    );

    // This attack breaks Head's armor entirely -- the exact "targeted part
    // breaks" scenario the curse/armor-break auto-sim bug fixed this session
    // was about, now proven through a real DB round trip.
    let attack = attack_event(
        raid_id,
        "clanA",
        "player1",
        0,
        attack_snapshot("Enemy1", 0, HEALTH),
        4610,
    );
    handle_attack(&state, attack).await.unwrap();

    let loaded = boss_repo::load(&pool).await.unwrap().unwrap();
    assert_eq!(loaded.boss.head.part_state, PartState::Body);
    assert!(
        loaded.version > version_before,
        "a phase transition on a targeted part must bump the simulation version"
    );

    let queued_jobs: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM simulation_jobs WHERE player_id=$1")
            .bind("player1")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(
        queued_jobs, 1,
        "an auto_sims player must get a simulation job queued when a target breaks"
    );
}

#[sqlx::test]
async fn handle_sub_cycle_is_trusted_for_hp_and_targets(pool: sqlx::PgPool) {
    let state = Arc::new(AppState::new(Some(pool.clone()), 1, "test-key".to_string(), true, None));
    let raid_id = 9005;
    let raid = simple_raid(simple_titan("Enemy1", "Lojak", HEAD_ARMOR));
    handle_sub_start(
        &state,
        SubStartEvent {
            clan_code: "clanA".to_string(),
            raid_id,
            morale: Some(RaidMorale { bonus_amount: 0.0 }),
            raid,
            start_at: Some(Utc::now()),
            titan_target: vec![],
        },
        serde_json::json!({}),
        true,
    )
    .await
    .unwrap();

    handle_attack(
        &state,
        attack_event(
            raid_id,
            "clanA",
            "player1",
            0,
            attack_snapshot("Enemy1", HEAD_ARMOR - 300_000, HEALTH),
            4610,
        ),
    )
    .await
    .unwrap();

    // sub_cycle is TT2's own snapshot of the live raid: its HP wins over
    // whatever the last attack reported, and it sets targeting.
    let sub_cycle_armor = HEAD_ARMOR - 500_000;
    handle_sub_cycle(
        &state,
        SubCycleEvent {
            clan_code: "clanA".to_string(),
            raid_id,
            next_reset_at: Utc::now(),
            card_bonuses: vec![],
            morale: RaidMorale { bonus_amount: 0.0 },
            raid_started_at: Utc::now(),
            raid: simple_raid(simple_titan("Enemy1", "Lojak", sub_cycle_armor)),
            titan_target: vec![TitanTarget {
                enemy_id: "Enemy1".to_string(),
                state: vec![TitanTargetPart {
                    id: "ChestUpper".to_string(),
                    state: "2".to_string(),
                }],
            }],
        },
        serde_json::json!({}),
    )
    .await
    .unwrap();

    let loaded = boss_repo::load(&pool).await.unwrap().unwrap();
    assert_eq!(loaded.boss.head.current_armor, sub_cycle_armor);
    assert_eq!(loaded.attackable_parts, vec![BossPartName::Torso]);

    let raid_data: serde_json::Value =
        sqlx::query_scalar("SELECT raid_data FROM raid_current_state WHERE raid_id=$1")
            .bind(raid_id)
            .fetch_one(&pool)
            .await
            .unwrap();
    let stored: RaidSnapshot = serde_json::from_value(raid_data).unwrap();
    let head_armor = stored.titans[0]
        .parts
        .iter()
        .find(|part| part.part_id == "ArmorHead")
        .unwrap();
    assert_eq!(head_armor.current_hp, sub_cycle_armor as f64, "sub_cycle's raid data replaces sub_start's");
}

#[sqlx::test]
async fn sub_cycle_for_an_unseen_raid_establishes_it_without_an_attack(pool: sqlx::PgPool) {
    let state = Arc::new(AppState::new(Some(pool.clone()), 1, "test-key".to_string(), true, None));
    establish_raid(&state, 9011, "Enemy1", "Lojak", Utc::now() - chrono::Duration::days(3)).await;

    let new_raid = 9012;
    handle_sub_cycle(
        &state,
        SubCycleEvent {
            clan_code: "clanA".to_string(),
            raid_id: new_raid,
            next_reset_at: Utc::now(),
            card_bonuses: vec![],
            morale: RaidMorale { bonus_amount: 0.0 },
            raid_started_at: Utc::now(),
            raid: simple_raid(simple_titan("Enemy2", "Takedar", HEAD_ARMOR - 1)),
            titan_target: vec![],
        },
        serde_json::json!({}),
    )
    .await
    .expect("sub_cycle must no longer require a prior attack for its raid");

    let loaded = boss_repo::load(&pool).await.unwrap().unwrap();
    assert_eq!(loaded.source_raid_id, Some(new_raid));
    assert_eq!(loaded.source_enemy_id.as_deref(), Some("Enemy2"));
    assert_eq!(loaded.boss.head.current_armor, HEAD_ARMOR - 1);
}

#[sqlx::test]
async fn repeated_identical_sub_cycle_does_not_bump_the_sims_version(pool: sqlx::PgPool) {
    let state = Arc::new(AppState::new(Some(pool.clone()), 1, "test-key".to_string(), true, None));
    let raid_id = 9013;
    let event = || SubCycleEvent {
        clan_code: "clanA".to_string(),
        raid_id,
        next_reset_at: Utc::now(),
        card_bonuses: vec![],
        morale: RaidMorale { bonus_amount: 0.0 },
        raid_started_at: Utc::now(),
        raid: simple_raid(simple_titan("Enemy1", "Lojak", HEAD_ARMOR)),
        titan_target: vec![],
    };
    handle_sub_cycle(&state, event(), serde_json::json!({})).await.unwrap();
    let version = boss_repo::load(&pool).await.unwrap().unwrap().version;

    handle_sub_cycle(&state, event(), serde_json::json!({})).await.unwrap();

    assert_eq!(boss_repo::load(&pool).await.unwrap().unwrap().version, version);
}

fn sub_start_event(raid_id: i64, raid: RaidSnapshot, start_at: DateTime<Utc>) -> SubStartEvent {
    SubStartEvent {
        clan_code: "clanA".to_string(),
        raid_id,
        morale: Some(RaidMorale { bonus_amount: 0.0 }),
        raid,
        start_at: Some(start_at),
        titan_target: vec![],
    }
}

fn head_only_target() -> Vec<TitanTargetPart> {
    vec![TitanTargetPart {
        id: "Head".to_string(),
        state: "2".to_string(),
    }]
}

async fn establish_raid(
    state: &Arc<AppState>,
    raid_id: i64,
    enemy_id: &str,
    enemy_name: &str,
    start_at: DateTime<Utc>,
) {
    handle_sub_start(
        state,
        sub_start_event(
            raid_id,
            simple_raid(simple_titan(enemy_id, enemy_name, HEAD_ARMOR)),
            start_at,
        ),
        serde_json::json!({}),
        false,
    )
    .await
    .unwrap();
}

#[sqlx::test]
async fn target_event_narrows_the_current_titans_attackable_parts(pool: sqlx::PgPool) {
    let state = Arc::new(AppState::new(Some(pool.clone()), 1, "test-key".to_string(), true, None));
    let raid_id = 9101;
    establish_raid(&state, raid_id, "Enemy1", "Lojak", Utc::now()).await;

    handle_event(
        &state,
        "target",
        serde_json::json!({
            "clan_code": "clanA",
            "raid_id": raid_id,
            "enemy_id": "Enemy1",
            "state": [{ "id": "Head", "state": "2" }],
        }),
    )
    .await
    .unwrap();

    let loaded = boss_repo::load(&pool).await.unwrap().unwrap();
    assert_eq!(loaded.attackable_parts, vec![BossPartName::Head]);
}

#[sqlx::test]
async fn target_event_for_another_raid_is_ignored(pool: sqlx::PgPool) {
    let state = Arc::new(AppState::new(Some(pool.clone()), 1, "test-key".to_string(), true, None));
    establish_raid(&state, 9102, "Enemy1", "Lojak", Utc::now()).await;

    handle_target(
        &state,
        TargetEvent {
            clan_code: "clanA".to_string(),
            raid_id: 9999,
            enemy_id: "Enemy1".to_string(),
            state: head_only_target(),
        },
    )
    .await
    .unwrap();

    let loaded = boss_repo::load(&pool).await.unwrap().unwrap();
    assert_eq!(loaded.attackable_parts.len(), 8);
}

#[sqlx::test]
async fn attack_for_a_newer_raid_with_stored_raid_data_switches_the_sims_boss(pool: sqlx::PgPool) {
    let state = Arc::new(AppState::new(Some(pool.clone()), 1, "test-key".to_string(), true, None));
    let old_raid = 9201;
    let new_raid = 9202;
    establish_raid(&state, old_raid, "Enemy1", "Lojak", Utc::now() - chrono::Duration::days(3)).await;

    // The new raid's data and start time are known (e.g. its `start` stored
    // them), but the sims boss is still on the old raid.
    let new_raid_data = simple_raid(simple_titan("Enemy2", "Takedar", HEAD_ARMOR));
    sqlx::query(
        "INSERT INTO raid_current_state (raid_id,clan_code,resulting_titan_index,current_enemy_id,raid_data,titan_targets) VALUES ($1,'clanA',0,'Enemy2',$2,'[]'::jsonb)",
    )
    .bind(new_raid)
    .bind(serde_json::to_value(&new_raid_data).unwrap())
    .execute(&pool)
    .await
    .unwrap();
    store_cycle_state(&state, "clanA", new_raid, None, Utc::now(), Utc::now(), 0.0, 0.0, 0.0)
        .await
        .unwrap();

    handle_attack(
        &state,
        attack_event(new_raid, "clanA", "player1", 0, attack_snapshot("Enemy2", HEAD_ARMOR - 5, HEALTH), 10),
    )
    .await
    .unwrap();

    let loaded = boss_repo::load(&pool).await.unwrap().unwrap();
    assert_eq!(loaded.source_raid_id, Some(new_raid));
    assert_eq!(loaded.boss.boss_name, BossName::Takedar);
    assert_eq!(loaded.boss.head.current_armor, HEAD_ARMOR - 5);
}

#[sqlx::test]
async fn late_attack_from_an_older_raid_does_not_switch_the_boss_or_live_view(pool: sqlx::PgPool) {
    let state = Arc::new(AppState::new(Some(pool.clone()), 1, "test-key".to_string(), true, None));
    let old_raid = 9301;
    let new_raid = 9302;
    establish_raid(&state, old_raid, "Enemy1", "Lojak", Utc::now() - chrono::Duration::days(3)).await;
    establish_raid(&state, new_raid, "Enemy2", "Takedar", Utc::now()).await;
    handle_attack(
        &state,
        attack_event(new_raid, "clanA", "player1", 0, attack_snapshot("Enemy2", HEAD_ARMOR, HEALTH), 10),
    )
    .await
    .unwrap();

    handle_attack(
        &state,
        attack_event(old_raid, "clanA", "player1", 0, attack_snapshot("Enemy1", 7, HEALTH), 10),
    )
    .await
    .unwrap();

    let loaded = boss_repo::load(&pool).await.unwrap().unwrap();
    assert_eq!(loaded.source_raid_id, Some(new_raid));
    assert_eq!(loaded.boss.boss_name, BossName::Takedar);
    let live = state.live_attack_boss.read().await.clone().unwrap();
    assert_eq!(live.raid_id, new_raid, "a straggler must not replace the live boss");
}

#[sqlx::test]
async fn new_raids_sub_start_clears_the_previous_raids_live_boss(pool: sqlx::PgPool) {
    let state = Arc::new(AppState::new(Some(pool.clone()), 1, "test-key".to_string(), true, None));
    let old_raid = 9351;
    establish_raid(&state, old_raid, "Enemy1", "Lojak", Utc::now() - chrono::Duration::days(3)).await;
    handle_attack(
        &state,
        attack_event(old_raid, "clanA", "player1", 0, attack_snapshot("Enemy1", HEAD_ARMOR, HEALTH), 10),
    )
    .await
    .unwrap();
    assert!(state.live_attack_boss.read().await.is_some());

    establish_raid(&state, 9352, "Enemy2", "Takedar", Utc::now()).await;

    assert!(state.live_attack_boss.read().await.is_none());
}

#[sqlx::test]
async fn sub_cycle_for_a_new_raid_on_the_same_enemy_switches_the_sims_boss(pool: sqlx::PgPool) {
    let state = Arc::new(AppState::new(Some(pool.clone()), 1, "test-key".to_string(), true, None));
    let old_raid = 9401;
    let new_raid = 9402;
    establish_raid(&state, old_raid, "Enemy1", "Lojak", Utc::now() - chrono::Duration::days(3)).await;

    // The new raid's `start` never landed: its attacks only record
    // raid_current_state, and can't move the boss (no start time known).
    handle_attack(
        &state,
        attack_event(new_raid, "clanA", "player1", 0, attack_snapshot("Enemy1", HEAD_ARMOR, HEALTH), 10),
    )
    .await
    .unwrap();
    assert_eq!(
        boss_repo::load(&pool).await.unwrap().unwrap().source_raid_id,
        Some(old_raid)
    );

    handle_sub_cycle(
        &state,
        SubCycleEvent {
            clan_code: "clanA".to_string(),
            raid_id: new_raid,
            next_reset_at: Utc::now(),
            card_bonuses: vec![],
            morale: RaidMorale { bonus_amount: 0.0 },
            raid_started_at: Utc::now(),
            raid: simple_raid(simple_titan("Enemy1", "Lojak", HEAD_ARMOR)),
            titan_target: vec![TitanTarget {
                enemy_id: "Enemy1".to_string(),
                state: head_only_target(),
            }],
        },
        serde_json::json!({}),
    )
    .await
    .unwrap();

    let loaded = boss_repo::load(&pool).await.unwrap().unwrap();
    assert_eq!(loaded.source_raid_id, Some(new_raid));
    assert_eq!(loaded.attackable_parts, vec![BossPartName::Head]);
}

#[sqlx::test]
async fn sub_start_sets_the_next_reset_and_keeps_cycle_boosts(pool: sqlx::PgPool) {
    let state = Arc::new(AppState::new(Some(pool.clone()), 1, "test-key".to_string(), true, None));
    let raid_id = 9501;
    let raid_started_at = Utc::now() - chrono::Duration::hours(30);
    let expected_next_reset = raid_started_at + chrono::Duration::hours(36);
    establish_raid(&state, raid_id, "Enemy1", "Lojak", raid_started_at).await;

    // sub_cycle lands first on a reconnect, then the same raid's sub_start.
    store_cycle_state(&state, "clanA", raid_id, None, raid_started_at, expected_next_reset, 0.3, 0.05, 0.35)
        .await
        .unwrap();
    establish_raid(&state, raid_id, "Enemy1", "Lojak", raid_started_at).await;

    let (next_reset_at, mirror, team): (DateTime<Utc>, f64, f64) = sqlx::query_as(
        "SELECT next_reset_at,mirror_force_boost,team_tactics_morale_boost FROM raid_cycle_state WHERE raid_id=$1",
    )
    .bind(raid_id)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert!((next_reset_at - expected_next_reset).num_seconds().abs() <= 1);
    assert!((mirror - 0.35).abs() < 1e-9, "sub_start must not wipe sub_cycle's boosts");
    assert!((team - 0.05).abs() < 1e-9);
}
