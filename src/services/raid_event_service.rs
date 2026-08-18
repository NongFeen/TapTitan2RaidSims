use std::{collections::HashMap, str::FromStr, sync::Arc};

use chrono::{DateTime, Utc};
use serde::Deserialize;
use serde_json::Value;
use uuid::Uuid;

use crate::{
    error::AppError,
    models::{
        app::{CreateSimulationJobRequest, LiveAttackBossView},
        boss::{Boss, BossName, BossPartName, CurseType, GlobalRaidModifier, PartState},
        cards::CardName,
    },
    services::job_service,
    state::AppState,
};

const RAID_STATE_LOCK: i64 = 721_934_762;
const RESET_INTERVAL_HOURS: i64 = 12;

pub async fn handle_event(state: &Arc<AppState>, event: &str, data: Value) -> Result<(), AppError> {
    match event {
        "attack" => handle_attack(state, serde_json::from_value(data.clone())?, data).await,
        "sub_cycle" => handle_sub_cycle(state, serde_json::from_value(data.clone())?, data).await,
        "cycle_reset" => handle_cycle_reset(state, serde_json::from_value(data)?).await,
        _ => Ok(()),
    }
}

async fn handle_attack(
    state: &Arc<AppState>,
    attack: AttackEvent,
    raw_payload: Value,
) -> Result<(), AppError> {
    *state.live_attack_boss.write().await = Some(LiveAttackBossView {
        clan_code: attack.clan_code.clone(),
        raid_id: attack.raid_id,
        cycle: attack.cycle,
        titan_index: attack.raid_state.titan_index,
        boss_data: serde_json::to_value(&attack.raid_state.current)?,
        received_at: Utc::now(),
    });
    tracing::info!(
        raid_id = attack.raid_id,
        cycle = attack.cycle,
        titan_index = attack.raid_state.titan_index,
        enemy_id = %attack.raid_state.current.enemy_id,
        "updated live current boss from TT2 attack event"
    );
    let raw_payload = without_live_boss_data(raw_payload);
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

    let attack_id = Uuid::new_v4();
    let inserted: Option<Uuid> = sqlx::query_scalar(
        "INSERT INTO raid_attack_logs (id,raid_id,clan_code,player_id,player_code,player_name,cycle,attack_datetime,attacked_titan_index,resulting_titan_index,enemy_id,tap_damage,total_damage,raw_payload) VALUES ($1,$2,$3,(SELECT id FROM players WHERE player_id=$4),$4,$5,$6,$7,$8,$9,$10,CAST($11 AS NUMERIC),CAST($12 AS NUMERIC),$13) ON CONFLICT (raid_id,player_code,attack_datetime) DO NOTHING RETURNING id",
    )
    .bind(attack_id)
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
    .bind(raw_payload)
    .fetch_optional(&mut *tx)
    .await?;

    if inserted.is_none() {
        tx.commit().await?;
        sync_sims_boss_on_body_transition(state, &attack).await?;
        return Ok(());
    }

    for (position, component) in components.iter().enumerate() {
        sqlx::query(
            "INSERT INTO raid_attack_components (attack_log_id,position,component_kind,card_id,card_name,card_level,total_damage,part_damage) VALUES ($1,$2,$3,$4,$5,$6,CAST($7 AS NUMERIC),$8)",
        )
        .bind(attack_id)
        .bind(position as i32)
        .bind(if component.card_id.is_some() { "card" } else { "tap" })
        .bind(&component.card_id)
        .bind(&component.card_name)
        .bind(component.card_level)
        .bind(component.total_damage.to_string())
        .bind(&component.part_damage)
        .execute(&mut *tx)
        .await?;
    }

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
        "stored TT2 raid attack"
    );
    sync_sims_boss_on_body_transition(state, &attack).await?;
    Ok(())
}

async fn sync_sims_boss_on_body_transition(
    state: &Arc<AppState>,
    attack: &AttackEvent,
) -> Result<bool, AppError> {
    let mut tx = state.db()?.begin().await?;
    sqlx::query("SELECT pg_advisory_xact_lock($1)")
        .bind(RAID_STATE_LOCK)
        .execute(&mut *tx)
        .await?;

    let current: Option<(Value, Value, i64, Option<i64>, Option<i32>, Option<String>)> =
        sqlx::query_as(
            "SELECT boss_data,attackable_parts,version,source_raid_id,source_titan_index,source_enemy_id FROM current_boss WHERE singleton=TRUE FOR UPDATE",
        )
        .fetch_optional(&mut *tx)
        .await?;
    let Some((boss_json, targets_json, _, source_raid_id, source_titan_index, source_enemy_id)) =
        current
    else {
        return Ok(false);
    };
    let mut boss: Boss = serde_json::from_value(boss_json)?;
    let targets: Vec<BossPartName> = serde_json::from_value(targets_json)?;
    let expected_boss = boss_name(&attack.raid_state.current.enemy_id, "")?;
    let source_matches = source_enemy_id
        .as_deref()
        .map_or(boss.boss_name == expected_boss, |enemy_id| {
            enemy_id == attack.raid_state.current.enemy_id
        })
        && source_raid_id.is_none_or(|raid_id| raid_id == attack.raid_id)
        && source_titan_index
            .is_none_or(|titan_index| titan_index == attack.raid_state.titan_index);
    if !source_matches
        || !apply_attack_body_transition(&mut boss, &targets, &attack.raid_state.current)?
    {
        return Ok(false);
    }

    let boss_version: i64 = sqlx::query_scalar(
        "UPDATE current_boss SET version=version+1,boss_data=$1,source_raid_id=$2,source_titan_index=$3,source_enemy_id=$4,updated_at=NOW() WHERE singleton=TRUE RETURNING version",
    )
    .bind(serde_json::to_value(&boss)?)
    .bind(attack.raid_id)
    .bind(attack.raid_state.titan_index)
    .bind(&attack.raid_state.current.enemy_id)
    .fetch_one(&mut *tx)
    .await?;
    tx.commit().await?;

    job_service::spawn_old_job_cleanup(Arc::clone(state), boss_version);
    queue_auto_simulations(state).await?;
    tracing::info!(
        raid_id = attack.raid_id,
        titan_index = attack.raid_state.titan_index,
        enemy_id = %attack.raid_state.current.enemy_id,
        boss_version,
        target_count = targets.len(),
        "all targeted armor became body; updated sims boss and queued auto simulations"
    );
    Ok(true)
}

fn apply_attack_body_transition(
    boss: &mut Boss,
    targets: &[BossPartName],
    live: &AttackCurrentBoss,
) -> Result<bool, AppError> {
    if targets.is_empty()
        || !targets.iter().any(|part_name| {
            matches!(
                boss.part(*part_name).part_state,
                PartState::Armor | PartState::Cursed
            )
        })
    {
        return Ok(false);
    }

    let target_values = targets
        .iter()
        .map(|part_name| attack_part_values(live, *part_name))
        .collect::<Result<Option<Vec<_>>, AppError>>()?;
    let Some(target_values) = target_values else {
        return Ok(false);
    };
    if !target_values
        .iter()
        .all(|(current_armor, current_health)| *current_armor == 0 && *current_health > 0)
    {
        return Ok(false);
    }

    for part_name in BossPartName::all() {
        if let Some((current_armor, current_health)) = attack_part_values(live, part_name)? {
            let part = boss.part_mut(part_name);
            part.current_armor = current_armor;
            part.current_health = current_health;
        }
    }
    boss.sync_part_states_from_current_values();
    Ok(true)
}

fn attack_part_values(
    live: &AttackCurrentBoss,
    part_name: BossPartName,
) -> Result<Option<(u64, u64)>, AppError> {
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
    let body = live.parts.iter().find(|part| part.part_id == body_id);
    let armor = live.parts.iter().find(|part| part.part_id == armor_id);
    match (armor, body) {
        (Some(armor), Some(body)) => Ok(Some((
            rounded_u64(armor.current_hp, armor_id)?,
            rounded_u64(body.current_hp, body_id)?,
        ))),
        _ => Ok(None),
    }
}

async fn handle_sub_cycle(
    state: &Arc<AppState>,
    event: SubCycleEvent,
    raw_payload: Value,
) -> Result<(), AppError> {
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

    let runtime: Option<(Option<i32>, Option<String>)> = sqlx::query_as(
        "SELECT resulting_titan_index,current_enemy_id FROM raid_current_state WHERE raid_id=$1",
    )
    .bind(event.raid_id)
    .fetch_optional(state.db()?)
    .await?;
    let (titan_index, enemy_id) = runtime.ok_or_else(|| {
        AppError::Conflict("sub_cycle arrived before an attack identified the current titan".into())
    })?;
    let enemy_id = enemy_id
        .ok_or_else(|| AppError::Conflict("Current raid state has no enemy_id".to_string()))?;
    let titan_index = titan_index.unwrap_or_default();

    let previous_boss: Option<(Value, Option<String>)> =
        sqlx::query_as("SELECT boss_data,source_enemy_id FROM current_boss WHERE singleton=TRUE")
            .fetch_optional(state.db()?)
            .await?;
    let preserve_narrow = previous_boss
        .as_ref()
        .and_then(|(boss, _)| boss.get("recommend_1_to_2_part_patterns_only"))
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let (boss, attackable_parts) = boss_from_sub_cycle(&event, &enemy_id, preserve_narrow)?;
    let enemy_changed = previous_boss
        .as_ref()
        .and_then(|(_, enemy)| enemy.as_deref())
        != Some(enemy_id.as_str());

    let mut tx = state.db()?.begin().await?;
    sqlx::query("SELECT pg_advisory_xact_lock($1)")
        .bind(RAID_STATE_LOCK)
        .execute(&mut *tx)
        .await?;
    sqlx::query(
        "UPDATE raid_current_state SET clan_code=$2,raid_data=$3,titan_targets=$4,raw_sub_cycle=$5,received_at=NOW(),refresh_required=FALSE,updated_at=NOW() WHERE raid_id=$1",
    )
    .bind(event.raid_id)
    .bind(&event.clan_code)
    .bind(serde_json::to_value(&event.raid)?)
    .bind(serde_json::to_value(&event.titan_target)?)
    .bind(raw_payload)
    .execute(&mut *tx)
    .await?;

    let boss_json = serde_json::to_value(&boss)?;
    let targets_json = serde_json::to_value(&attackable_parts)?;
    let boss_version: i64 = if enemy_changed {
        sqlx::query_scalar(
            "INSERT INTO current_boss (singleton,version,boss_data,attackable_parts,source_raid_id,source_titan_index,source_enemy_id) VALUES (TRUE,1,$1,$2,$3,$4,$5) ON CONFLICT (singleton) DO UPDATE SET version=current_boss.version+1,boss_data=EXCLUDED.boss_data,attackable_parts=EXCLUDED.attackable_parts,source_raid_id=EXCLUDED.source_raid_id,source_titan_index=EXCLUDED.source_titan_index,source_enemy_id=EXCLUDED.source_enemy_id,updated_at=NOW() RETURNING version",
        )
        .bind(boss_json)
        .bind(targets_json)
        .bind(event.raid_id)
        .bind(titan_index)
        .bind(&enemy_id)
        .fetch_one(&mut *tx)
        .await?
    } else {
        sqlx::query_scalar(
            "UPDATE current_boss SET boss_data=$1,attackable_parts=$2,source_raid_id=$3,source_titan_index=$4,source_enemy_id=$5,updated_at=NOW() WHERE singleton=TRUE RETURNING version",
        )
        .bind(boss_json)
        .bind(targets_json)
        .bind(event.raid_id)
        .bind(titan_index)
        .bind(&enemy_id)
        .fetch_one(&mut *tx)
        .await?
    };
    tx.commit().await?;

    if enemy_changed {
        job_service::spawn_old_job_cleanup(Arc::clone(state), boss_version);
    }
    if enemy_changed || mirror_changed {
        queue_auto_simulations(state).await?;
    }
    tracing::info!(
        event.raid_id,
        titan_index,
        enemy_id,
        enemy_changed,
        mirror_changed,
        "synchronized current boss from sub_cycle"
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
        queue_auto_simulations(state).await?;
    }
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

async fn queue_auto_simulations(state: &Arc<AppState>) -> Result<(), AppError> {
    let player_ids: Vec<String> = sqlx::query_scalar(
        "SELECT p.player_id FROM players p WHERE p.auto_sims=TRUE AND EXISTS (SELECT 1 FROM player_stats s WHERE s.player_id=p.id)",
    )
    .fetch_all(state.db()?)
    .await?;
    for player_id in player_ids {
        if let Err(error) = job_service::create_job(
            state,
            CreateSimulationJobRequest {
                player_id: player_id.clone(),
                include_body_phase: true,
            },
        )
        .await
        {
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
            let part_damage = component
                .damage_log
                .iter()
                .map(|damage| {
                    Ok(serde_json::json!({
                        "part_id": damage.id,
                        "damage": rounded_u64(damage.value, "attack damage")?,
                    }))
                })
                .collect::<Result<Vec<_>, AppError>>()?;
            let total_damage = part_damage
                .iter()
                .filter_map(|damage| damage.get("damage").and_then(Value::as_u64))
                .sum();
            let card_name = component.card_id.as_deref().map_or_else(
                || "Tap".to_string(),
                |card_id| {
                    CardName::from_str(card_id).map_or_else(
                        |_| card_id.to_string(),
                        |card| card.display_name().to_string(),
                    )
                },
            );
            Ok(AttackComponent {
                titan_index: component.titan_index,
                card_level: component
                    .card_id
                    .as_deref()
                    .and_then(|card_id| levels.get(card_id).copied()),
                card_id: component.card_id.clone(),
                card_name,
                total_damage,
                part_damage: Value::Array(part_damage),
            })
        })
        .collect()
}

fn without_live_boss_data(mut payload: Value) -> Value {
    if let Some(raid_state) = payload
        .as_object_mut()
        .and_then(|root| root.get_mut("raid_state"))
        .and_then(Value::as_object_mut)
    {
        raid_state.remove("current");
    }
    payload
}

fn boss_from_sub_cycle(
    event: &SubCycleEvent,
    enemy_id: &str,
    preserve_narrow: bool,
) -> Result<(Boss, Vec<BossPartName>), AppError> {
    let titan = event
        .raid
        .titans
        .iter()
        .find(|titan| titan.enemy_id == enemy_id)
        .ok_or_else(|| AppError::BadRequest(format!("sub_cycle has no titan {enemy_id}")))?;
    let target = event
        .titan_target
        .iter()
        .find(|target| target.enemy_id == enemy_id)
        .ok_or_else(|| {
            AppError::BadRequest(format!("sub_cycle has no target data for {enemy_id}"))
        })?;
    let attackable_parts = target
        .state
        .iter()
        .filter(|part| part.state == "2")
        .map(|part| target_part_name(&part.id))
        .collect::<Result<Vec<_>, _>>()?;
    if attackable_parts.is_empty() {
        return Err(AppError::BadRequest(
            "sub_cycle selected no attackable titan parts".into(),
        ));
    }

    if event.raid.area_buffs.len() > 1 || titan.cursed_debuffs.len() > 1 {
        return Err(AppError::BadRequest(
            "Only one global raid modifier and one curse modifier are supported".into(),
        ));
    }
    let (global_modifier, global_amount) = event
        .raid
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
    id: String,
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
    card_name: String,
    card_level: Option<i32>,
    total_damage: u64,
    part_damage: Value,
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

#[derive(Debug, Deserialize, serde::Serialize)]
struct RaidSnapshot {
    titans: Vec<RaidTitan>,
    #[serde(default)]
    area_buffs: Vec<TitanBonus>,
    #[serde(flatten)]
    extra: HashMap<String, Value>,
}

#[derive(Debug, Deserialize, serde::Serialize)]
struct RaidTitan {
    enemy_id: String,
    enemy_name: String,
    parts: Vec<RaidTitanPart>,
    #[serde(default)]
    cursed_debuffs: Vec<TitanBonus>,
    #[serde(flatten)]
    extra: HashMap<String, Value>,
}

#[derive(Debug, Deserialize, serde::Serialize)]
struct RaidTitanPart {
    part_id: String,
    current_hp: f64,
    total_hp: f64,
    #[serde(default)]
    cursed: bool,
}

#[derive(Debug, Deserialize, serde::Serialize)]
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
mod tests {
    use super::*;

    #[test]
    fn supplied_attack_samples_have_expected_totals_and_transition_indices() {
        let cases = [
            (
                include_str!("../../exampleSocketdatajson/beforebosstrans.json"),
                449_811_765,
                1,
            ),
            (
                include_str!("../../exampleSocketdatajson/bosstrans.json"),
                835_323_242,
                1,
            ),
            (
                include_str!("../../exampleSocketdatajson/afterbosstrans.json"),
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
            "../../exampleSocketdatajson/afterbosstrans.json"
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
            "../../exampleSocketdatajson/afterbosstrans.json"
        ))
        .unwrap();

        assert!(handle_event(&state, "attack", raw).await.is_err());

        let live = state.live_attack_boss.read().await.clone().unwrap();
        assert_eq!(live.titan_index, 2);
        assert_eq!(live.boss_data["enemy_id"], "Enemy8");
        assert_eq!(live.boss_data["current_hp"], 107_028_000_000.0);
    }

    #[test]
    fn sub_cycle_uses_part_totals_targets_and_live_modifiers() {
        let event: SubCycleEvent = serde_json::from_str(include_str!(
            "../../exampleSocketdatajson/sub_cycle_example.json"
        ))
        .unwrap();
        let (boss, targets) = boss_from_sub_cycle(&event, "Enemy8", false).unwrap();
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
    fn attack_updates_sims_boss_once_when_every_target_becomes_body() {
        let event: SubCycleEvent = serde_json::from_str(include_str!(
            "../../exampleSocketdatajson/sub_cycle_example.json"
        ))
        .unwrap();
        let (mut boss, _) = boss_from_sub_cycle(&event, "Enemy8", false).unwrap();
        let targets = [BossPartName::Head, BossPartName::Torso];
        let live = body_phase_attack_boss(0);

        assert!(apply_attack_body_transition(&mut boss, &targets, &live).unwrap());
        assert_eq!(boss.head.part_state, PartState::Body);
        assert_eq!(boss.head.current_armor, 0);
        assert_eq!(boss.head.current_health, 15_000_000_000);
        assert_eq!(boss.torso.part_state, PartState::Body);

        assert!(!apply_attack_body_transition(&mut boss, &targets, &live).unwrap());
    }

    #[test]
    fn attack_waits_until_every_target_is_body() {
        let event: SubCycleEvent = serde_json::from_str(include_str!(
            "../../exampleSocketdatajson/sub_cycle_example.json"
        ))
        .unwrap();
        let (mut boss, _) = boss_from_sub_cycle(&event, "Enemy8", false).unwrap();
        let targets = [BossPartName::Head, BossPartName::Torso];

        assert!(
            !apply_attack_body_transition(&mut boss, &targets, &body_phase_attack_boss(1)).unwrap()
        );
        assert_ne!(boss.head.current_armor, 0);
    }

    fn body_phase_attack_boss(torso_armor: u64) -> AttackCurrentBoss {
        AttackCurrentBoss {
            enemy_id: "Enemy8".to_string(),
            current_hp: 30_000_000_000.0,
            parts: vec![
                AttackCurrentBossPart {
                    part_id: "BodyHead".to_string(),
                    current_hp: 15_000_000_000.0,
                },
                AttackCurrentBossPart {
                    part_id: "ArmorHead".to_string(),
                    current_hp: 0.0,
                },
                AttackCurrentBossPart {
                    part_id: "BodyChestUpper".to_string(),
                    current_hp: 15_000_000_000.0,
                },
                AttackCurrentBossPart {
                    part_id: "ArmorChestUpper".to_string(),
                    current_hp: torso_armor as f64,
                },
            ],
        }
    }

    #[test]
    fn cycle_sample_produces_expected_percentages_and_reset_boundary() {
        let event: CycleResetEvent = serde_json::from_str(include_str!(
            "../../exampleSocketdatajson/cycle_reset_example.json"
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
}
