use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ── Top-level wrapper ──────────────────────────────────────────────

#[derive(Debug, Deserialize, Serialize)]
pub struct PlayerData {
    #[serde(rename = "playerStats")]
    pub player_stats: PlayerStats,

    #[serde(rename = "raidStats")]
    pub raid_stats: RaidStats,

    pub artifacts: HashMap<String, Artifact>,

    #[serde(rename = "splashStats")]
    pub splash_stats: SplashStats,

    #[serde(rename = "raidCards")]
    pub raid_cards: HashMap<String, Card>,

    pub raid_card_research: HashMap<String, String>,

    #[serde(rename = "titanCards")]
    pub titan_cards: HashMap<String, Card>,

    #[serde(rename = "titanResearch")]
    pub titan_research: HashMap<String, String>,

    #[serde(rename = "gemstonesResearch")]
    pub gem_research: HashMap<String, String>,
    // skill points and other dynamic maps can be added here
    #[serde(rename = "equipmentSets")]
    pub equip_set: Vec<String>,
}

// ── Player Stats ───────────────────────────────────────────────────

#[derive(Debug, Deserialize, Serialize)]
pub struct PlayerStats {
    #[serde(rename = "Max Prestige Stage")]
    pub max_prestige_stage: String,

    #[serde(rename = "Artifacts Collected")]
    pub artifacts_collected: String,

    #[serde(rename = "Crafting Power")]
    pub crafting_power: String,

    #[serde(rename = "Total Pet Levels")]
    pub total_pet_levels: String,

    #[serde(rename = "Skill Points Owned")]
    pub skill_points_owned: String,

    #[serde(rename = "Hero Weapon Upgrades")]
    pub hero_weapon_upgrades: String,

    #[serde(rename = "Clan Scroll Upgrades")]
    pub clan_scroll_upgrades: String,

    #[serde(rename = "Tournaments Joined")]
    pub tournaments_joined: String,

    #[serde(rename = "Undisputed Wins")]
    pub undisputed_wins: String,

    #[serde(rename = "Tournament Points")]
    pub tournament_points: String,

    #[serde(rename = "Lifetime Relics")]
    pub lifetime_relics: String,

    #[serde(rename = "Mementos")]
    pub mementos: String,

    #[serde(rename = "Necrobear Level")]
    pub necrobear_level: String,
}

// ── Raid Stats ─────────────────────────────────────────────────────

#[derive(Debug, Deserialize, Serialize)]
pub struct RaidStats {
    #[serde(rename = "Raid Level")]
    pub raid_level: String,

    #[serde(rename = "Raid Level Base Damage ")] // note trailing space in JSON
    pub raid_level_base_damage: String,

    #[serde(rename = "Total Raid Experience")]
    pub total_raid_experience: String,

    #[serde(rename = "Total Raid Attacks")]
    pub total_raid_attacks: String,

    #[serde(rename = "Total Raid Card Levels")]
    pub total_raid_card_levels: String,

    #[serde(rename = "Raid Cards Owned")]
    pub raid_cards_owned: String,

    #[serde(rename = "Wildcards")]
    pub wildcards: String,

    #[serde(rename = "Lifetime Clan Morale")]
    pub lifetime_clan_morale: String,

    #[serde(rename = "Solo Raid World Reached")]
    pub solo_raid_world_reached: String,
}

// ── Shared card type (used by raidCards and titanCards) ────────────

#[derive(Debug, Deserialize, Serialize)]
pub struct Card {
    pub lv: u16,
    pub num: u16,
}

// ── Artifact ───────────────────────────────────────────────────────

#[derive(Debug, Deserialize, Serialize)]
pub struct Artifact {
    pub e: u8,      // 0 = not enchanted, 1 = enchanted
    pub lv: String, // scientific notation e.g. "2.864E+106"
}

// ── Splash Stats ───────────────────────────────────────────────────

#[derive(Debug, Deserialize, Serialize)]
pub struct SplashStats {
    #[serde(rename = "Titan Skip")]
    pub titan_skip: String,

    #[serde(rename = "Pet Titan Skip")]
    pub pet_titan_skip: String,

    #[serde(rename = "Heavenly Strike Titan Skip")]
    pub heavenly_strike_titan_skip: String,

    #[serde(rename = "Shadow Clone Titan Skip")]
    pub shadow_clone_titan_skip: String,

    #[serde(rename = "Clanship Titan Skip")]
    pub clanship_titan_skip: String,

    #[serde(rename = "Dagger Titan Skip")]
    pub dagger_titan_skip: String,

    #[serde(rename = "Gold Gun Titan Skip")]
    pub gold_gun_titan_skip: String,

    #[serde(rename = "Stage Skip")]
    pub stage_skip: String,

    #[serde(rename = "Pet Stage Skip")]
    pub pet_stage_skip: String,

    #[serde(rename = "Pet Skill Stage Skip")]
    pub pet_skill_stage_skip: String,

    #[serde(rename = "Dual Pet Stage Skip")]
    pub dual_pet_stage_skip: String,

    #[serde(rename = "Heavenly Strike Stage Skip")]
    pub heavenly_strike_stage_skip: String,

    #[serde(rename = "Shadow Clone Stage Skip")]
    pub shadow_clone_stage_skip: String,

    #[serde(rename = "Clanship Stage Skip")]
    pub clanship_stage_skip: String,

    #[serde(rename = "Dagger Stage Skip")]
    pub dagger_stage_skip: String,

    #[serde(rename = "Dagger Target Stage Skip")]
    pub dagger_target_stage_skip: String,

    #[serde(rename = "Blade Stream Target Stage Skip")]
    pub blade_stream_target_stage_skip: String,

    #[serde(rename = "Magnum Opus Stage Skip")]
    pub magnum_opus_stage_skip: String,

    #[serde(rename = "Golden Missile Stage Skip")]
    pub golden_missile_stage_skip: String,

    #[serde(rename = "Twilight Fairy Stage Skip")]
    pub twilight_fairy_stage_skip: String,
}
