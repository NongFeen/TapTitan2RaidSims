use std::{collections::HashMap, str::FromStr, sync::Arc};

use chrono::{DateTime, Utc};
use serde::Deserialize;
use serde_json::Value;

use crate::{
    error::AppError,
    models::{
        app::{
            CreateSimulationJobRequest, LiveAttackBossView, LiveAttackingCard,
            LiveAttackingPlayer, LiveBossDisplayPart,
        },
        boss::{Boss, BossName, BossPartName, CurseType, GlobalRaidModifier, PartState},
        cards::CardName,
    },
    services::{boss_repo, job_service},
    state::AppState,
};

const RAID_STATE_LOCK: i64 = 721_934_762;
const RESET_INTERVAL_HOURS: i64 = 12;

pub async fn handle_event(state: &Arc<AppState>, event: &str, data: Value) -> Result<(), AppError> {
    let affects_live_boss = matches!(event, "attack" | "sub_start" | "sub_cycle" | "cycle_reset");
    let result = match event {
        "attack" => handle_attack(state, serde_json::from_value(data)?).await,
        "sub_start" => handle_sub_start(state, serde_json::from_value(data.clone())?, data).await,
        "sub_cycle" => handle_sub_cycle(state, serde_json::from_value(data.clone())?, data).await,
        "cycle_reset" => handle_cycle_reset(state, serde_json::from_value(data)?).await,
        "start_attack" => handle_start_attack(state, serde_json::from_value(data)?).await,
        _ => Ok(()),
    };
    // Pings any open live-boss SSE streams to rebuild and re-check their view
    // -- see `routes::raids::live_current_boss_stream`.
    if result.is_ok() && affects_live_boss {
        let _ = state.live_boss_tx.send(());
    }
    result
}

const BASE_ATTACK_DURATION_SECONDS: f64 = 33.0;
const BATTLE_DRUMS_DURATION_ADJUST_SECONDS: f64 = -10.0;
const ATTACK_DURATION_MODIFIER_ADJUST_SECONDS: f64 = 3.0;
const SUPPORT_EFFECT_MODIFIER_ADJUST_SECONDS: f64 = -10.0 * 1.15;

/// The attack timer TT2 shows to other clan members: a fixed base window,
/// shortened if the attacker's own deck carries Battle Drums (matches its
/// `attack_duration_add_seconds` support modifier, see
/// `card_function/support/battle_drums.rs`), and then adjusted again by
/// whatever global raid modifier is currently active for the whole clan.
fn attack_duration_seconds(cards: &[CardName], global_modifier: GlobalRaidModifier) -> f64 {
    let mut duration = BASE_ATTACK_DURATION_SECONDS;
    if cards.contains(&CardName::BattleDrums) {
        duration += BATTLE_DRUMS_DURATION_ADJUST_SECONDS;
    }
    duration += match global_modifier {
        GlobalRaidModifier::AttackDuration => ATTACK_DURATION_MODIFIER_ADJUST_SECONDS,
        GlobalRaidModifier::SupportEffect => SUPPORT_EFFECT_MODIFIER_ADJUST_SECONDS,
        _ => 0.0,
    };
    duration.max(0.0)
}

async fn handle_start_attack(
    state: &Arc<AppState>,
    event: StartAttackEvent,
) -> Result<(), AppError> {
    let cards: Vec<LiveAttackingCard> = event
        .cards
        .iter()
        .map(|card_id| match CardName::from_str(card_id) {
            Ok(card) => LiveAttackingCard {
                card_id: card_id.clone(),
                display_name: card.display_name().to_string(),
                image_url: card.image_url(),
            },
            Err(_) => LiveAttackingCard {
                card_id: card_id.clone(),
                display_name: card_id.clone(),
                image_url: String::new(),
            },
        })
        .collect();
    let recognized_cards: Vec<CardName> = event
        .cards
        .iter()
        .filter_map(|card_id| CardName::from_str(card_id).ok())
        .collect();

    let global_modifier = match state.optional_db() {
        Some(db) => boss_repo::load(db)
            .await?
            .map_or(GlobalRaidModifier::None, |loaded| {
                loaded.boss.global_raid_modifier
            }),
        None => GlobalRaidModifier::None,
    };
    let duration_seconds = attack_duration_seconds(&recognized_cards, global_modifier);

    let player = LiveAttackingPlayer {
        player_code: event.player.player_code.clone(),
        name: event.player.name,
        cards,
        started_at: event.started_at,
        duration_seconds,
    };
    let mut players = state.live_attacking_players.write().await;
    let now = Utc::now();
    players.retain(|_, existing| !existing.is_expired(now));
    players.insert(event.player.player_code, player.clone());
    drop(players);
    // No receivers (no widget currently open anywhere) just drops the send --
    // that's fine, the next SSE connection gets the full snapshot anyway.
    let _ = state.live_attacking_players_tx.send(player);
    Ok(())
}

async fn handle_sub_start(
    state: &Arc<AppState>,
    event: SubStartEvent,
    raw_payload: Value,
) -> Result<(), AppError> {
    store_sub_start_cycle_state(
        state,
        &event.clan_code,
        event.raid_id,
        event.start_at,
        event.morale.bonus_amount,
    )
    .await?;

    let titan_count = event.raid.titans.len();
    let sequence_count = event.raid.spawn_sequence.len();
    let raid_json = serde_json::to_value(&event.raid)?;
    let titan_target_json = serde_json::to_value(&event.titan_target)?;

    let mut tx = state.db()?.begin().await?;
    sqlx::query("SELECT pg_advisory_xact_lock($1)")
        .bind(RAID_STATE_LOCK)
        .execute(&mut *tx)
        .await?;
    // Held across every check and write below so a concurrent attack event
    // (which takes the same lock) can't observe or leave things half-updated.
    let is_new_raid_row = !sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS(SELECT 1 FROM raid_current_state WHERE raid_id=$1)",
    )
    .bind(event.raid_id)
    .fetch_one(&mut *tx)
    .await?;
    let current = boss_repo::load_for_update(&mut tx).await?;
    // Not just "raid_id never seen before": even for an already-tracked
    // raid_id, the sims boss can still be pointed at a different raid (e.g.
    // events arriving out of order across a reconnect). Either way, the boss
    // needs to be (re)established for *this* raid_id.
    let boss_matches_raid = current
        .as_ref()
        .is_some_and(|boss| boss.source_raid_id == Some(event.raid_id));

    let new_boss_version = if !boss_matches_raid {
        // Set up a fresh sims boss starting from the first titan in this
        // raid's own spawn sequence -- also establishes
        // `resulting_titan_index`/`current_enemy_id` immediately (normally
        // only an `attack` event sets these) so a `sub_cycle` arriving
        // before the first attack doesn't hit "arrived before an attack
        // established raid state".
        let enemy_id = first_titan_enemy_id(&event.raid)?.to_string();
        // recommend_1_to_2_part_patterns_only is an admin preference, not
        // tied to any one raid -- carry it over rather than silently
        // resetting it just because the boss is being re-established.
        let preserve_narrow = current
            .map(|previous| previous.boss.recommend_1_to_2_part_patterns_only)
            .unwrap_or(false);
        let (boss, attackable_parts) = boss_from_raid_snapshot(
            &event.raid,
            &event.titan_target,
            &enemy_id,
            preserve_narrow,
        )?;

        if is_new_raid_row {
            sqlx::query(
                "INSERT INTO raid_current_state (raid_id,clan_code,resulting_titan_index,current_enemy_id,raid_data,titan_targets,raw_sub_start,received_at) VALUES ($1,$2,0,$3,$4,$5,$6,NOW())",
            )
            .bind(event.raid_id)
            .bind(&event.clan_code)
            .bind(&enemy_id)
            .bind(&raid_json)
            .bind(&titan_target_json)
            .bind(&raw_payload)
            .execute(&mut *tx)
            .await?;
        } else {
            sqlx::query(
                "UPDATE raid_current_state SET clan_code=$2,resulting_titan_index=0,current_enemy_id=$3,raid_data=$4,titan_targets=$5,raw_sub_start=$6,received_at=NOW(),updated_at=NOW() WHERE raid_id=$1",
            )
            .bind(event.raid_id)
            .bind(&event.clan_code)
            .bind(&enemy_id)
            .bind(&raid_json)
            .bind(&titan_target_json)
            .bind(&raw_payload)
            .execute(&mut *tx)
            .await?;
        }

        let boss_version = boss_repo::store(
            &mut tx,
            boss_repo::BossWrite {
                boss: &boss,
                attackable_parts: Some(&attackable_parts),
                source_raid_id: Some(event.raid_id),
                source_titan_index: Some(0),
                source_enemy_id: Some(&enemy_id),
                bump_version: true,
            },
        )
        .await?;
        Some(boss_version)
    } else {
        // Boss already belongs to this raid -- refresh the stored raw data
        // unconditionally (a later sub_start can carry a real target
        // selection even when an earlier one for this same raid didn't),
        // and update the boss's attackable_parts to match if so.
        sqlx::query(
            "UPDATE raid_current_state SET clan_code=$2,raid_data=$3,titan_targets=$4,raw_sub_start=$5,received_at=NOW(),updated_at=NOW() WHERE raid_id=$1",
        )
        .bind(event.raid_id)
        .bind(&event.clan_code)
        .bind(&raid_json)
        .bind(&titan_target_json)
        .bind(&raw_payload)
        .execute(&mut *tx)
        .await?;

        match current.and_then(|boss| boss.source_enemy_id) {
            Some(enemy_id) => {
                update_boss_targets_from_titan_target(&mut tx, &event.titan_target, &enemy_id)
                    .await?
            }
            None => None,
        }
    };
    tx.commit().await?;

    if let Some(boss_version) = new_boss_version {
        job_service::spawn_old_job_cleanup(Arc::clone(state), boss_version);
        queue_auto_simulations(state, None).await?;
    }

    tracing::info!(
        raid_id = event.raid_id,
        clan_code = event.clan_code,
        start_at = %event.start_at,
        morale = event.morale.bonus_amount,
        titan_count,
        sequence_count,
        is_new_raid_row,
        boss_matches_raid,
        "stored TT2 sub_start base raid data"
    );
    Ok(())
}

async fn handle_attack(state: &Arc<AppState>, attack: AttackEvent) -> Result<(), AppError> {
    *state.live_attack_boss.write().await = Some(LiveAttackBossView {
        clan_code: attack.clan_code.clone(),
        raid_id: attack.raid_id,
        cycle: attack.cycle,
        titan_index: attack.raid_state.titan_index,
        boss_data: serde_json::to_value(&attack.raid_state.current)?,
        received_at: Utc::now(),
        display_parts: None,
    });
    tracing::info!(
        raid_id = attack.raid_id,
        cycle = attack.cycle,
        titan_index = attack.raid_state.titan_index,
        enemy_id = %attack.raid_state.current.enemy_id,
        "updated live current boss from TT2 attack event"
    );
    let components = attack_components(&attack)?;
    let attacked_titan_index = components
        .first()
        .map_or(attack.raid_state.titan_index, |component| {
            component.titan_index
        });
    let tap_damage = components
        .iter()
        .filter(|component| component.card_id.is_none())
        .map(|component| component.total_damage)
        .sum::<u64>();
    let total_damage = components
        .iter()
        .map(|component| component.total_damage)
        .sum::<u64>();

    let mut tx = state.db()?.begin().await?;
    sqlx::query("SELECT pg_advisory_xact_lock($1)")
        .bind(RAID_STATE_LOCK)
        .execute(&mut *tx)
        .await?;

    // Attacks can arrive for a player who isn't in `players` yet (they've
    // never been through a clan-stats sync). Rather than store an
    // orphaned/unlinked log row, skip logging this one attack (and its
    // card-damage breakdown) entirely -- boss HP tracking below is driven
    // by the live raid_state snapshot regardless, so it isn't affected.
    let player_known: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM players WHERE player_id=$1)")
        .bind(&attack.player.player_code)
        .fetch_one(&mut *tx)
        .await?;

    let logged = if player_known {
        let inserted = sqlx::query(
            "INSERT INTO raid_attack_logs (raid_id,clan_code,player_id,player_name,cycle,attack_datetime,attacked_titan_index,resulting_titan_index,enemy_id,tap_damage,total_damage) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,CAST($10 AS NUMERIC),CAST($11 AS NUMERIC)) ON CONFLICT (raid_id,player_id,attack_datetime) DO NOTHING",
        )
        .bind(attack.raid_id)
        .bind(&attack.clan_code)
        .bind(&attack.player.player_code)
        .bind(&attack.player.name)
        .bind(attack.cycle)
        .bind(attack.attack_log.attack_datetime)
        .bind(attacked_titan_index)
        .bind(attack.raid_state.titan_index)
        .bind(&attack.raid_state.current.enemy_id)
        .bind(tap_damage.to_string())
        .bind(total_damage.to_string())
        .execute(&mut *tx)
        .await?;

        if inserted.rows_affected() == 0 {
            tx.commit().await?;
            sync_sims_boss_on_phase_transition(state, &attack).await?;
            return Ok(());
        }

        let cards = aggregate_card_components(&components);
        sqlx::query(
            "INSERT INTO raid_attack_components (raid_id,player_id,attack_datetime,tap_damage,card1,card1_level,card1_damage,card2,card2_level,card2_damage,card3,card3_level,card3_damage) VALUES ($1,$2,$3,CAST($4 AS NUMERIC),$5,$6,CAST($7 AS NUMERIC),$8,$9,CAST($10 AS NUMERIC),$11,$12,CAST($13 AS NUMERIC))",
        )
        .bind(attack.raid_id)
        .bind(&attack.player.player_code)
        .bind(attack.attack_log.attack_datetime)
        .bind(tap_damage.to_string())
        .bind(cards.first().map(|card| card.card_id.as_str()))
        .bind(cards.first().and_then(|card| card.card_level))
        .bind(cards.first().map_or("0".to_string(), |card| card.damage.to_string()))
        .bind(cards.get(1).map(|card| card.card_id.as_str()))
        .bind(cards.get(1).and_then(|card| card.card_level))
        .bind(cards.get(1).map_or("0".to_string(), |card| card.damage.to_string()))
        .bind(cards.get(2).map(|card| card.card_id.as_str()))
        .bind(cards.get(2).and_then(|card| card.card_level))
        .bind(cards.get(2).map_or("0".to_string(), |card| card.damage.to_string()))
        .execute(&mut *tx)
        .await?;
        true
    } else {
        tracing::debug!(
            raid_id = attack.raid_id,
            player_code = attack.player.player_code,
            "skipping raid_attack_logs insert; player is not tracked in players yet"
        );
        false
    };

    sqlx::query(
        "INSERT INTO raid_current_state (raid_id,clan_code,resulting_titan_index,current_enemy_id,refresh_required) VALUES ($1,$2,$3,$4,FALSE) ON CONFLICT (raid_id) DO UPDATE SET clan_code=EXCLUDED.clan_code, resulting_titan_index=CASE WHEN EXCLUDED.resulting_titan_index >= COALESCE(raid_current_state.resulting_titan_index,-1) THEN EXCLUDED.resulting_titan_index ELSE raid_current_state.resulting_titan_index END, current_enemy_id=CASE WHEN EXCLUDED.resulting_titan_index >= COALESCE(raid_current_state.resulting_titan_index,-1) THEN EXCLUDED.current_enemy_id ELSE raid_current_state.current_enemy_id END, refresh_required=FALSE, updated_at=NOW()",
    )
    .bind(attack.raid_id)
    .bind(&attack.clan_code)
    .bind(attack.raid_state.titan_index)
    .bind(&attack.raid_state.current.enemy_id)
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;

    tracing::info!(
        raid_id = attack.raid_id,
        player_code = attack.player.player_code,
        cycle = attack.cycle,
        attacked_titan_index,
        resulting_titan_index = attack.raid_state.titan_index,
        total_damage,
        logged,
        "stored TT2 raid attack"
    );
    sync_sims_boss_on_phase_transition(state, &attack).await?;
    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PhaseRefresh {
    Full,
    Incremental(u8),
}

async fn sync_sims_boss_on_phase_transition(
    state: &Arc<AppState>,
    attack: &AttackEvent,
) -> Result<bool, AppError> {
    let mut tx = state.db()?.begin().await?;
    sqlx::query("SELECT pg_advisory_xact_lock($1)")
        .bind(RAID_STATE_LOCK)
        .execute(&mut *tx)
        .await?;

    let Some(current) = boss_repo::load_for_update(&mut tx).await? else {
        return Ok(false);
    };
    let boss = current.boss;
    let targets = current.attackable_parts;
    let source_raid_id = current.source_raid_id;
    let source_titan_index = current.source_titan_index;
    let source_enemy_id = current.source_enemy_id;
    let expected_boss = boss_name(&attack.raid_state.current.enemy_id, "")?;
    let source_matches = source_enemy_id
        .as_deref()
        .map_or(boss.boss_name == expected_boss, |enemy_id| {
            enemy_id == attack.raid_state.current.enemy_id
        })
        && source_raid_id.is_none_or(|raid_id| raid_id == attack.raid_id)
        && source_titan_index
            .is_none_or(|titan_index| titan_index == attack.raid_state.titan_index);
    let Some(incoming) = boss_from_attack_snapshot(&boss, &attack.raid_state.current)? else {
        return Ok(false);
    };
    if !source_matches {
        return Ok(false);
    }
    let refresh = classify_phase_refresh(&boss, &targets, &incoming);

    if let Some(refresh) = refresh {
        let boss_version = boss_repo::store(
            &mut tx,
            boss_repo::BossWrite {
                boss: &incoming,
                attackable_parts: None,
                source_raid_id: Some(attack.raid_id),
                source_titan_index: Some(attack.raid_state.titan_index),
                source_enemy_id: Some(&attack.raid_state.current.enemy_id),
                bump_version: true,
            },
        )
        .await?;
        tx.commit().await?;

        job_service::spawn_old_job_cleanup(Arc::clone(state), boss_version);
        queue_auto_simulations(
            state,
            match refresh {
                PhaseRefresh::Full => None,
                PhaseRefresh::Incremental(mask) => Some(mask),
            },
        )
        .await?;
        tracing::info!(
            raid_id = attack.raid_id,
            titan_index = attack.raid_state.titan_index,
            enemy_id = %attack.raid_state.current.enemy_id,
            boss_version,
            target_count = targets.len(),
            refresh_mode = ?refresh,
            "boss target phase changed; updated sims boss and queued auto simulations"
        );
        Ok(true)
    } else {
        // No phase change — sync HP values without bumping the simulation version.
        boss_repo::store(
            &mut tx,
            boss_repo::BossWrite {
                boss: &incoming,
                attackable_parts: None,
                source_raid_id: Some(attack.raid_id),
                source_titan_index: Some(attack.raid_state.titan_index),
                source_enemy_id: Some(&attack.raid_state.current.enemy_id),
                bump_version: false,
            },
        )
        .await?;
        tx.commit().await?;
        tracing::debug!(
            raid_id = attack.raid_id,
            titan_index = attack.raid_state.titan_index,
            enemy_id = %attack.raid_state.current.enemy_id,
            "synced sims boss HP values from attack event (no phase change)"
        );
        Ok(false)
    }
}

fn boss_from_attack_snapshot(
    boss: &Boss,
    live: &AttackCurrentBoss,
) -> Result<Option<Boss>, AppError> {
    let mut incoming = boss.clone();
    for part_name in BossPartName::all() {
        let Some((current_armor, current_health)) = attack_part_values(live, part_name)? else {
            return Ok(None);
        };
        let part = incoming.part_mut(part_name);
        part.current_armor = current_armor;
        part.current_health = current_health;
    }
    incoming.sync_part_states_from_current_values();
    Ok(Some(incoming))
}

fn classify_phase_refresh(
    current: &Boss,
    targets: &[BossPartName],
    incoming: &Boss,
) -> Option<PhaseRefresh> {
    // A part appearing to regress here means an `attack` event arrived out
    // of order relative to another (concurrent players, network jitter) --
    // not a real boss reset. Real resets (new titan, new cycle) go through
    // `handle_sub_cycle` instead, which is authoritative and not subject to
    // this per-tap reordering risk. Treat any regression here as stale and
    // ignore it, rather than resimulating against a phantom rollback.
    if targets.is_empty()
        || BossPartName::all().iter().any(|part_name| {
            phase_rank(incoming.part(*part_name).part_state)
                < phase_rank(current.part(*part_name).part_state)
        })
    {
        return None;
    }

    let had_selected_armor = targets.iter().any(|part_name| {
        matches!(
            current.part(*part_name).part_state,
            PartState::Armor | PartState::Cursed
        )
    });
    let incoming_has_selected_armor = targets.iter().any(|part_name| {
        matches!(
            incoming.part(*part_name).part_state,
            PartState::Armor | PartState::Cursed
        )
    });
    if had_selected_armor && !incoming_has_selected_armor {
        return Some(PhaseRefresh::Full);
    }

    // All cursed parts cleared — curse damage modifier changes even on non-targeted parts.
    let all_parts = BossPartName::all();
    let had_any_cursed = all_parts
        .iter()
        .any(|p| current.part(*p).part_state == PartState::Cursed);
    let incoming_has_any_cursed = all_parts
        .iter()
        .any(|p| incoming.part(*p).part_state == PartState::Cursed);
    if had_any_cursed && !incoming_has_any_cursed {
        return Some(PhaseRefresh::Full);
    }

    let skeleton_mask = targets.iter().fold(0u8, |mask, part_name| {
        if current.part(*part_name).part_state == PartState::Body
            && incoming.part(*part_name).part_state == PartState::Skeleton
        {
            mask | part_name.dependency_mask()
        } else {
            mask
        }
    });
    (skeleton_mask != 0).then_some(PhaseRefresh::Incremental(skeleton_mask))
}

fn phase_rank(state: PartState) -> u8 {
    match state {
        PartState::Cursed | PartState::Armor => 0,
        PartState::Body => 1,
        PartState::Skeleton => 2,
    }
}

fn attack_part_values(
    live: &AttackCurrentBoss,
    part_name: BossPartName,
) -> Result<Option<(u64, u64)>, AppError> {
    let (body_id, armor_id) = part_ids(part_name);
    let Some(body) = live.parts.iter().find(|part| part.part_id == body_id) else {
        return Ok(None);
    };
    // TT2 omits armor entries for parts whose armor is already destroyed; treat absence as 0.
    let current_armor = match live.parts.iter().find(|part| part.part_id == armor_id) {
        Some(armor) => rounded_u64(armor.current_hp, armor_id)?,
        None => 0,
    };
    Ok(Some((current_armor, rounded_u64(body.current_hp, body_id)?)))
}

fn part_ids(part_name: BossPartName) -> (&'static str, &'static str) {
    match part_name {
        BossPartName::Head => ("BodyHead", "ArmorHead"),
        BossPartName::Torso => ("BodyChestUpper", "ArmorChestUpper"),
        BossPartName::RightShoulder => ("BodyArmUpperLeft", "ArmorArmUpperLeft"),
        BossPartName::LeftShoulder => ("BodyArmUpperRight", "ArmorArmUpperRight"),
        BossPartName::RightHand => ("BodyHandLeft", "ArmorHandLeft"),
        BossPartName::LeftHand => ("BodyHandRight", "ArmorHandRight"),
        BossPartName::RightLeg => ("BodyLegUpperLeft", "ArmorLegUpperLeft"),
        BossPartName::LeftLeg => ("BodyLegUpperRight", "ArmorLegUpperRight"),
    }
}

/// Reconstructs a `LiveAttackBossView` from the persisted, continuously-synced
/// `current_boss`/`current_boss_parts` tables -- used when the in-memory live
/// attack snapshot is empty (e.g. right after a backend restart, before the
/// next `attack` event arrives). This is necessarily an approximation of the
/// real live feed: `current_hp` is the sum of each part's current body HP
/// (the raw feed reports the titan's own total, which isn't stored), and
/// armor entries are only synthesized for parts still in Armor/Cursed state,
/// matching TT2's own convention of omitting a destroyed layer.
pub fn live_boss_from_persisted(
    loaded: &crate::services::boss_repo::LoadedBoss,
    clan_code: String,
    cycle: i32,
) -> Option<LiveAttackBossView> {
    let raid_id = loaded.source_raid_id?;
    let titan_index = loaded.source_titan_index.unwrap_or_default();
    let enemy_id = loaded.source_enemy_id.clone().unwrap_or_default();

    let mut parts = Vec::with_capacity(16);
    let mut current_hp_total = 0.0f64;
    let mut display_parts = Vec::with_capacity(8);
    for part_name in BossPartName::all() {
        let part = loaded.boss.part(part_name);
        let (body_id, armor_id) = part_ids(part_name);
        parts.push(serde_json::json!({
            "part_id": body_id,
            "current_hp": part.current_health as f64,
        }));
        if matches!(part.part_state, PartState::Armor | PartState::Cursed) {
            parts.push(serde_json::json!({
                "part_id": armor_id,
                "current_hp": part.current_armor as f64,
            }));
        }
        current_hp_total += part.current_health as f64;

        let (current_hp, max_hp) = match part.part_state {
            PartState::Armor | PartState::Cursed => (part.current_armor, part.max_armor),
            PartState::Body => (part.current_health, part.max_health),
            PartState::Skeleton => (0, part.max_health),
        };
        display_parts.push(LiveBossDisplayPart {
            part_name,
            part_state: part.part_state,
            current_hp,
            max_hp,
            is_targeted: loaded.attackable_parts.contains(&part_name),
        });
    }

    Some(LiveAttackBossView {
        clan_code,
        raid_id,
        cycle,
        titan_index,
        boss_data: serde_json::json!({
            "enemy_id": enemy_id,
            "current_hp": current_hp_total,
            "parts": parts,
        }),
        received_at: loaded.updated_at,
        display_parts: Some(display_parts),
    })
}

pub fn live_boss_display_parts(
    live_boss_data: &Value,
    raid_data: &Value,
    titan_targets: &Value,
) -> Result<Option<Vec<LiveBossDisplayPart>>, AppError> {
    let live: AttackCurrentBoss = serde_json::from_value(live_boss_data.clone())?;
    let raid: RaidSnapshot = serde_json::from_value(raid_data.clone())?;
    let targets: Vec<TitanTarget> = serde_json::from_value(titan_targets.clone())?;
    let Some(titan) = raid
        .titans
        .iter()
        .find(|titan| titan.enemy_id == live.enemy_id)
    else {
        return Ok(None);
    };
    let Some(titan_target) = targets
        .iter()
        .find(|target| target.enemy_id == live.enemy_id)
    else {
        return Ok(None);
    };
    let stored_parts = titan
        .parts
        .iter()
        .map(|part| (part.part_id.as_str(), part))
        .collect::<HashMap<_, _>>();
    let mut display_parts = Vec::with_capacity(8);

    for part_name in BossPartName::all() {
        let (body_id, armor_id) = part_ids(part_name);
        let Some(body) = stored_parts.get(body_id) else {
            return Ok(None);
        };
        let Some(armor) = stored_parts.get(armor_id) else {
            return Ok(None);
        };
        let Some((current_armor, current_health)) = attack_part_values(&live, part_name)? else {
            return Ok(None);
        };
        let (part_state, current_hp, max_hp) = if current_armor > 0 {
            (
                if armor.cursed {
                    PartState::Cursed
                } else {
                    PartState::Armor
                },
                current_armor,
                rounded_u64(armor.total_hp, armor_id)?,
            )
        } else if current_health > 0 {
            (
                PartState::Body,
                current_health,
                rounded_u64(body.total_hp, body_id)?,
            )
        } else {
            (PartState::Skeleton, 0, rounded_u64(body.total_hp, body_id)?)
        };
        display_parts.push(LiveBossDisplayPart {
            part_name,
            part_state,
            current_hp,
            max_hp,
            is_targeted: titan_target.state.iter().any(|target| {
                target.state == "2" && target_part_name(&target.id).ok() == Some(part_name)
            }),
        });
    }

    Ok(Some(display_parts))
}

async fn handle_sub_cycle(
    state: &Arc<AppState>,
    event: SubCycleEvent,
    raw_payload: Value,
) -> Result<(), AppError> {
    let previous_next_reset_at: Option<DateTime<Utc>> =
        sqlx::query_scalar("SELECT next_reset_at FROM raid_cycle_state WHERE raid_id=$1")
            .bind(event.raid_id)
            .fetch_optional(state.db()?)
            .await?;
    let cycle_changed = previous_next_reset_at != Some(event.next_reset_at);
    let mirror_changed = store_cycle_state(
        state,
        &event.clan_code,
        event.raid_id,
        None,
        event.raid_started_at,
        event.next_reset_at,
        event.morale.bonus_amount,
        bonus_value(&event.card_bonuses, "TeamTacticsClanMoraleBoost"),
        bonus_value(&event.card_bonuses, "MirrorForceBoost"),
    )
    .await?;

    let runtime: Option<(Option<i32>, Option<String>, Option<Value>, Option<Value>)> =
        sqlx::query_as(
        "SELECT resulting_titan_index,current_enemy_id,raid_data,raw_sub_start FROM raid_current_state WHERE raid_id=$1",
    )
    .bind(event.raid_id)
    .fetch_optional(state.db()?)
    .await?;
    let (titan_index, enemy_id, stored_base_raid, raw_sub_start) = runtime.ok_or_else(|| {
        AppError::Conflict("sub_cycle arrived before an attack established raid state".into())
    })?;
    let (base_raid, used_sub_cycle_fallback) =
        select_base_raid(stored_base_raid, raw_sub_start.is_some(), &event.raid)?;
    let enemy_id = enemy_id
        .ok_or_else(|| AppError::Conflict("No attack has identified the current titan".into()))?;
    let titan_index = titan_index.unwrap_or_default();

    let previous_boss = boss_repo::load(state.db()?).await?;
    let preserve_narrow = previous_boss
        .as_ref()
        .map(|previous| previous.boss.recommend_1_to_2_part_patterns_only)
        .unwrap_or(false);
    let (mut boss, attackable_parts) =
        boss_from_raid_snapshot(&base_raid, &event.titan_target, &enemy_id, preserve_narrow)?;
    let enemy_changed = previous_boss
        .as_ref()
        .and_then(|previous| previous.source_enemy_id.as_deref())
        != Some(enemy_id.as_str());
    let targets_changed = previous_boss
        .as_ref()
        .is_none_or(|previous| previous.attackable_parts != attackable_parts);
    // Only trust attack-tracked HP over this snapshot within the same cycle
    // against the same titan -- sub_cycle reports periodically and can lag
    // behind more frequent attack events, but a new cycle or a new titan is
    // exactly when parts are expected to legitimately reset/regenerate, and
    // the fresh snapshot must be allowed through rather than silently
    // reverted back to the old, already-depleted values.
    if !enemy_changed && !cycle_changed {
        if let Some(previous) = &previous_boss {
            preserve_current_boss_values(&mut boss, &previous.boss);
        }
    }
    let parts_changed = previous_boss.as_ref().is_none_or(|previous| {
        BossPartName::all()
            .iter()
            .any(|part_name| boss.part(*part_name).part_state != previous.boss.part(*part_name).part_state)
    });
    let needs_simulation =
        cycle_changed || enemy_changed || targets_changed || mirror_changed || parts_changed;

    let mut tx = state.db()?.begin().await?;
    sqlx::query("SELECT pg_advisory_xact_lock($1)")
        .bind(RAID_STATE_LOCK)
        .execute(&mut *tx)
        .await?;
    sqlx::query(
        "UPDATE raid_current_state SET clan_code=$2,titan_targets=$3,raw_sub_cycle=$4,raid_data=CASE WHEN raw_sub_start IS NULL THEN $5 ELSE raid_data END,received_at=NOW(),refresh_required=FALSE,updated_at=NOW() WHERE raid_id=$1",
    )
    .bind(event.raid_id)
    .bind(&event.clan_code)
    .bind(serde_json::to_value(&event.titan_target)?)
    .bind(raw_payload)
    .bind(serde_json::to_value(&event.raid)?)
    .execute(&mut *tx)
    .await?;

    let boss_version = boss_repo::store(
        &mut tx,
        boss_repo::BossWrite {
            boss: &boss,
            attackable_parts: Some(&attackable_parts),
            source_raid_id: Some(event.raid_id),
            source_titan_index: Some(titan_index),
            source_enemy_id: Some(&enemy_id),
            bump_version: needs_simulation,
        },
    )
    .await?;
    tx.commit().await?;

    if needs_simulation {
        job_service::spawn_old_job_cleanup(Arc::clone(state), boss_version);
        queue_auto_simulations(state, None).await?;
    }
    tracing::info!(
        event.raid_id,
        titan_index,
        enemy_id,
        enemy_changed,
        targets_changed,
        cycle_changed,
        mirror_changed,
        needs_simulation,
        used_sub_cycle_fallback,
        "applied TT2 sub_cycle targeting to the stored base raid boss"
    );
    Ok(())
}

async fn handle_cycle_reset(state: &Arc<AppState>, event: CycleResetEvent) -> Result<(), AppError> {
    let mirror_changed = store_cycle_state(
        state,
        &event.clan_code,
        event.raid_id,
        Some(event.started_at),
        event.raid_started_at,
        event.next_reset_at,
        event.morale.bonus_amount,
        bonus_value(&event.card_bonuses, "TeamTacticsClanMoraleBoost"),
        bonus_value(&event.card_bonuses, "MirrorForceBoost"),
    )
    .await?;
    if mirror_changed {
        queue_auto_simulations(state, None).await?;
    }
    Ok(())
}

async fn store_sub_start_cycle_state(
    state: &Arc<AppState>,
    clan_code: &str,
    raid_id: i64,
    start_at: DateTime<Utc>,
    morale: f64,
) -> Result<(), AppError> {
    if !morale.is_finite() || morale < 0.0 {
        return Err(AppError::BadRequest(
            "morale must be a non-negative finite number".into(),
        ));
    }
    sqlx::query(
        "INSERT INTO raid_cycle_state (raid_id,clan_code,started_at,raid_started_at,next_reset_at,morale,team_tactics_morale_boost,mirror_force_boost) VALUES ($1,$2,$3,$3,$3,$4,0,0) ON CONFLICT (raid_id) DO UPDATE SET clan_code=EXCLUDED.clan_code,started_at=EXCLUDED.started_at,raid_started_at=EXCLUDED.raid_started_at,next_reset_at=EXCLUDED.next_reset_at,morale=EXCLUDED.morale,team_tactics_morale_boost=0,mirror_force_boost=0,updated_at=NOW()",
    )
    .bind(raid_id)
    .bind(clan_code)
    .bind(start_at)
    .bind(morale)
    .execute(state.db()?)
    .await?;
    Ok(())
}

async fn store_cycle_state(
    state: &Arc<AppState>,
    clan_code: &str,
    raid_id: i64,
    started_at: Option<DateTime<Utc>>,
    raid_started_at: DateTime<Utc>,
    reported_next_reset_at: DateTime<Utc>,
    morale: f64,
    team_tactics_morale_boost: f64,
    mirror_force_boost: f64,
) -> Result<bool, AppError> {
    for (name, value) in [
        ("morale", morale),
        ("TeamTacticsClanMoraleBoost", team_tactics_morale_boost),
        ("MirrorForceBoost", mirror_force_boost),
    ] {
        if !value.is_finite() || value < 0.0 {
            return Err(AppError::BadRequest(format!(
                "{name} must be a non-negative finite number"
            )));
        }
    }
    let boundary_from = started_at.unwrap_or_else(Utc::now);
    let predicted_next_reset_at = next_reset_boundary(raid_started_at, boundary_from);
    if (reported_next_reset_at - predicted_next_reset_at)
        .num_seconds()
        .abs()
        > 300
    {
        tracing::warn!(
            raid_id,
            reported = %reported_next_reset_at,
            predicted = %predicted_next_reset_at,
            "TT2 next_reset_at differs from the 12-hour raid boundary"
        );
    }
    let previous: Option<f64> =
        sqlx::query_scalar("SELECT mirror_force_boost FROM raid_cycle_state WHERE raid_id=$1")
            .bind(raid_id)
            .fetch_optional(state.db()?)
            .await?;
    sqlx::query(
        "INSERT INTO raid_cycle_state (raid_id,clan_code,started_at,raid_started_at,next_reset_at,morale,team_tactics_morale_boost,mirror_force_boost) VALUES ($1,$2,$3,$4,$5,$6,$7,$8) ON CONFLICT (raid_id) DO UPDATE SET clan_code=EXCLUDED.clan_code,started_at=COALESCE(EXCLUDED.started_at,raid_cycle_state.started_at),raid_started_at=EXCLUDED.raid_started_at,next_reset_at=EXCLUDED.next_reset_at,morale=EXCLUDED.morale,team_tactics_morale_boost=EXCLUDED.team_tactics_morale_boost,mirror_force_boost=EXCLUDED.mirror_force_boost,updated_at=NOW()",
    )
    .bind(raid_id)
    .bind(clan_code)
    .bind(started_at)
    .bind(raid_started_at)
    .bind(predicted_next_reset_at)
    .bind(morale)
    .bind(team_tactics_morale_boost)
    .bind(mirror_force_boost)
    .execute(state.db()?)
    .await?;
    Ok(previous.map_or(mirror_force_boost.abs() > 1e-9, |value| {
        (value - mirror_force_boost).abs() > 1e-9
    }))
}

async fn queue_auto_simulations(
    state: &Arc<AppState>,
    phase_change_mask: Option<u8>,
) -> Result<(), AppError> {
    let player_ids: Vec<String> = sqlx::query_scalar(
        "SELECT p.player_id FROM players p WHERE p.auto_sims=TRUE AND EXISTS (SELECT 1 FROM player_stats s WHERE s.player_id=p.player_id)",
    )
    .fetch_all(state.db()?)
    .await?;
    for player_id in player_ids {
        let request = CreateSimulationJobRequest {
            player_id: player_id.clone(),
            include_body_phase: true,
        };
        let result = if let Some(mask) = phase_change_mask {
            job_service::create_phase_aware_job(state, request, mask).await
        } else {
            job_service::create_job(state, request).await
        };
        if let Err(error) = result {
            tracing::error!(
                player_id,
                ?error,
                "could not queue automatic raid simulation"
            );
        }
    }
    Ok(())
}

fn attack_components(attack: &AttackEvent) -> Result<Vec<AttackComponent>, AppError> {
    let levels = attack
        .attack_log
        .cards_level
        .iter()
        .map(|card| (card.id.as_str(), card.value))
        .collect::<HashMap<_, _>>();
    attack
        .attack_log
        .cards_damage
        .iter()
        .map(|component| {
            let total_damage = component
                .damage_log
                .iter()
                .map(|damage| rounded_u64(damage.value, "attack damage"))
                .sum::<Result<u64, AppError>>()?;
            Ok(AttackComponent {
                titan_index: component.titan_index,
                card_level: component
                    .card_id
                    .as_deref()
                    .and_then(|card_id| levels.get(card_id).copied()),
                card_id: component.card_id.clone(),
                total_damage,
            })
        })
        .collect()
}

struct AggregatedCardComponent {
    card_id: String,
    card_level: Option<i32>,
    damage: u64,
}

/// A raid deck is always exactly 3 cards, but the same card can appear as
/// more than one component in a single attack (TT2 splits a hit into
/// separate Body/Armor components when a limb's cursed state flips
/// mid-attack) -- fold those back into one damage total per card, in the
/// order each card first appeared.
fn aggregate_card_components(components: &[AttackComponent]) -> Vec<AggregatedCardComponent> {
    let mut cards: Vec<AggregatedCardComponent> = Vec::with_capacity(3);
    for component in components {
        let Some(card_id) = &component.card_id else {
            continue;
        };
        if let Some(existing) = cards.iter_mut().find(|card| &card.card_id == card_id) {
            existing.damage += component.total_damage;
        } else {
            cards.push(AggregatedCardComponent {
                card_id: card_id.clone(),
                card_level: component.card_level,
                damage: component.total_damage,
            });
        }
    }
    cards
}


fn boss_from_raid_snapshot(
    raid: &RaidSnapshot,
    titan_targets: &[TitanTarget],
    enemy_id: &str,
    preserve_narrow: bool,
) -> Result<(Boss, Vec<BossPartName>), AppError> {
    let titan = raid
        .titans
        .iter()
        .find(|titan| titan.enemy_id == enemy_id)
        .ok_or_else(|| AppError::BadRequest(format!("base raid has no titan {enemy_id}")))?;
    // A brand new raid's first sub_start reports every part as "0" (no
    // target locked in yet) -- and titan_target can be entirely absent from
    // some events. Rather than fail, treat "no selection reported" as
    // "everything is attackable"; a later sub_cycle (or sub_start) with a
    // real selection will narrow this down as soon as it arrives.
    let attackable_parts = titan_targets
        .iter()
        .find(|target| target.enemy_id == enemy_id)
        .map(|target| {
            target
                .state
                .iter()
                .filter(|part| part.state == "2")
                .map(|part| target_part_name(&part.id))
                .collect::<Result<Vec<_>, _>>()
        })
        .transpose()?
        .filter(|parts| !parts.is_empty())
        .unwrap_or_else(|| BossPartName::all().to_vec());

    if raid.area_buffs.len() > 1 || titan.cursed_debuffs.len() > 1 {
        return Err(AppError::BadRequest(
            "Only one global raid modifier and one curse modifier are supported".into(),
        ));
    }
    let (global_modifier, global_amount) = raid
        .area_buffs
        .first()
        .map(|bonus| {
            Ok::<(GlobalRaidModifier, Option<f64>), AppError>((
                global_modifier(&bonus.bonus_type)?,
                Some(bonus.bonus_amount),
            ))
        })
        .transpose()?
        .unwrap_or((GlobalRaidModifier::None, None));
    let (curse_type, curse_amount) = titan
        .cursed_debuffs
        .first()
        .map(|bonus| {
            Ok::<(CurseType, f64), AppError>((
                curse_type(&bonus.bonus_type)?,
                bonus.bonus_amount.abs(),
            ))
        })
        .transpose()?
        .unwrap_or((CurseType::None, 0.06));

    let parts = titan
        .parts
        .iter()
        .map(|part| (part.part_id.as_str(), part))
        .collect::<HashMap<_, _>>();
    let part_json = |name: BossPartName, body_id: &str, armor_id: &str| {
        let body = parts
            .get(body_id)
            .ok_or_else(|| AppError::BadRequest(format!("Missing {body_id}")))?;
        let armor = parts
            .get(armor_id)
            .ok_or_else(|| AppError::BadRequest(format!("Missing {armor_id}")))?;
        let current_armor = rounded_u64(armor.current_hp, armor_id)?;
        let current_health = rounded_u64(body.current_hp, body_id)?;
        let part_state = if current_armor > 0 {
            if armor.cursed {
                PartState::Cursed
            } else {
                PartState::Armor
            }
        } else if current_health > 0 {
            PartState::Body
        } else {
            PartState::Skeleton
        };
        Ok::<Value, AppError>(serde_json::json!({
            "part_name": name,
            "part_state": part_state,
            "max_armor": rounded_u64(armor.total_hp, armor_id)?,
            "max_health": rounded_u64(body.total_hp, body_id)?,
            "current_armor": current_armor,
            "current_health": current_health,
            "radioactivity_afflicted_seconds": 0.0,
        }))
    };
    let boss: Boss = serde_json::from_value(serde_json::json!({
        "boss_name": boss_name(enemy_id, &titan.enemy_name)?,
        "global_raid_modifier": global_modifier,
        "global_raid_modifier_amount": global_amount,
        "curse_type": curse_type,
        "curse_damage_per_curse": curse_amount,
        "recommend_1_to_2_part_patterns_only": preserve_narrow,
        "head": part_json(BossPartName::Head, "BodyHead", "ArmorHead")?,
        "torso": part_json(BossPartName::Torso, "BodyChestUpper", "ArmorChestUpper")?,
        "right_shoulder": part_json(BossPartName::RightShoulder, "BodyArmUpperLeft", "ArmorArmUpperLeft")?,
        "left_shoulder": part_json(BossPartName::LeftShoulder, "BodyArmUpperRight", "ArmorArmUpperRight")?,
        "right_hand": part_json(BossPartName::RightHand, "BodyHandLeft", "ArmorHandLeft")?,
        "left_hand": part_json(BossPartName::LeftHand, "BodyHandRight", "ArmorHandRight")?,
        "right_leg": part_json(BossPartName::RightLeg, "BodyLegUpperLeft", "ArmorLegUpperLeft")?,
        "left_leg": part_json(BossPartName::LeftLeg, "BodyLegUpperRight", "ArmorLegUpperRight")?,
        "damage_results": [],
    }))?;
    Ok((boss, attackable_parts))
}

/// The titan a brand new raid starts on: whichever enemy_name the spawn
/// order lists first (falling back to the first titan definition if the
/// spawn order is empty), always at titan_index 0.
fn first_titan_enemy_id(raid: &RaidSnapshot) -> Result<&str, AppError> {
    raid.spawn_sequence
        .first()
        .and_then(|name| raid.titans.iter().find(|titan| &titan.enemy_name == name))
        .or_else(|| raid.titans.first())
        .map(|titan| titan.enemy_id.as_str())
        .ok_or_else(|| AppError::BadRequest("sub_start raid has no titans".into()))
}

/// Refreshes the sims boss's `attackable_parts` when `titan_target` reveals
/// a real selection for `enemy_id` that differs from what's currently
/// stored -- used when a sub_start for an already-known raid carries target
/// data an earlier one for the same raid didn't have yet. Returns the new
/// boss version if anything actually changed.
async fn update_boss_targets_from_titan_target(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    titan_target: &[TitanTarget],
    enemy_id: &str,
) -> Result<Option<i64>, AppError> {
    let Some(target) = titan_target.iter().find(|target| target.enemy_id == enemy_id) else {
        return Ok(None);
    };
    let attackable_parts = target
        .state
        .iter()
        .filter(|part| part.state == "2")
        .map(|part| target_part_name(&part.id))
        .collect::<Result<Vec<_>, _>>()?;
    if attackable_parts.is_empty() {
        return Ok(None);
    }

    let Some(current) = boss_repo::load_for_update(tx).await? else {
        return Ok(None);
    };
    if current.attackable_parts == attackable_parts {
        return Ok(None);
    }

    let boss_version = boss_repo::store(
        tx,
        boss_repo::BossWrite {
            boss: &current.boss,
            attackable_parts: Some(&attackable_parts),
            source_raid_id: current.source_raid_id,
            source_titan_index: current.source_titan_index,
            source_enemy_id: current.source_enemy_id.as_deref(),
            bump_version: true,
        },
    )
    .await?;
    Ok(Some(boss_version))
}

fn select_base_raid(
    stored_base_raid: Option<Value>,
    has_sub_start: bool,
    sub_cycle_raid: &RaidSnapshot,
) -> Result<(RaidSnapshot, bool), AppError> {
    if !has_sub_start {
        return Ok((sub_cycle_raid.clone(), true));
    }
    let stored_base_raid = stored_base_raid
        .ok_or_else(|| AppError::Conflict("sub_start did not contain base raid data".into()))?;
    Ok((serde_json::from_value(stored_base_raid)?, false))
}

fn preserve_current_boss_values(boss: &mut Boss, current: &Boss) {
    for part_name in BossPartName::all() {
        let current_part = current.part(part_name);
        let part = boss.part_mut(part_name);
        part.current_armor = current_part.current_armor;
        part.current_health = current_part.current_health;
        part.radioactivity_afflicted_seconds = current_part.radioactivity_afflicted_seconds;
        part.sync_state_from_current_values();
    }
}

fn rounded_u64(value: f64, field: &str) -> Result<u64, AppError> {
    if !value.is_finite() || value < 0.0 || value.round() > u64::MAX as f64 {
        return Err(AppError::BadRequest(format!(
            "{field} is not a valid HP/damage value"
        )));
    }
    Ok(value.round() as u64)
}

fn next_reset_boundary(raid_started_at: DateTime<Utc>, after: DateTime<Utc>) -> DateTime<Utc> {
    let interval = chrono::Duration::hours(RESET_INTERVAL_HOURS);
    let elapsed = after
        .signed_duration_since(raid_started_at)
        .num_seconds()
        .max(0);
    let steps = elapsed / interval.num_seconds() + 1;
    raid_started_at + interval * steps as i32
}

fn bonus_value(bonuses: &[RaidBonus], id: &str) -> f64 {
    bonuses
        .iter()
        .find(|bonus| bonus.id == id)
        .map_or(0.0, |bonus| bonus.value)
}

fn boss_name(enemy_id: &str, enemy_name: &str) -> Result<BossName, AppError> {
    let expected = match enemy_id {
        "Enemy1" => BossName::Lojak,
        "Enemy2" => BossName::Takedar,
        "Enemy3" => BossName::Jukk,
        "Enemy4" => BossName::Sterl,
        "Enemy5" => BossName::Mohaca,
        "Enemy6" => BossName::Terro,
        "Enemy7" => BossName::Klonk,
        "Enemy8" => BossName::Priker,
        _ => return Err(AppError::BadRequest(format!("Unknown enemy_id {enemy_id}"))),
    };
    if !enemy_name.is_empty() && enemy_name != format!("{expected:?}") {
        return Err(AppError::BadRequest(format!(
            "Enemy ID {enemy_id} does not match name {enemy_name}"
        )));
    }
    Ok(expected)
}

fn target_part_name(id: &str) -> Result<BossPartName, AppError> {
    match id {
        "Head" => Ok(BossPartName::Head),
        "ChestUpper" => Ok(BossPartName::Torso),
        "ArmUpperRight" => Ok(BossPartName::LeftShoulder),
        "ArmUpperLeft" => Ok(BossPartName::RightShoulder),
        "HandRight" => Ok(BossPartName::LeftHand),
        "HandLeft" => Ok(BossPartName::RightHand),
        "LegUpperRight" => Ok(BossPartName::LeftLeg),
        "LegUpperLeft" => Ok(BossPartName::RightLeg),
        _ => Err(AppError::BadRequest(format!("Unknown titan target {id}"))),
    }
}

fn global_modifier(value: &str) -> Result<GlobalRaidModifier, AppError> {
    match value {
        "BurstDamage" => Ok(GlobalRaidModifier::BurstDamage),
        "BurstChance" => Ok(GlobalRaidModifier::BurstChance),
        "SupportEffect" => Ok(GlobalRaidModifier::SupportEffect),
        "AfflictedChance" | "AfflictionChance" => Ok(GlobalRaidModifier::AfflictionChance),
        "AfflictedDamage" | "AfflictionDamage" => Ok(GlobalRaidModifier::AfflictionDamage),
        "AllDamage" => Ok(GlobalRaidModifier::AllDamage),
        "AttackDuration" => Ok(GlobalRaidModifier::AttackDuration),
        "AfflictedDuration" | "AfflictionDuration" => Ok(GlobalRaidModifier::AfflictionDuration),
        _ => Err(AppError::BadRequest(format!(
            "Unsupported global raid modifier {value}"
        ))),
    }
}

fn curse_type(value: &str) -> Result<CurseType, AppError> {
    match value {
        "BodyDamagePerCurse" => Ok(CurseType::BodyDamage),
        "BurstDamagePerCurse" => Ok(CurseType::BurstDamage),
        "AfflictedDamagePerCurse" | "AfflictionDamagePerCurse" => Ok(CurseType::AfflictionDamage),
        _ => Err(AppError::BadRequest(format!(
            "Unsupported curse modifier {value}"
        ))),
    }
}

#[derive(Debug, Deserialize)]
struct AttackEvent {
    attack_log: AttackLog,
    clan_code: String,
    raid_id: i64,
    player: AttackPlayer,
    raid_state: AttackRaidState,
    cycle: i32,
}

#[derive(Debug, Deserialize)]
struct AttackLog {
    attack_datetime: DateTime<Utc>,
    cards_damage: Vec<AttackCardDamage>,
    cards_level: Vec<AttackCardLevel>,
}

#[derive(Debug, Deserialize)]
struct AttackCardDamage {
    titan_index: i32,
    #[serde(rename = "id")]
    card_id: Option<String>,
    damage_log: Vec<AttackPartDamage>,
}

#[derive(Debug, Deserialize)]
struct AttackPartDamage {
    value: f64,
}

#[derive(Debug, Deserialize)]
struct AttackCardLevel {
    id: String,
    value: i32,
}

#[derive(Debug, Deserialize)]
struct AttackPlayer {
    player_code: String,
    name: String,
}

#[derive(Debug, Deserialize)]
struct StartAttackEvent {
    player: AttackPlayer,
    cards: Vec<String>,
    started_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
struct AttackRaidState {
    current: AttackCurrentBoss,
    titan_index: i32,
}

#[derive(Debug, Deserialize, serde::Serialize)]
struct AttackCurrentBoss {
    enemy_id: String,
    current_hp: f64,
    parts: Vec<AttackCurrentBossPart>,
}

#[derive(Debug, Deserialize, serde::Serialize)]
struct AttackCurrentBossPart {
    part_id: String,
    current_hp: f64,
}

struct AttackComponent {
    titan_index: i32,
    card_id: Option<String>,
    card_level: Option<i32>,
    total_damage: u64,
}

#[derive(Debug, Deserialize)]
struct CycleResetEvent {
    clan_code: String,
    raid_id: i64,
    next_reset_at: DateTime<Utc>,
    #[serde(default)]
    card_bonuses: Vec<RaidBonus>,
    morale: RaidMorale,
    raid_started_at: DateTime<Utc>,
    started_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
struct SubStartEvent {
    clan_code: String,
    raid_id: i64,
    morale: RaidMorale,
    raid: RaidSnapshot,
    start_at: DateTime<Utc>,
    /// Usually all "0" (no target locked in yet) on the very first sub_start
    /// of a raid, but a later sub_start (e.g. a reconnect resend) can carry
    /// real target selections -- see `handle_sub_start`.
    #[serde(default)]
    titan_target: Vec<TitanTarget>,
}

#[derive(Debug, Deserialize)]
struct SubCycleEvent {
    clan_code: String,
    raid_id: i64,
    next_reset_at: DateTime<Utc>,
    #[serde(default)]
    card_bonuses: Vec<RaidBonus>,
    morale: RaidMorale,
    raid_started_at: DateTime<Utc>,
    raid: RaidSnapshot,
    titan_target: Vec<TitanTarget>,
}

#[derive(Debug, Deserialize)]
struct RaidMorale {
    bonus_amount: f64,
}

#[derive(Debug, Deserialize)]
struct RaidBonus {
    id: String,
    value: f64,
}

#[derive(Debug, Clone, Deserialize, serde::Serialize)]
struct RaidSnapshot {
    #[serde(default)]
    spawn_sequence: Vec<String>,
    titans: Vec<RaidTitan>,
    #[serde(default)]
    area_buffs: Vec<TitanBonus>,
    #[serde(flatten)]
    extra: HashMap<String, Value>,
}

#[derive(Debug, Clone, Deserialize, serde::Serialize)]
struct RaidTitan {
    enemy_id: String,
    enemy_name: String,
    parts: Vec<RaidTitanPart>,
    #[serde(default)]
    cursed_debuffs: Vec<TitanBonus>,
    #[serde(flatten)]
    extra: HashMap<String, Value>,
}

#[derive(Debug, Clone, Deserialize, serde::Serialize)]
struct RaidTitanPart {
    part_id: String,
    current_hp: f64,
    total_hp: f64,
    #[serde(default)]
    cursed: bool,
}

#[derive(Debug, Clone, Deserialize, serde::Serialize)]
struct TitanBonus {
    bonus_type: String,
    bonus_amount: f64,
}

#[derive(Debug, Deserialize, serde::Serialize)]
struct TitanTarget {
    enemy_id: String,
    state: Vec<TitanTargetPart>,
}

#[derive(Debug, Deserialize, serde::Serialize)]
struct TitanTargetPart {
    id: String,
    state: String,
}

#[cfg(test)]
#[path = "../../tests/unit/services/raid_event_service_tests.rs"]
mod tests;
