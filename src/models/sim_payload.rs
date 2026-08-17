//player_cleandata + boss

use serde::{Deserialize, Serialize};

use crate::models::{
    boss::{Boss, BossPartName},
    cards::CardName,
    player_raid_data::PlayerRaidData,
};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SimPayLoad {
    pub player_raid_data: PlayerRaidData,
    pub boss_data: Boss,
    pub attackable_part: Vec<BossPartName>,
    pub usable_card: Vec<CardName>,
    #[serde(default)]
    pub include_body_phase: bool,
    /// Fractional clan boost: 0.35 means Mirror Force deals 35% more damage.
    #[serde(default)]
    pub mirror_force_boost: f64,
}
