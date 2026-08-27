use chrono::{DateTime, Utc};
use serde_json::{Value, json};
use sqlx::{PgExecutor, Postgres, Transaction};

use crate::{
    error::AppError,
    models::boss::{Boss, BossName, BossPartName, CurseType, GlobalRaidModifier, PartState},
};

pub struct LoadedBoss {
    pub version: i64,
    pub boss: Boss,
    pub attackable_parts: Vec<BossPartName>,
    pub source_raid_id: Option<i64>,
    pub source_titan_index: Option<i32>,
    pub source_enemy_id: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(sqlx::FromRow)]
struct CurrentBossRow {
    version: i64,
    boss_name: BossName,
    global_raid_modifier: GlobalRaidModifier,
    global_raid_modifier_amount: Option<f64>,
    curse_type: CurseType,
    curse_damage_per_curse: f64,
    recommend_1_to_2_part_patterns_only: bool,
    damage_results: Value,
    source_raid_id: Option<i64>,
    source_titan_index: Option<i32>,
    source_enemy_id: Option<String>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
    head_part_state: PartState,
    head_max_armor: i64,
    head_max_health: i64,
    head_current_armor: i64,
    head_current_health: i64,
    head_is_attackable: bool,
    torso_part_state: PartState,
    torso_max_armor: i64,
    torso_max_health: i64,
    torso_current_armor: i64,
    torso_current_health: i64,
    torso_is_attackable: bool,
    left_shoulder_part_state: PartState,
    left_shoulder_max_armor: i64,
    left_shoulder_max_health: i64,
    left_shoulder_current_armor: i64,
    left_shoulder_current_health: i64,
    left_shoulder_is_attackable: bool,
    right_shoulder_part_state: PartState,
    right_shoulder_max_armor: i64,
    right_shoulder_max_health: i64,
    right_shoulder_current_armor: i64,
    right_shoulder_current_health: i64,
    right_shoulder_is_attackable: bool,
    left_hand_part_state: PartState,
    left_hand_max_armor: i64,
    left_hand_max_health: i64,
    left_hand_current_armor: i64,
    left_hand_current_health: i64,
    left_hand_is_attackable: bool,
    right_hand_part_state: PartState,
    right_hand_max_armor: i64,
    right_hand_max_health: i64,
    right_hand_current_armor: i64,
    right_hand_current_health: i64,
    right_hand_is_attackable: bool,
    left_leg_part_state: PartState,
    left_leg_max_armor: i64,
    left_leg_max_health: i64,
    left_leg_current_armor: i64,
    left_leg_current_health: i64,
    left_leg_is_attackable: bool,
    right_leg_part_state: PartState,
    right_leg_max_armor: i64,
    right_leg_max_health: i64,
    right_leg_current_armor: i64,
    right_leg_current_health: i64,
    right_leg_is_attackable: bool,
}

const CURRENT_BOSS_COLUMNS: &str = "version, boss_name, global_raid_modifier, \
    global_raid_modifier_amount, curse_type, curse_damage_per_curse, \
    recommend_1_to_2_part_patterns_only, damage_results, \
    source_raid_id, source_titan_index, source_enemy_id, created_at, updated_at, \
    head_part_state, head_max_armor, head_max_health, head_current_armor, head_current_health, head_is_attackable, \
    torso_part_state, torso_max_armor, torso_max_health, torso_current_armor, torso_current_health, torso_is_attackable, \
    left_shoulder_part_state, left_shoulder_max_armor, left_shoulder_max_health, left_shoulder_current_armor, left_shoulder_current_health, left_shoulder_is_attackable, \
    right_shoulder_part_state, right_shoulder_max_armor, right_shoulder_max_health, right_shoulder_current_armor, right_shoulder_current_health, right_shoulder_is_attackable, \
    left_hand_part_state, left_hand_max_armor, left_hand_max_health, left_hand_current_armor, left_hand_current_health, left_hand_is_attackable, \
    right_hand_part_state, right_hand_max_armor, right_hand_max_health, right_hand_current_armor, right_hand_current_health, right_hand_is_attackable, \
    left_leg_part_state, left_leg_max_armor, left_leg_max_health, left_leg_current_armor, left_leg_current_health, left_leg_is_attackable, \
    right_leg_part_state, right_leg_max_armor, right_leg_max_health, right_leg_current_armor, right_leg_current_health, right_leg_is_attackable";

/// Rebuild the exact `boss_data` JSON shape the old JSONB column held, then
/// reuse `Boss`'s existing (already-correct) `Deserialize` impl to construct
/// it -- `Boss` has a private field (`initial_cursed_part_count`) that a
/// struct literal from outside its module can't set directly.
/// `radioactivity_afflicted_seconds` isn't persisted (see migration 0033) --
/// `BossPart`'s `#[serde(default)]` on that field fills it in as 0.0, which
/// matches its real value: it's a per-simulation-run tick accumulator that
/// never survives past the single simulation that produced it.
fn boss_from_row(row: &CurrentBossRow) -> Result<Boss, AppError> {
    let value = json!({
        "boss_name": row.boss_name,
        "global_raid_modifier": row.global_raid_modifier,
        "global_raid_modifier_amount": row.global_raid_modifier_amount,
        "curse_type": row.curse_type,
        "curse_damage_per_curse": row.curse_damage_per_curse,
        "recommend_1_to_2_part_patterns_only": row.recommend_1_to_2_part_patterns_only,
        "head": json!({
            "part_name": "Head",
            "part_state": row.head_part_state,
            "max_armor": row.head_max_armor,
            "max_health": row.head_max_health,
            "current_armor": row.head_current_armor,
            "current_health": row.head_current_health,
        }),
        "torso": json!({
            "part_name": "Torso",
            "part_state": row.torso_part_state,
            "max_armor": row.torso_max_armor,
            "max_health": row.torso_max_health,
            "current_armor": row.torso_current_armor,
            "current_health": row.torso_current_health,
        }),
        "left_shoulder": json!({
            "part_name": "LeftShoulder",
            "part_state": row.left_shoulder_part_state,
            "max_armor": row.left_shoulder_max_armor,
            "max_health": row.left_shoulder_max_health,
            "current_armor": row.left_shoulder_current_armor,
            "current_health": row.left_shoulder_current_health,
        }),
        "right_shoulder": json!({
            "part_name": "RightShoulder",
            "part_state": row.right_shoulder_part_state,
            "max_armor": row.right_shoulder_max_armor,
            "max_health": row.right_shoulder_max_health,
            "current_armor": row.right_shoulder_current_armor,
            "current_health": row.right_shoulder_current_health,
        }),
        "left_hand": json!({
            "part_name": "LeftHand",
            "part_state": row.left_hand_part_state,
            "max_armor": row.left_hand_max_armor,
            "max_health": row.left_hand_max_health,
            "current_armor": row.left_hand_current_armor,
            "current_health": row.left_hand_current_health,
        }),
        "right_hand": json!({
            "part_name": "RightHand",
            "part_state": row.right_hand_part_state,
            "max_armor": row.right_hand_max_armor,
            "max_health": row.right_hand_max_health,
            "current_armor": row.right_hand_current_armor,
            "current_health": row.right_hand_current_health,
        }),
        "left_leg": json!({
            "part_name": "LeftLeg",
            "part_state": row.left_leg_part_state,
            "max_armor": row.left_leg_max_armor,
            "max_health": row.left_leg_max_health,
            "current_armor": row.left_leg_current_armor,
            "current_health": row.left_leg_current_health,
        }),
        "right_leg": json!({
            "part_name": "RightLeg",
            "part_state": row.right_leg_part_state,
            "max_armor": row.right_leg_max_armor,
            "max_health": row.right_leg_max_health,
            "current_armor": row.right_leg_current_armor,
            "current_health": row.right_leg_current_health,
        }),
        "damage_results": row.damage_results,
    });
    Ok(serde_json::from_value(value)?)
}

fn finish(row: CurrentBossRow) -> Result<LoadedBoss, AppError> {
    let mut attackable_parts = Vec::with_capacity(8);
    if row.head_is_attackable {
        attackable_parts.push(BossPartName::Head);
    }
    if row.torso_is_attackable {
        attackable_parts.push(BossPartName::Torso);
    }
    if row.left_shoulder_is_attackable {
        attackable_parts.push(BossPartName::LeftShoulder);
    }
    if row.right_shoulder_is_attackable {
        attackable_parts.push(BossPartName::RightShoulder);
    }
    if row.left_hand_is_attackable {
        attackable_parts.push(BossPartName::LeftHand);
    }
    if row.right_hand_is_attackable {
        attackable_parts.push(BossPartName::RightHand);
    }
    if row.left_leg_is_attackable {
        attackable_parts.push(BossPartName::LeftLeg);
    }
    if row.right_leg_is_attackable {
        attackable_parts.push(BossPartName::RightLeg);
    }
    let boss = boss_from_row(&row)?;
    Ok(LoadedBoss {
        version: row.version,
        boss,
        attackable_parts,
        source_raid_id: row.source_raid_id,
        source_titan_index: row.source_titan_index,
        source_enemy_id: row.source_enemy_id,
        created_at: row.created_at,
        updated_at: row.updated_at,
    })
}

pub async fn load<'e, E>(executor: E) -> Result<Option<LoadedBoss>, AppError>
where
    E: PgExecutor<'e> + Copy,
{
    let row: Option<CurrentBossRow> = sqlx::query_as(&format!(
        "SELECT {CURRENT_BOSS_COLUMNS} FROM current_boss WHERE singleton = TRUE"
    ))
    .fetch_optional(executor)
    .await?;
    let Some(row) = row else {
        return Ok(None);
    };
    Ok(Some(finish(row)?))
}

/// Same as `load`, but locks the singleton row for the duration of `tx` --
/// used by the raid-ingestion pipeline, which already holds an advisory
/// lock (`RAID_STATE_LOCK`) serializing all boss-state writes anyway; this
/// row lock is defense-in-depth on top of that.
pub async fn load_for_update(
    tx: &mut Transaction<'_, Postgres>,
) -> Result<Option<LoadedBoss>, AppError> {
    let row: Option<CurrentBossRow> = sqlx::query_as(&format!(
        "SELECT {CURRENT_BOSS_COLUMNS} FROM current_boss WHERE singleton = TRUE FOR UPDATE"
    ))
    .fetch_optional(&mut **tx)
    .await?;
    let Some(row) = row else {
        return Ok(None);
    };
    Ok(Some(finish(row)?))
}

pub struct BossWrite<'a> {
    pub boss: &'a Boss,
    /// `None` leaves each part's `is_attackable` flag untouched (matches the
    /// call sites that only sync HP/curse state and never change targets).
    pub attackable_parts: Option<&'a [BossPartName]>,
    pub source_raid_id: Option<i64>,
    pub source_titan_index: Option<i32>,
    pub source_enemy_id: Option<&'a str>,
    pub bump_version: bool,
}

/// Upsert the singleton `current_boss` row, including its 8 body parts
/// (flattened into per-part columns -- see migration 0033). Each part's
/// `is_attackable` flag only changes when `write.attackable_parts` is
/// `Some`; when `None`, the `CASE` in the `ON CONFLICT` clause falls back to
/// the row's existing flag instead of the (unused) freshly-bound value.
/// Returns the resulting version.
pub async fn store(tx: &mut Transaction<'_, Postgres>, write: BossWrite<'_>) -> Result<i64, AppError> {
    let boss = write.boss;
    let attackable_parts_provided = write.attackable_parts.is_some();
    let is_attackable = |part_name: BossPartName| {
        write
            .attackable_parts
            .is_some_and(|parts| parts.contains(&part_name))
    };

    let version: i64 = sqlx::query_scalar(
        "INSERT INTO current_boss (
            singleton, version, boss_name, global_raid_modifier, global_raid_modifier_amount,
            curse_type, curse_damage_per_curse, recommend_1_to_2_part_patterns_only,
            damage_results, source_raid_id, source_titan_index, source_enemy_id,
            head_part_state, head_max_armor, head_max_health, head_current_armor, head_current_health, head_is_attackable,
            torso_part_state, torso_max_armor, torso_max_health, torso_current_armor, torso_current_health, torso_is_attackable,
            left_shoulder_part_state, left_shoulder_max_armor, left_shoulder_max_health, left_shoulder_current_armor, left_shoulder_current_health, left_shoulder_is_attackable,
            right_shoulder_part_state, right_shoulder_max_armor, right_shoulder_max_health, right_shoulder_current_armor, right_shoulder_current_health, right_shoulder_is_attackable,
            left_hand_part_state, left_hand_max_armor, left_hand_max_health, left_hand_current_armor, left_hand_current_health, left_hand_is_attackable,
            right_hand_part_state, right_hand_max_armor, right_hand_max_health, right_hand_current_armor, right_hand_current_health, right_hand_is_attackable,
            left_leg_part_state, left_leg_max_armor, left_leg_max_health, left_leg_current_armor, left_leg_current_health, left_leg_is_attackable,
            right_leg_part_state, right_leg_max_armor, right_leg_max_health, right_leg_current_armor, right_leg_current_health, right_leg_is_attackable
        ) VALUES (
            TRUE, 1, $1, $2, $3, $4, $5, $6, $7, $8, $9, $10,
            $12, $13, $14, $15, $16, $17,
            $18, $19, $20, $21, $22, $23,
            $24, $25, $26, $27, $28, $29,
            $30, $31, $32, $33, $34, $35,
            $36, $37, $38, $39, $40, $41,
            $42, $43, $44, $45, $46, $47,
            $48, $49, $50, $51, $52, $53,
            $54, $55, $56, $57, $58, $59
        )
        ON CONFLICT (singleton) DO UPDATE SET
            version = CASE WHEN $11 THEN current_boss.version + 1 ELSE current_boss.version END,
            boss_name = EXCLUDED.boss_name,
            global_raid_modifier = EXCLUDED.global_raid_modifier,
            global_raid_modifier_amount = EXCLUDED.global_raid_modifier_amount,
            curse_type = EXCLUDED.curse_type,
            curse_damage_per_curse = EXCLUDED.curse_damage_per_curse,
            recommend_1_to_2_part_patterns_only = EXCLUDED.recommend_1_to_2_part_patterns_only,
            damage_results = EXCLUDED.damage_results,
            source_raid_id = EXCLUDED.source_raid_id,
            source_titan_index = EXCLUDED.source_titan_index,
            source_enemy_id = EXCLUDED.source_enemy_id,
            updated_at = NOW(),
            head_part_state = EXCLUDED.head_part_state,
            head_max_armor = EXCLUDED.head_max_armor,
            head_max_health = EXCLUDED.head_max_health,
            head_current_armor = EXCLUDED.head_current_armor,
            head_current_health = EXCLUDED.head_current_health,
            head_is_attackable = CASE WHEN $60 THEN EXCLUDED.head_is_attackable ELSE current_boss.head_is_attackable END,
            torso_part_state = EXCLUDED.torso_part_state,
            torso_max_armor = EXCLUDED.torso_max_armor,
            torso_max_health = EXCLUDED.torso_max_health,
            torso_current_armor = EXCLUDED.torso_current_armor,
            torso_current_health = EXCLUDED.torso_current_health,
            torso_is_attackable = CASE WHEN $60 THEN EXCLUDED.torso_is_attackable ELSE current_boss.torso_is_attackable END,
            left_shoulder_part_state = EXCLUDED.left_shoulder_part_state,
            left_shoulder_max_armor = EXCLUDED.left_shoulder_max_armor,
            left_shoulder_max_health = EXCLUDED.left_shoulder_max_health,
            left_shoulder_current_armor = EXCLUDED.left_shoulder_current_armor,
            left_shoulder_current_health = EXCLUDED.left_shoulder_current_health,
            left_shoulder_is_attackable = CASE WHEN $60 THEN EXCLUDED.left_shoulder_is_attackable ELSE current_boss.left_shoulder_is_attackable END,
            right_shoulder_part_state = EXCLUDED.right_shoulder_part_state,
            right_shoulder_max_armor = EXCLUDED.right_shoulder_max_armor,
            right_shoulder_max_health = EXCLUDED.right_shoulder_max_health,
            right_shoulder_current_armor = EXCLUDED.right_shoulder_current_armor,
            right_shoulder_current_health = EXCLUDED.right_shoulder_current_health,
            right_shoulder_is_attackable = CASE WHEN $60 THEN EXCLUDED.right_shoulder_is_attackable ELSE current_boss.right_shoulder_is_attackable END,
            left_hand_part_state = EXCLUDED.left_hand_part_state,
            left_hand_max_armor = EXCLUDED.left_hand_max_armor,
            left_hand_max_health = EXCLUDED.left_hand_max_health,
            left_hand_current_armor = EXCLUDED.left_hand_current_armor,
            left_hand_current_health = EXCLUDED.left_hand_current_health,
            left_hand_is_attackable = CASE WHEN $60 THEN EXCLUDED.left_hand_is_attackable ELSE current_boss.left_hand_is_attackable END,
            right_hand_part_state = EXCLUDED.right_hand_part_state,
            right_hand_max_armor = EXCLUDED.right_hand_max_armor,
            right_hand_max_health = EXCLUDED.right_hand_max_health,
            right_hand_current_armor = EXCLUDED.right_hand_current_armor,
            right_hand_current_health = EXCLUDED.right_hand_current_health,
            right_hand_is_attackable = CASE WHEN $60 THEN EXCLUDED.right_hand_is_attackable ELSE current_boss.right_hand_is_attackable END,
            left_leg_part_state = EXCLUDED.left_leg_part_state,
            left_leg_max_armor = EXCLUDED.left_leg_max_armor,
            left_leg_max_health = EXCLUDED.left_leg_max_health,
            left_leg_current_armor = EXCLUDED.left_leg_current_armor,
            left_leg_current_health = EXCLUDED.left_leg_current_health,
            left_leg_is_attackable = CASE WHEN $60 THEN EXCLUDED.left_leg_is_attackable ELSE current_boss.left_leg_is_attackable END,
            right_leg_part_state = EXCLUDED.right_leg_part_state,
            right_leg_max_armor = EXCLUDED.right_leg_max_armor,
            right_leg_max_health = EXCLUDED.right_leg_max_health,
            right_leg_current_armor = EXCLUDED.right_leg_current_armor,
            right_leg_current_health = EXCLUDED.right_leg_current_health,
            right_leg_is_attackable = CASE WHEN $60 THEN EXCLUDED.right_leg_is_attackable ELSE current_boss.right_leg_is_attackable END
        RETURNING version",
    )
    .bind(boss.boss_name)
    .bind(boss.global_raid_modifier)
    .bind(boss.global_raid_modifier_amount)
    .bind(boss.curse_type)
    .bind(boss.curse_damage_per_curse)
    .bind(boss.recommend_1_to_2_part_patterns_only)
    .bind(serde_json::to_value(&boss.damage_results)?)
    .bind(write.source_raid_id)
    .bind(write.source_titan_index)
    .bind(write.source_enemy_id)
    .bind(write.bump_version)
    .bind(boss.head.part_state)
    .bind(boss.head.max_armor as i64)
    .bind(boss.head.max_health as i64)
    .bind(boss.head.current_armor as i64)
    .bind(boss.head.current_health as i64)
    .bind(is_attackable(BossPartName::Head))
    .bind(boss.torso.part_state)
    .bind(boss.torso.max_armor as i64)
    .bind(boss.torso.max_health as i64)
    .bind(boss.torso.current_armor as i64)
    .bind(boss.torso.current_health as i64)
    .bind(is_attackable(BossPartName::Torso))
    .bind(boss.left_shoulder.part_state)
    .bind(boss.left_shoulder.max_armor as i64)
    .bind(boss.left_shoulder.max_health as i64)
    .bind(boss.left_shoulder.current_armor as i64)
    .bind(boss.left_shoulder.current_health as i64)
    .bind(is_attackable(BossPartName::LeftShoulder))
    .bind(boss.right_shoulder.part_state)
    .bind(boss.right_shoulder.max_armor as i64)
    .bind(boss.right_shoulder.max_health as i64)
    .bind(boss.right_shoulder.current_armor as i64)
    .bind(boss.right_shoulder.current_health as i64)
    .bind(is_attackable(BossPartName::RightShoulder))
    .bind(boss.left_hand.part_state)
    .bind(boss.left_hand.max_armor as i64)
    .bind(boss.left_hand.max_health as i64)
    .bind(boss.left_hand.current_armor as i64)
    .bind(boss.left_hand.current_health as i64)
    .bind(is_attackable(BossPartName::LeftHand))
    .bind(boss.right_hand.part_state)
    .bind(boss.right_hand.max_armor as i64)
    .bind(boss.right_hand.max_health as i64)
    .bind(boss.right_hand.current_armor as i64)
    .bind(boss.right_hand.current_health as i64)
    .bind(is_attackable(BossPartName::RightHand))
    .bind(boss.left_leg.part_state)
    .bind(boss.left_leg.max_armor as i64)
    .bind(boss.left_leg.max_health as i64)
    .bind(boss.left_leg.current_armor as i64)
    .bind(boss.left_leg.current_health as i64)
    .bind(is_attackable(BossPartName::LeftLeg))
    .bind(boss.right_leg.part_state)
    .bind(boss.right_leg.max_armor as i64)
    .bind(boss.right_leg.max_health as i64)
    .bind(boss.right_leg.current_armor as i64)
    .bind(boss.right_leg.current_health as i64)
    .bind(is_attackable(BossPartName::RightLeg))
    .bind(attackable_parts_provided)
    .fetch_one(&mut **tx)
    .await?;

    Ok(version)
}
