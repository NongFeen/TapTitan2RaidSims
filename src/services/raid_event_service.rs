use std::{collections::HashMap, str::FromStr, sync::Arc, time::Duration};

use chrono::{DateTime, Utc};
use serde::Deserialize;
use serde_json::Value;
use uuid::Uuid;

use crate::{
    error::AppError,
    models::{
        app::CreateSimulationJobRequest,
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

pub fn recover_pending_refresh(state: Arc<AppState>) {
    if state.optional_db().is_none() {
        return;
    }
    tokio::spawn(async move {
        let required = sqlx::query_scalar::<_, bool>(
            "SELECT EXISTS(SELECT 1 FROM raid_current_state WHERE refresh_required=TRUE)",
        )
        .fetch_one(match state.db() {
            Ok(db) => db,
            Err(_) => return,
        })
        .await
        .unwrap_or(false);
        if required {
            refresh_subscription(state).await;
        }
    });
}

async fn handle_attack(
    state: &Arc<AppState>,
    attack: AttackEvent,
    raw_payload: Value,
) -> Result<(), AppError> {
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

    let previous_index: Option<i32> =
        sqlx::query_scalar("SELECT resulting_titan_index FROM raid_current_state WHERE raid_id=$1")
            .bind(attack.raid_id)
            .fetch_optional(&mut *tx)
            .await?
            .flatten();
    let transition = previous_index.is_none_or(|index| attack.raid_state.titan_index > index);

    sqlx::query(
        "INSERT INTO raid_current_state (raid_id,clan_code,resulting_titan_index,current_enemy_id,refresh_required) VALUES ($1,$2,$3,$4,$5) ON CONFLICT (raid_id) DO UPDATE SET clan_code=EXCLUDED.clan_code, resulting_titan_index=CASE WHEN EXCLUDED.resulting_titan_index >= COALESCE(raid_current_state.resulting_titan_index,-1) THEN EXCLUDED.resulting_titan_index ELSE raid_current_state.resulting_titan_index END, current_enemy_id=CASE WHEN EXCLUDED.resulting_titan_index >= COALESCE(raid_current_state.resulting_titan_index,-1) THEN EXCLUDED.current_enemy_id ELSE raid_current_state.current_enemy_id END, refresh_required=raid_current_state.refresh_required OR EXCLUDED.refresh_required, updated_at=NOW()",
    )
    .bind(attack.raid_id)
    .bind(&attack.clan_code)
    .bind(attack.raid_state.titan_index)
    .bind(&attack.raid_state.current.enemy_id)
    .bind(transition)
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

    if transition {
        let refresh_state = Arc::clone(state);
        tokio::spawn(async move { refresh_subscription(refresh_state).await });
    }
    Ok(())
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

async fn refresh_subscription(state: Arc<AppState>) {
    let _guard = state.raid_refresh_lock.lock().await;
    for attempt in 1..=3u64 {
        let required = sqlx::query_scalar::<_, bool>(
            "SELECT EXISTS(SELECT 1 FROM raid_current_state WHERE refresh_required=TRUE)",
        )
        .fetch_one(match state.db() {
            Ok(db) => db,
            Err(_) => return,
        })
        .await
        .unwrap_or(false);
        if !required {
            return;
        }
        let result = match state.gamehive_api.as_ref() {
            Some(client) => client.resubscribe_raid(&state).await,
            None => Err(AppError::ServiceUnavailable(
                "TT2 integration is not configured".into(),
            )),
        };
        match result {
            Ok(()) => tracing::info!(attempt, "TT2 raid unsubscribe/subscribe completed"),
            Err(error) => tracing::warn!(attempt, ?error, "TT2 raid refresh failed"),
        }
        tokio::time::sleep(Duration::from_secs(attempt * 2)).await;
    }
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
        "right_shoulder": part_json(BossPartName::RightShoulder, "BodyArmUpperRight", "ArmorArmUpperRight")?,
        "left_shoulder": part_json(BossPartName::LeftShoulder, "BodyArmUpperLeft", "ArmorArmUpperLeft")?,
        "right_hand": part_json(BossPartName::RightHand, "BodyHandRight", "ArmorHandRight")?,
        "left_hand": part_json(BossPartName::LeftHand, "BodyHandLeft", "ArmorHandLeft")?,
        "right_leg": part_json(BossPartName::RightLeg, "BodyLegUpperRight", "ArmorLegUpperRight")?,
        "left_leg": part_json(BossPartName::LeftLeg, "BodyLegUpperLeft", "ArmorLegUpperLeft")?,
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
        "ArmUpperRight" => Ok(BossPartName::RightShoulder),
        "ArmUpperLeft" => Ok(BossPartName::LeftShoulder),
        "HandRight" => Ok(BossPartName::RightHand),
        "HandLeft" => Ok(BossPartName::LeftHand),
        "LegUpperRight" => Ok(BossPartName::RightLeg),
        "LegUpperLeft" => Ok(BossPartName::LeftLeg),
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

#[derive(Debug, Deserialize)]
struct AttackCurrentBoss {
    enemy_id: String,
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
                include_str!("../../attackdatajson/beforebosstrans.json"),
                449_811_765,
                1,
            ),
            (
                include_str!("../../attackdatajson/bosstrans.json"),
                835_323_242,
                1,
            ),
            (
                include_str!("../../attackdatajson/afterbosstrans.json"),
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
    fn sub_cycle_uses_part_totals_targets_and_live_modifiers() {
        let event: SubCycleEvent =
            serde_json::from_str(include_str!("../../sub_cycle_example.json")).unwrap();
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
    }

    #[test]
    fn cycle_sample_produces_expected_percentages_and_reset_boundary() {
        let event: CycleResetEvent =
            serde_json::from_str(include_str!("../../cycle_reset_example.json")).unwrap();
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
