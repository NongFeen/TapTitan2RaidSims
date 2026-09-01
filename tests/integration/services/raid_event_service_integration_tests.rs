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
    let state = Arc::new(AppState::new(Some(pool.clone()), 1, "test-key".to_string(), None));
    let raid_id = 9001;
    let titan = simple_titan("Enemy1", "Lojak", HEAD_ARMOR);
    let raid = simple_raid(titan);
    let event = SubStartEvent {
        clan_code: "clanA".to_string(),
        raid_id,
        morale: RaidMorale { bonus_amount: 0.4 },
        raid,
        start_at: Utc::now(),
        titan_target: vec![],
    };

    handle_sub_start(&state, event, serde_json::json!({}))
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
    let state = Arc::new(AppState::new(Some(pool.clone()), 1, "test-key".to_string(), None));
    let raid_id = 9002;
    let first_event = SubStartEvent {
        clan_code: "clanA".to_string(),
        raid_id,
        morale: RaidMorale { bonus_amount: 0.1 },
        raid: simple_raid(simple_titan("Enemy1", "Lojak", HEAD_ARMOR)),
        start_at: Utc::now(),
        titan_target: vec![],
    };
    handle_sub_start(&state, first_event, serde_json::json!({}))
        .await
        .unwrap();
    let first_version = boss_repo::load(&pool).await.unwrap().unwrap().version;

    // A later sub_start for the SAME raid -- carries a real (but unreliable,
    // per the doc comment on handle_sub_start) titan_target selection and
    // different current_armor. Neither should touch the boss row at all.
    let second_event = SubStartEvent {
        clan_code: "clanA".to_string(),
        raid_id,
        morale: RaidMorale { bonus_amount: 0.5 },
        raid: simple_raid(simple_titan("Enemy1", "Lojak", 42)),
        start_at: Utc::now(),
        titan_target: vec![TitanTarget {
            enemy_id: "Enemy1".to_string(),
            state: vec![TitanTargetPart {
                id: "Head".to_string(),
                state: "2".to_string(),
            }],
        }],
    };
    handle_sub_start(&state, second_event, serde_json::json!({}))
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
    let state = Arc::new(AppState::new(Some(pool.clone()), 1, "test-key".to_string(), None));
    let raid_id = 9003;
    handle_sub_start(
        &state,
        SubStartEvent {
            clan_code: "clanA".to_string(),
            raid_id,
            morale: RaidMorale { bonus_amount: 0.0 },
            raid: simple_raid(simple_titan("Enemy1", "Lojak", HEAD_ARMOR)),
            start_at: Utc::now(),
            titan_target: vec![],
        },
        serde_json::json!({}),
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
    let state = Arc::new(AppState::new(Some(pool.clone()), 1, "test-key".to_string(), None));
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
            morale: RaidMorale { bonus_amount: 0.0 },
            raid: raid.clone(),
            start_at: Utc::now(),
            titan_target: vec![],
        },
        serde_json::json!({}),
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
async fn handle_sub_cycle_updates_targets_only_and_never_touches_current_hp(pool: sqlx::PgPool) {
    let state = Arc::new(AppState::new(Some(pool.clone()), 1, "test-key".to_string(), None));
    let raid_id = 9005;
    let raid = simple_raid(simple_titan("Enemy1", "Lojak", HEAD_ARMOR));
    handle_sub_start(
        &state,
        SubStartEvent {
            clan_code: "clanA".to_string(),
            raid_id,
            morale: RaidMorale { bonus_amount: 0.0 },
            raid: raid.clone(),
            start_at: Utc::now(),
            titan_target: vec![],
        },
        serde_json::json!({}),
    )
    .await
    .unwrap();

    // An attack records a real, attack-owned HP value on Head.
    let attack_set_armor = HEAD_ARMOR - 300_000;
    handle_attack(
        &state,
        attack_event(
            raid_id,
            "clanA",
            "player1",
            0,
            attack_snapshot("Enemy1", attack_set_armor, HEALTH),
            4610,
        ),
    )
    .await
    .unwrap();

    // sub_cycle for the SAME titan, narrowing targeting down to Torso only.
    // Its own `raid` snapshot deliberately carries Head's ORIGINAL armor
    // value -- if sub_cycle touched HP at all, this would revert the
    // attack's update.
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
    assert_eq!(
        loaded.boss.head.current_armor, attack_set_armor,
        "sub_cycle must never overwrite HP that attack already set"
    );
    assert_eq!(
        loaded.attackable_parts,
        vec![BossPartName::Torso],
        "sub_cycle is the sole authority for targeting"
    );
}
