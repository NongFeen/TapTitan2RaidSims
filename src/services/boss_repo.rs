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
}

#[derive(sqlx::FromRow)]
struct CurrentBossPartRow {
    part_name: BossPartName,
    part_state: PartState,
    max_armor: i64,
    max_health: i64,
    current_armor: i64,
    current_health: i64,
    radioactivity_afflicted_seconds: f64,
}

const CURRENT_BOSS_COLUMNS: &str = "version, boss_name, global_raid_modifier, \
    global_raid_modifier_amount, curse_type, curse_damage_per_curse, \
    recommend_1_to_2_part_patterns_only, damage_results, \
    source_raid_id, source_titan_index, source_enemy_id, created_at, updated_at";

/// Rebuild the exact `boss_data` JSON shape the old JSONB column held, then
/// reuse `Boss`'s existing (already-correct) `Deserialize` impl to construct
/// it -- `Boss` has a private field (`initial_cursed_part_count`) that a
/// struct literal from outside its module can't set directly.
fn boss_from_row(row: &CurrentBossRow, parts: &[CurrentBossPartRow]) -> Result<Boss, AppError> {
    let part_value = |name: BossPartName| -> Value {
        let part = parts
            .iter()
            .find(|p| p.part_name == name)
            .expect("current_boss_parts always has all 8 parts");
        json!({
            "part_name": part.part_name,
            "part_state": part.part_state,
            "max_armor": part.max_armor,
            "max_health": part.max_health,
            "current_armor": part.current_armor,
            "current_health": part.current_health,
            "radioactivity_afflicted_seconds": part.radioactivity_afflicted_seconds,
        })
    };
    let value = json!({
        "boss_name": row.boss_name,
        "global_raid_modifier": row.global_raid_modifier,
        "global_raid_modifier_amount": row.global_raid_modifier_amount,
        "curse_type": row.curse_type,
        "curse_damage_per_curse": row.curse_damage_per_curse,
        "recommend_1_to_2_part_patterns_only": row.recommend_1_to_2_part_patterns_only,
        "head": part_value(BossPartName::Head),
        "torso": part_value(BossPartName::Torso),
        "left_shoulder": part_value(BossPartName::LeftShoulder),
        "right_shoulder": part_value(BossPartName::RightShoulder),
        "left_hand": part_value(BossPartName::LeftHand),
        "right_hand": part_value(BossPartName::RightHand),
        "left_leg": part_value(BossPartName::LeftLeg),
        "right_leg": part_value(BossPartName::RightLeg),
        "damage_results": row.damage_results,
    });
    Ok(serde_json::from_value(value)?)
}

fn finish(
    row: CurrentBossRow,
    parts: Vec<CurrentBossPartRow>,
    attackable_parts: Vec<BossPartName>,
) -> Result<LoadedBoss, AppError> {
    let boss = boss_from_row(&row, &parts)?;
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

const CURRENT_BOSS_PARTS_QUERY: &str = "SELECT part_name, part_state, max_armor, max_health, current_armor, current_health, radioactivity_afflicted_seconds FROM current_boss_parts";
const CURRENT_BOSS_ATTACKABLE_PARTS_QUERY: &str =
    "SELECT part_name FROM current_boss_attackable_parts";

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
    let parts: Vec<CurrentBossPartRow> = sqlx::query_as(CURRENT_BOSS_PARTS_QUERY)
        .fetch_all(executor)
        .await?;
    let attackable_parts: Vec<BossPartName> = sqlx::query_scalar(CURRENT_BOSS_ATTACKABLE_PARTS_QUERY)
        .fetch_all(executor)
        .await?;
    Ok(Some(finish(row, parts, attackable_parts)?))
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
    let parts: Vec<CurrentBossPartRow> = sqlx::query_as(CURRENT_BOSS_PARTS_QUERY)
        .fetch_all(&mut **tx)
        .await?;
    let attackable_parts: Vec<BossPartName> = sqlx::query_scalar(CURRENT_BOSS_ATTACKABLE_PARTS_QUERY)
        .fetch_all(&mut **tx)
        .await?;
    Ok(Some(finish(row, parts, attackable_parts)?))
}

pub struct BossWrite<'a> {
    pub boss: &'a Boss,
    /// `None` leaves `current_boss_attackable_parts` untouched (matches the
    /// call sites that only sync HP/curse state and never change targets).
    pub attackable_parts: Option<&'a [BossPartName]>,
    pub source_raid_id: Option<i64>,
    pub source_titan_index: Option<i32>,
    pub source_enemy_id: Option<&'a str>,
    pub bump_version: bool,
}

/// Upsert the singleton `current_boss` row + its `current_boss_parts` child
/// rows (always replaced) + `current_boss_attackable_parts` (replaced only
/// if `write.attackable_parts` is `Some`). Returns the resulting version.
pub async fn store(tx: &mut Transaction<'_, Postgres>, write: BossWrite<'_>) -> Result<i64, AppError> {
    let boss = write.boss;
    let version: i64 = sqlx::query_scalar(
        "INSERT INTO current_boss (
            singleton, version, boss_name, global_raid_modifier, global_raid_modifier_amount,
            curse_type, curse_damage_per_curse, recommend_1_to_2_part_patterns_only,
            damage_results, source_raid_id, source_titan_index, source_enemy_id
        ) VALUES (TRUE, 1, $1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
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
            updated_at = NOW()
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
    .fetch_one(&mut **tx)
    .await?;

    sqlx::query("DELETE FROM current_boss_parts")
        .execute(&mut **tx)
        .await?;
    for part in boss.parts() {
        sqlx::query(
            "INSERT INTO current_boss_parts (part_name, part_state, max_armor, max_health, current_armor, current_health, radioactivity_afflicted_seconds) VALUES ($1,$2,$3,$4,$5,$6,$7)",
        )
        .bind(part.part_name)
        .bind(part.part_state)
        .bind(part.max_armor as i64)
        .bind(part.max_health as i64)
        .bind(part.current_armor as i64)
        .bind(part.current_health as i64)
        .bind(part.radioactivity_afflicted_seconds)
        .execute(&mut **tx)
        .await?;
    }

    if let Some(attackable_parts) = write.attackable_parts {
        sqlx::query("DELETE FROM current_boss_attackable_parts")
            .execute(&mut **tx)
            .await?;
        for part_name in attackable_parts {
            sqlx::query("INSERT INTO current_boss_attackable_parts (part_name) VALUES ($1)")
                .bind(part_name)
                .execute(&mut **tx)
                .await?;
        }
    }

    Ok(version)
}
