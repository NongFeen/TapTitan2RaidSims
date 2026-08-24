use chrono::{DateTime, Utc};
use sqlx::{PgExecutor, Postgres, Transaction};
use uuid::Uuid;

use crate::models::{
    cards::{Card, CardName},
    player_raid_data::{GemstoneResearch, PlayerRaidData, RaidCardResearch, RaidSet, TitanSoulResearch},
};

pub struct LoadedPlayerStats {
    pub revision: i64,
    pub data: PlayerRaidData,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(sqlx::FromRow)]
struct PlayerStatsRow {
    revision: i64,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,

    player_raid_level: i32,
    player_raid_base_damage: i32,
    title: f32,

    raid_set_jade_anniversary: bool,
    raid_set_jukk_juggernaut: bool,
    raid_set_airforce_ace: bool,
    raid_set_dancer_venom: bool,
    raid_set_rose_anniversary: bool,

    titan_soul_head_mult: f32,
    titan_soul_torso_mult: f32,
    titan_soul_limbs_mult: f32,
    titan_soul_armor_mult: f32,
    titan_soul_body_mult: f32,
    titan_soul_lojak_mult: f32,
    titan_soul_takedar_mult: f32,
    titan_soul_jukk_mult: f32,
    titan_soul_sterl_mult: f32,
    titan_soul_mohaca_mult: f32,
    titan_soul_terro_mult: f32,
    titan_soul_klonk_mult: f32,
    titan_soul_priker_mult: f32,

    card_base_damage: i32,
    card_head_damage: i32,
    card_torso_damage: i32,
    card_limbs_damage: i32,
    card_armor_damage: i32,
    card_head_armor_damage: i32,
    card_torso_armor_damage: i32,
    card_limbs_armor_damage: i32,
    card_body_damage: i32,
    card_head_body_damage: i32,
    card_torso_body_damage: i32,
    card_limbs_body_damage: i32,
    card_lojak_damage: i32,
    card_takedar_damage: i32,
    card_jukk_damage: i32,
    card_sterl_damage: i32,
    card_mohaca_damage: i32,
    card_terro_damage: i32,
    card_klonk_damage: i32,
    card_priker_damage: i32,
    card_base_burst_damage: i32,
    card_burst_lojak_damage: i32,
    card_burst_takedar_damage: i32,
    card_burst_jukk_damage: i32,
    card_burst_sterl_damage: i32,
    card_burst_mohaca_damage: i32,
    card_burst_terro_damage: i32,
    card_burst_klonk_damage: i32,
    card_burst_priker_damage: i32,
    card_base_affliction_damage: i32,
    card_affliction_lojak_damage: i32,
    card_affliction_takedar_damage: i32,
    card_affliction_jukk_damage: i32,
    card_affliction_sterl_damage: i32,
    card_affliction_mohaca_damage: i32,
    card_affliction_terro_damage: i32,
    card_affliction_klonk_damage: i32,
    card_affliction_priker_damage: i32,

    gem_base_damage: i32,
    gem_head_damage: i32,
    gem_torso_damage: i32,
    gem_limbs_damage: i32,
    gem_armor_damage: i32,
    gem_head_armor_damage: i32,
    gem_torso_armor_damage: i32,
    gem_limbs_armor_damage: i32,
    gem_body_damage: i32,
    gem_head_body_damage: i32,
    gem_torso_body_damage: i32,
    gem_limbs_body_damage: i32,
    gem_lojak_damage: i32,
    gem_takedar_damage: i32,
    gem_jukk_damage: i32,
    gem_sterl_damage: i32,
    gem_mohaca_damage: i32,
    gem_terro_damage: i32,
    gem_klonk_damage: i32,
    gem_priker_damage: i32,
    gem_base_burst_damage: i32,
    gem_burst_lojak_damage: i32,
    gem_burst_takedar_damage: i32,
    gem_burst_jukk_damage: i32,
    gem_burst_sterl_damage: i32,
    gem_burst_mohaca_damage: i32,
    gem_burst_terro_damage: i32,
    gem_burst_klonk_damage: i32,
    gem_burst_priker_damage: i32,
    gem_base_affliction_damage: i32,
    gem_affliction_lojak_damage: i32,
    gem_affliction_takedar_damage: i32,
    gem_affliction_jukk_damage: i32,
    gem_affliction_sterl_damage: i32,
    gem_affliction_mohaca_damage: i32,
    gem_affliction_terro_damage: i32,
    gem_affliction_klonk_damage: i32,
    gem_affliction_priker_damage: i32,
}

const PLAYER_STATS_COLUMNS: &str = "revision, created_at, updated_at, \
    player_raid_level, player_raid_base_damage, title, \
    raid_set_jade_anniversary, raid_set_jukk_juggernaut, raid_set_airforce_ace, \
    raid_set_dancer_venom, raid_set_rose_anniversary, \
    titan_soul_head_mult, titan_soul_torso_mult, titan_soul_limbs_mult, titan_soul_armor_mult, \
    titan_soul_body_mult, titan_soul_lojak_mult, titan_soul_takedar_mult, titan_soul_jukk_mult, \
    titan_soul_sterl_mult, titan_soul_mohaca_mult, titan_soul_terro_mult, titan_soul_klonk_mult, \
    titan_soul_priker_mult, \
    card_base_damage, card_head_damage, card_torso_damage, card_limbs_damage, card_armor_damage, \
    card_head_armor_damage, card_torso_armor_damage, card_limbs_armor_damage, card_body_damage, \
    card_head_body_damage, card_torso_body_damage, card_limbs_body_damage, card_lojak_damage, \
    card_takedar_damage, card_jukk_damage, card_sterl_damage, card_mohaca_damage, card_terro_damage, \
    card_klonk_damage, card_priker_damage, card_base_burst_damage, card_burst_lojak_damage, \
    card_burst_takedar_damage, card_burst_jukk_damage, card_burst_sterl_damage, card_burst_mohaca_damage, \
    card_burst_terro_damage, card_burst_klonk_damage, card_burst_priker_damage, \
    card_base_affliction_damage, card_affliction_lojak_damage, card_affliction_takedar_damage, \
    card_affliction_jukk_damage, card_affliction_sterl_damage, card_affliction_mohaca_damage, \
    card_affliction_terro_damage, card_affliction_klonk_damage, card_affliction_priker_damage, \
    gem_base_damage, gem_head_damage, gem_torso_damage, gem_limbs_damage, gem_armor_damage, \
    gem_head_armor_damage, gem_torso_armor_damage, gem_limbs_armor_damage, gem_body_damage, \
    gem_head_body_damage, gem_torso_body_damage, gem_limbs_body_damage, gem_lojak_damage, \
    gem_takedar_damage, gem_jukk_damage, gem_sterl_damage, gem_mohaca_damage, gem_terro_damage, \
    gem_klonk_damage, gem_priker_damage, gem_base_burst_damage, gem_burst_lojak_damage, \
    gem_burst_takedar_damage, gem_burst_jukk_damage, gem_burst_sterl_damage, gem_burst_mohaca_damage, \
    gem_burst_terro_damage, gem_burst_klonk_damage, gem_burst_priker_damage, \
    gem_base_affliction_damage, gem_affliction_lojak_damage, gem_affliction_takedar_damage, \
    gem_affliction_jukk_damage, gem_affliction_sterl_damage, gem_affliction_mohaca_damage, \
    gem_affliction_terro_damage, gem_affliction_klonk_damage, gem_affliction_priker_damage";

#[derive(sqlx::FromRow)]
struct PlayerCardRow {
    card_id: CardName,
    level: i32,
    enabled: bool,
}

fn row_to_raid_card_research(row: &PlayerStatsRow) -> RaidCardResearch {
    RaidCardResearch {
        base_damage: row.card_base_damage as u16,
        head_damage: row.card_head_damage as u16,
        torso_damage: row.card_torso_damage as u16,
        limbs_damage: row.card_limbs_damage as u16,
        armor_damage: row.card_armor_damage as u16,
        head_armor_damage: row.card_head_armor_damage as u16,
        torso_armor_damage: row.card_torso_armor_damage as u16,
        limbs_armor_damage: row.card_limbs_armor_damage as u16,
        body_damage: row.card_body_damage as u16,
        head_body_damage: row.card_head_body_damage as u16,
        torso_body_damage: row.card_torso_body_damage as u16,
        limbs_body_damage: row.card_limbs_body_damage as u16,
        lojak_damage: row.card_lojak_damage as u16,
        takedar_damage: row.card_takedar_damage as u16,
        jukk_damage: row.card_jukk_damage as u16,
        sterl_damage: row.card_sterl_damage as u16,
        mohaca_damage: row.card_mohaca_damage as u16,
        terro_damage: row.card_terro_damage as u16,
        klonk_damage: row.card_klonk_damage as u16,
        priker_damage: row.card_priker_damage as u16,
        base_burst_damage: row.card_base_burst_damage as u16,
        burst_lojak_damage: row.card_burst_lojak_damage as u16,
        burst_takedar_damage: row.card_burst_takedar_damage as u16,
        burst_jukk_damage: row.card_burst_jukk_damage as u16,
        burst_sterl_damage: row.card_burst_sterl_damage as u16,
        burst_mohaca_damage: row.card_burst_mohaca_damage as u16,
        burst_terro_damage: row.card_burst_terro_damage as u16,
        burst_klonk_damage: row.card_burst_klonk_damage as u16,
        burst_priker_damage: row.card_burst_priker_damage as u16,
        base_affliction_damage: row.card_base_affliction_damage as u16,
        affliction_lojak_damage: row.card_affliction_lojak_damage as u16,
        affliction_takedar_damage: row.card_affliction_takedar_damage as u16,
        affliction_jukk_damage: row.card_affliction_jukk_damage as u16,
        affliction_sterl_damage: row.card_affliction_sterl_damage as u16,
        affliction_mohaca_damage: row.card_affliction_mohaca_damage as u16,
        affliction_terro_damage: row.card_affliction_terro_damage as u16,
        affliction_klonk_damage: row.card_affliction_klonk_damage as u16,
        affliction_priker_damage: row.card_affliction_priker_damage as u16,
    }
}

fn row_to_gemstone_research(row: &PlayerStatsRow) -> GemstoneResearch {
    GemstoneResearch {
        base_damage: row.gem_base_damage as u16,
        head_damage: row.gem_head_damage as u16,
        torso_damage: row.gem_torso_damage as u16,
        limbs_damage: row.gem_limbs_damage as u16,
        armor_damage: row.gem_armor_damage as u16,
        head_armor_damage: row.gem_head_armor_damage as u16,
        torso_armor_damage: row.gem_torso_armor_damage as u16,
        limbs_armor_damage: row.gem_limbs_armor_damage as u16,
        body_damage: row.gem_body_damage as u16,
        head_body_damage: row.gem_head_body_damage as u16,
        torso_body_damage: row.gem_torso_body_damage as u16,
        limbs_body_damage: row.gem_limbs_body_damage as u16,
        lojak_damage: row.gem_lojak_damage as u16,
        takedar_damage: row.gem_takedar_damage as u16,
        jukk_damage: row.gem_jukk_damage as u16,
        sterl_damage: row.gem_sterl_damage as u16,
        mohaca_damage: row.gem_mohaca_damage as u16,
        terro_damage: row.gem_terro_damage as u16,
        klonk_damage: row.gem_klonk_damage as u16,
        priker_damage: row.gem_priker_damage as u16,
        base_burst_damage: row.gem_base_burst_damage as u16,
        burst_lojak_damage: row.gem_burst_lojak_damage as u16,
        burst_takedar_damage: row.gem_burst_takedar_damage as u16,
        burst_jukk_damage: row.gem_burst_jukk_damage as u16,
        burst_sterl_damage: row.gem_burst_sterl_damage as u16,
        burst_mohaca_damage: row.gem_burst_mohaca_damage as u16,
        burst_terro_damage: row.gem_burst_terro_damage as u16,
        burst_klonk_damage: row.gem_burst_klonk_damage as u16,
        burst_priker_damage: row.gem_burst_priker_damage as u16,
        base_affliction_damage: row.gem_base_affliction_damage as u16,
        affliction_lojak_damage: row.gem_affliction_lojak_damage as u16,
        affliction_takedar_damage: row.gem_affliction_takedar_damage as u16,
        affliction_jukk_damage: row.gem_affliction_jukk_damage as u16,
        affliction_sterl_damage: row.gem_affliction_sterl_damage as u16,
        affliction_mohaca_damage: row.gem_affliction_mohaca_damage as u16,
        affliction_terro_damage: row.gem_affliction_terro_damage as u16,
        affliction_klonk_damage: row.gem_affliction_klonk_damage as u16,
        affliction_priker_damage: row.gem_affliction_priker_damage as u16,
    }
}

fn row_to_titan_soul_research(row: &PlayerStatsRow) -> TitanSoulResearch {
    TitanSoulResearch {
        head_mult: row.titan_soul_head_mult,
        torso_mult: row.titan_soul_torso_mult,
        limbs_mult: row.titan_soul_limbs_mult,
        armor_mult: row.titan_soul_armor_mult,
        body_mult: row.titan_soul_body_mult,
        lojak_mult: row.titan_soul_lojak_mult,
        takedar_mult: row.titan_soul_takedar_mult,
        jukk_mult: row.titan_soul_jukk_mult,
        sterl_mult: row.titan_soul_sterl_mult,
        mohaca_mult: row.titan_soul_mohaca_mult,
        terro_mult: row.titan_soul_terro_mult,
        klonk_mult: row.titan_soul_klonk_mult,
        priker_mult: row.titan_soul_priker_mult,
    }
}

/// Load a player's normalized stats + card list and reassemble the exact
/// `PlayerRaidData` shape the rest of the app (and the frontend, via
/// `serde_json::to_value`) expects.
pub async fn load<'e, E>(executor: E, player_id: Uuid) -> Result<Option<LoadedPlayerStats>, sqlx::Error>
where
    E: PgExecutor<'e> + Copy,
{
    let row: Option<PlayerStatsRow> = sqlx::query_as(&format!(
        "SELECT {PLAYER_STATS_COLUMNS} FROM player_stats WHERE player_id = $1"
    ))
    .bind(player_id)
    .fetch_optional(executor)
    .await?;
    let Some(row) = row else {
        return Ok(None);
    };

    let card_rows: Vec<PlayerCardRow> =
        sqlx::query_as("SELECT card_id, level, enabled FROM player_cards WHERE player_id = $1")
            .bind(player_id)
            .fetch_all(executor)
            .await?;
    let card_list = card_rows
        .into_iter()
        .map(|card_row| Card {
            card_id: card_row.card_id,
            cardtype: card_row.card_id.card_type(),
            level: card_row.level as u16,
            enabled: card_row.enabled,
            tap_count: 0,
            chained_parts: Vec::new(),
            celestial_stacks: 0,
            skill: Default::default(),
            proc_chance_cache: 0.0,
        })
        .collect();

    let data = PlayerRaidData {
        player_raid_level: row.player_raid_level as u16,
        player_raid_base_damage: row.player_raid_base_damage as u16,
        raid_set: RaidSet {
            jade_anniversary: row.raid_set_jade_anniversary,
            jukk_juggernaut: row.raid_set_jukk_juggernaut,
            airforce_ace: row.raid_set_airforce_ace,
            dancer_venom: row.raid_set_dancer_venom,
            rose_anniversary: row.raid_set_rose_anniversary,
        },
        titan_soul_research: row_to_titan_soul_research(&row),
        raid_card_research: row_to_raid_card_research(&row),
        gem_stone_research: row_to_gemstone_research(&row),
        card_list,
        title: row.title,
    };

    Ok(Some(LoadedPlayerStats {
        revision: row.revision,
        data,
        created_at: row.created_at,
        updated_at: row.updated_at,
    }))
}

/// Upsert a player's normalized stats + card list inside the caller's
/// transaction, bumping `revision` the same way the old JSONB upsert did.
pub async fn store(
    tx: &mut Transaction<'_, Postgres>,
    player_id: Uuid,
    data: &PlayerRaidData,
) -> Result<LoadedPlayerStats, sqlx::Error> {
    let research = &data.raid_card_research;
    let gem = &data.gem_stone_research;
    let titan = &data.titan_soul_research;
    let raid_set = &data.raid_set;

    let row: PlayerStatsRow = sqlx::query_as(&format!(
        "INSERT INTO player_stats (
            player_id, revision,
            player_raid_level, player_raid_base_damage, title,
            raid_set_jade_anniversary, raid_set_jukk_juggernaut, raid_set_airforce_ace,
            raid_set_dancer_venom, raid_set_rose_anniversary,
            titan_soul_head_mult, titan_soul_torso_mult, titan_soul_limbs_mult, titan_soul_armor_mult,
            titan_soul_body_mult, titan_soul_lojak_mult, titan_soul_takedar_mult, titan_soul_jukk_mult,
            titan_soul_sterl_mult, titan_soul_mohaca_mult, titan_soul_terro_mult, titan_soul_klonk_mult,
            titan_soul_priker_mult,
            card_base_damage, card_head_damage, card_torso_damage, card_limbs_damage, card_armor_damage,
            card_head_armor_damage, card_torso_armor_damage, card_limbs_armor_damage, card_body_damage,
            card_head_body_damage, card_torso_body_damage, card_limbs_body_damage, card_lojak_damage,
            card_takedar_damage, card_jukk_damage, card_sterl_damage, card_mohaca_damage, card_terro_damage,
            card_klonk_damage, card_priker_damage, card_base_burst_damage, card_burst_lojak_damage,
            card_burst_takedar_damage, card_burst_jukk_damage, card_burst_sterl_damage, card_burst_mohaca_damage,
            card_burst_terro_damage, card_burst_klonk_damage, card_burst_priker_damage,
            card_base_affliction_damage, card_affliction_lojak_damage, card_affliction_takedar_damage,
            card_affliction_jukk_damage, card_affliction_sterl_damage, card_affliction_mohaca_damage,
            card_affliction_terro_damage, card_affliction_klonk_damage, card_affliction_priker_damage,
            gem_base_damage, gem_head_damage, gem_torso_damage, gem_limbs_damage, gem_armor_damage,
            gem_head_armor_damage, gem_torso_armor_damage, gem_limbs_armor_damage, gem_body_damage,
            gem_head_body_damage, gem_torso_body_damage, gem_limbs_body_damage, gem_lojak_damage,
            gem_takedar_damage, gem_jukk_damage, gem_sterl_damage, gem_mohaca_damage, gem_terro_damage,
            gem_klonk_damage, gem_priker_damage, gem_base_burst_damage, gem_burst_lojak_damage,
            gem_burst_takedar_damage, gem_burst_jukk_damage, gem_burst_sterl_damage, gem_burst_mohaca_damage,
            gem_burst_terro_damage, gem_burst_klonk_damage, gem_burst_priker_damage,
            gem_base_affliction_damage, gem_affliction_lojak_damage, gem_affliction_takedar_damage,
            gem_affliction_jukk_damage, gem_affliction_sterl_damage, gem_affliction_mohaca_damage,
            gem_affliction_terro_damage, gem_affliction_klonk_damage, gem_affliction_priker_damage
        ) VALUES (
            $1, 1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18, $19, $20,
            $21, $22, $23, $24, $25, $26, $27, $28, $29, $30, $31, $32, $33, $34, $35, $36, $37, $38, $39,
            $40, $41, $42, $43, $44, $45, $46, $47, $48, $49, $50, $51, $52, $53, $54, $55, $56, $57, $58,
            $59, $60, $61, $62, $63, $64, $65, $66, $67, $68, $69, $70, $71, $72, $73, $74, $75, $76, $77,
            $78, $79, $80, $81, $82, $83, $84, $85, $86, $87, $88, $89, $90, $91, $92, $93, $94, $95
        )
        ON CONFLICT (player_id) DO UPDATE SET
            revision = player_stats.revision + 1,
            player_raid_level = EXCLUDED.player_raid_level,
            player_raid_base_damage = EXCLUDED.player_raid_base_damage,
            title = EXCLUDED.title,
            raid_set_jade_anniversary = EXCLUDED.raid_set_jade_anniversary,
            raid_set_jukk_juggernaut = EXCLUDED.raid_set_jukk_juggernaut,
            raid_set_airforce_ace = EXCLUDED.raid_set_airforce_ace,
            raid_set_dancer_venom = EXCLUDED.raid_set_dancer_venom,
            raid_set_rose_anniversary = EXCLUDED.raid_set_rose_anniversary,
            titan_soul_head_mult = EXCLUDED.titan_soul_head_mult,
            titan_soul_torso_mult = EXCLUDED.titan_soul_torso_mult,
            titan_soul_limbs_mult = EXCLUDED.titan_soul_limbs_mult,
            titan_soul_armor_mult = EXCLUDED.titan_soul_armor_mult,
            titan_soul_body_mult = EXCLUDED.titan_soul_body_mult,
            titan_soul_lojak_mult = EXCLUDED.titan_soul_lojak_mult,
            titan_soul_takedar_mult = EXCLUDED.titan_soul_takedar_mult,
            titan_soul_jukk_mult = EXCLUDED.titan_soul_jukk_mult,
            titan_soul_sterl_mult = EXCLUDED.titan_soul_sterl_mult,
            titan_soul_mohaca_mult = EXCLUDED.titan_soul_mohaca_mult,
            titan_soul_terro_mult = EXCLUDED.titan_soul_terro_mult,
            titan_soul_klonk_mult = EXCLUDED.titan_soul_klonk_mult,
            titan_soul_priker_mult = EXCLUDED.titan_soul_priker_mult,
            card_base_damage = EXCLUDED.card_base_damage,
            card_head_damage = EXCLUDED.card_head_damage,
            card_torso_damage = EXCLUDED.card_torso_damage,
            card_limbs_damage = EXCLUDED.card_limbs_damage,
            card_armor_damage = EXCLUDED.card_armor_damage,
            card_head_armor_damage = EXCLUDED.card_head_armor_damage,
            card_torso_armor_damage = EXCLUDED.card_torso_armor_damage,
            card_limbs_armor_damage = EXCLUDED.card_limbs_armor_damage,
            card_body_damage = EXCLUDED.card_body_damage,
            card_head_body_damage = EXCLUDED.card_head_body_damage,
            card_torso_body_damage = EXCLUDED.card_torso_body_damage,
            card_limbs_body_damage = EXCLUDED.card_limbs_body_damage,
            card_lojak_damage = EXCLUDED.card_lojak_damage,
            card_takedar_damage = EXCLUDED.card_takedar_damage,
            card_jukk_damage = EXCLUDED.card_jukk_damage,
            card_sterl_damage = EXCLUDED.card_sterl_damage,
            card_mohaca_damage = EXCLUDED.card_mohaca_damage,
            card_terro_damage = EXCLUDED.card_terro_damage,
            card_klonk_damage = EXCLUDED.card_klonk_damage,
            card_priker_damage = EXCLUDED.card_priker_damage,
            card_base_burst_damage = EXCLUDED.card_base_burst_damage,
            card_burst_lojak_damage = EXCLUDED.card_burst_lojak_damage,
            card_burst_takedar_damage = EXCLUDED.card_burst_takedar_damage,
            card_burst_jukk_damage = EXCLUDED.card_burst_jukk_damage,
            card_burst_sterl_damage = EXCLUDED.card_burst_sterl_damage,
            card_burst_mohaca_damage = EXCLUDED.card_burst_mohaca_damage,
            card_burst_terro_damage = EXCLUDED.card_burst_terro_damage,
            card_burst_klonk_damage = EXCLUDED.card_burst_klonk_damage,
            card_burst_priker_damage = EXCLUDED.card_burst_priker_damage,
            card_base_affliction_damage = EXCLUDED.card_base_affliction_damage,
            card_affliction_lojak_damage = EXCLUDED.card_affliction_lojak_damage,
            card_affliction_takedar_damage = EXCLUDED.card_affliction_takedar_damage,
            card_affliction_jukk_damage = EXCLUDED.card_affliction_jukk_damage,
            card_affliction_sterl_damage = EXCLUDED.card_affliction_sterl_damage,
            card_affliction_mohaca_damage = EXCLUDED.card_affliction_mohaca_damage,
            card_affliction_terro_damage = EXCLUDED.card_affliction_terro_damage,
            card_affliction_klonk_damage = EXCLUDED.card_affliction_klonk_damage,
            card_affliction_priker_damage = EXCLUDED.card_affliction_priker_damage,
            gem_base_damage = EXCLUDED.gem_base_damage,
            gem_head_damage = EXCLUDED.gem_head_damage,
            gem_torso_damage = EXCLUDED.gem_torso_damage,
            gem_limbs_damage = EXCLUDED.gem_limbs_damage,
            gem_armor_damage = EXCLUDED.gem_armor_damage,
            gem_head_armor_damage = EXCLUDED.gem_head_armor_damage,
            gem_torso_armor_damage = EXCLUDED.gem_torso_armor_damage,
            gem_limbs_armor_damage = EXCLUDED.gem_limbs_armor_damage,
            gem_body_damage = EXCLUDED.gem_body_damage,
            gem_head_body_damage = EXCLUDED.gem_head_body_damage,
            gem_torso_body_damage = EXCLUDED.gem_torso_body_damage,
            gem_limbs_body_damage = EXCLUDED.gem_limbs_body_damage,
            gem_lojak_damage = EXCLUDED.gem_lojak_damage,
            gem_takedar_damage = EXCLUDED.gem_takedar_damage,
            gem_jukk_damage = EXCLUDED.gem_jukk_damage,
            gem_sterl_damage = EXCLUDED.gem_sterl_damage,
            gem_mohaca_damage = EXCLUDED.gem_mohaca_damage,
            gem_terro_damage = EXCLUDED.gem_terro_damage,
            gem_klonk_damage = EXCLUDED.gem_klonk_damage,
            gem_priker_damage = EXCLUDED.gem_priker_damage,
            gem_base_burst_damage = EXCLUDED.gem_base_burst_damage,
            gem_burst_lojak_damage = EXCLUDED.gem_burst_lojak_damage,
            gem_burst_takedar_damage = EXCLUDED.gem_burst_takedar_damage,
            gem_burst_jukk_damage = EXCLUDED.gem_burst_jukk_damage,
            gem_burst_sterl_damage = EXCLUDED.gem_burst_sterl_damage,
            gem_burst_mohaca_damage = EXCLUDED.gem_burst_mohaca_damage,
            gem_burst_terro_damage = EXCLUDED.gem_burst_terro_damage,
            gem_burst_klonk_damage = EXCLUDED.gem_burst_klonk_damage,
            gem_burst_priker_damage = EXCLUDED.gem_burst_priker_damage,
            gem_base_affliction_damage = EXCLUDED.gem_base_affliction_damage,
            gem_affliction_lojak_damage = EXCLUDED.gem_affliction_lojak_damage,
            gem_affliction_takedar_damage = EXCLUDED.gem_affliction_takedar_damage,
            gem_affliction_jukk_damage = EXCLUDED.gem_affliction_jukk_damage,
            gem_affliction_sterl_damage = EXCLUDED.gem_affliction_sterl_damage,
            gem_affliction_mohaca_damage = EXCLUDED.gem_affliction_mohaca_damage,
            gem_affliction_terro_damage = EXCLUDED.gem_affliction_terro_damage,
            gem_affliction_klonk_damage = EXCLUDED.gem_affliction_klonk_damage,
            gem_affliction_priker_damage = EXCLUDED.gem_affliction_priker_damage,
            updated_at = NOW()
        RETURNING {PLAYER_STATS_COLUMNS}"
    ))
    .bind(player_id)
    .bind(data.player_raid_level as i32)
    .bind(data.player_raid_base_damage as i32)
    .bind(data.title)
    .bind(raid_set.jade_anniversary)
    .bind(raid_set.jukk_juggernaut)
    .bind(raid_set.airforce_ace)
    .bind(raid_set.dancer_venom)
    .bind(raid_set.rose_anniversary)
    .bind(titan.head_mult)
    .bind(titan.torso_mult)
    .bind(titan.limbs_mult)
    .bind(titan.armor_mult)
    .bind(titan.body_mult)
    .bind(titan.lojak_mult)
    .bind(titan.takedar_mult)
    .bind(titan.jukk_mult)
    .bind(titan.sterl_mult)
    .bind(titan.mohaca_mult)
    .bind(titan.terro_mult)
    .bind(titan.klonk_mult)
    .bind(titan.priker_mult)
    .bind(research.base_damage as i32)
    .bind(research.head_damage as i32)
    .bind(research.torso_damage as i32)
    .bind(research.limbs_damage as i32)
    .bind(research.armor_damage as i32)
    .bind(research.head_armor_damage as i32)
    .bind(research.torso_armor_damage as i32)
    .bind(research.limbs_armor_damage as i32)
    .bind(research.body_damage as i32)
    .bind(research.head_body_damage as i32)
    .bind(research.torso_body_damage as i32)
    .bind(research.limbs_body_damage as i32)
    .bind(research.lojak_damage as i32)
    .bind(research.takedar_damage as i32)
    .bind(research.jukk_damage as i32)
    .bind(research.sterl_damage as i32)
    .bind(research.mohaca_damage as i32)
    .bind(research.terro_damage as i32)
    .bind(research.klonk_damage as i32)
    .bind(research.priker_damage as i32)
    .bind(research.base_burst_damage as i32)
    .bind(research.burst_lojak_damage as i32)
    .bind(research.burst_takedar_damage as i32)
    .bind(research.burst_jukk_damage as i32)
    .bind(research.burst_sterl_damage as i32)
    .bind(research.burst_mohaca_damage as i32)
    .bind(research.burst_terro_damage as i32)
    .bind(research.burst_klonk_damage as i32)
    .bind(research.burst_priker_damage as i32)
    .bind(research.base_affliction_damage as i32)
    .bind(research.affliction_lojak_damage as i32)
    .bind(research.affliction_takedar_damage as i32)
    .bind(research.affliction_jukk_damage as i32)
    .bind(research.affliction_sterl_damage as i32)
    .bind(research.affliction_mohaca_damage as i32)
    .bind(research.affliction_terro_damage as i32)
    .bind(research.affliction_klonk_damage as i32)
    .bind(research.affliction_priker_damage as i32)
    .bind(gem.base_damage as i32)
    .bind(gem.head_damage as i32)
    .bind(gem.torso_damage as i32)
    .bind(gem.limbs_damage as i32)
    .bind(gem.armor_damage as i32)
    .bind(gem.head_armor_damage as i32)
    .bind(gem.torso_armor_damage as i32)
    .bind(gem.limbs_armor_damage as i32)
    .bind(gem.body_damage as i32)
    .bind(gem.head_body_damage as i32)
    .bind(gem.torso_body_damage as i32)
    .bind(gem.limbs_body_damage as i32)
    .bind(gem.lojak_damage as i32)
    .bind(gem.takedar_damage as i32)
    .bind(gem.jukk_damage as i32)
    .bind(gem.sterl_damage as i32)
    .bind(gem.mohaca_damage as i32)
    .bind(gem.terro_damage as i32)
    .bind(gem.klonk_damage as i32)
    .bind(gem.priker_damage as i32)
    .bind(gem.base_burst_damage as i32)
    .bind(gem.burst_lojak_damage as i32)
    .bind(gem.burst_takedar_damage as i32)
    .bind(gem.burst_jukk_damage as i32)
    .bind(gem.burst_sterl_damage as i32)
    .bind(gem.burst_mohaca_damage as i32)
    .bind(gem.burst_terro_damage as i32)
    .bind(gem.burst_klonk_damage as i32)
    .bind(gem.burst_priker_damage as i32)
    .bind(gem.base_affliction_damage as i32)
    .bind(gem.affliction_lojak_damage as i32)
    .bind(gem.affliction_takedar_damage as i32)
    .bind(gem.affliction_jukk_damage as i32)
    .bind(gem.affliction_sterl_damage as i32)
    .bind(gem.affliction_mohaca_damage as i32)
    .bind(gem.affliction_terro_damage as i32)
    .bind(gem.affliction_klonk_damage as i32)
    .bind(gem.affliction_priker_damage as i32)
    .fetch_one(&mut **tx)
    .await?;

    sqlx::query("DELETE FROM player_cards WHERE player_id = $1")
        .bind(player_id)
        .execute(&mut **tx)
        .await?;
    for card in &data.card_list {
        sqlx::query(
            "INSERT INTO player_cards (player_id, card_id, level, enabled) VALUES ($1,$2,$3,$4)",
        )
        .bind(player_id)
        .bind(card.card_id)
        .bind(card.level as i32)
        .bind(card.enabled)
        .execute(&mut **tx)
        .await?;
    }

    Ok(LoadedPlayerStats {
        revision: row.revision,
        data: data.clone(),
        created_at: row.created_at,
        updated_at: row.updated_at,
    })
}
