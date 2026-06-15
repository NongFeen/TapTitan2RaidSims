use crate::models::cards::{Card};
use serde::{Deserialize, Serialize};
#[derive(Debug,Clone, Deserialize, Serialize)]
pub struct PlayerRaidData{
    pub player_raid_level: u16,
    pub player_raid_base_damage: u16,

    pub raid_set: RaidSet,
    pub titan_soul_research: TitanSoulResearch,
    pub raid_card_research: RaidCardResearch,
    pub gem_stone_research: GemstoneResearch,

    pub card_list: Vec<Card>,
    pub title: u16
}

#[derive(Debug,Clone, Deserialize, Serialize)]
pub struct RaidSet{
    pub jade_anniversary: bool,      // +4% All Raid Damage
    pub jukk_juggernaut: bool,       // +100 Raid Base Damage
    pub airforce_ace: bool,          // +100 Raid Burst Base Damage
    pub dancer_venom: bool,          // +100 Raid Affliction Base Damage
    pub rose_anniversary: bool,      // +100 Raid Base Damage
}

#[derive(Debug,Clone, Deserialize, Serialize)]
pub struct TitanSoulResearch{
    //basic
    pub head_mult: f32,
    pub torso_mult: f32,
    pub limbs_mult: f32,
    pub armor_mult: f32,
    pub body_mult: f32,
    
    //boss
    pub lojak_mult: f32,
    pub takedar_mult: f32,
    pub jukk_mult: f32,
    pub sterl_mult: f32,
    pub mohaca_mult: f32,
    pub terro_mult: f32,
    pub klonk_mult: f32,
    pub priker_mult: f32,
}

#[derive(Debug,Clone, Deserialize, Serialize)]
pub struct RaidCardResearch{
    pub base_damage: u16,
    //boss part
    pub head_damage: u16,
    pub torso_damage: u16,
    pub limbs_damage: u16,

    //armor
    pub armor_damage: u16,
    pub head_armor_damage: u16,
    pub torso_armor_damage: u16,
    pub limbs_armor_damage: u16,

    //body
    pub body_damage: u16,
    pub head_body_damage: u16,
    pub torso_body_damage: u16,
    pub limbs_body_damage: u16,

    //boss
    pub lojak_damage: u16,
    pub takedar_damage: u16,
    pub jukk_damage: u16,
    pub sterl_damage: u16,
    pub mohaca_damage: u16,
    pub terro_damage: u16,
    pub klonk_damage: u16,
    pub priker_damage: u16,

    //burst damage
    pub base_burst_damage: u16,
    pub burst_lojak_damage: u16,
    pub burst_takedar_damage: u16,
    pub burst_jukk_damage: u16,
    pub burst_sterl_damage: u16,
    pub burst_mohaca_damage: u16,
    pub burst_terro_damage: u16,
    pub burst_klonk_damage: u16,
    pub burst_priker_damage: u16,

    //affliction damage
    pub base_affliction_damage: u16,
    pub affliction_lojak_damage: u16,
    pub affliction_takedar_damage: u16,
    pub affliction_jukk_damage: u16,
    pub affliction_sterl_damage: u16,
    pub affliction_mohaca_damage: u16,
    pub affliction_terro_damage: u16,
    pub affliction_klonk_damage: u16,
    pub affliction_priker_damage: u16, 
}

#[derive(Debug,Clone, Deserialize, Serialize)]
pub struct GemstoneResearch{
    pub base_damage: u16,

    //boss part
    pub head_damage: u16,
    pub torso_damage: u16,
    pub limbs_damage: u16,

    //armor
    pub armor_damage: u16,
    pub head_armor_damage: u16,
    pub torso_armor_damage: u16,
    pub limbs_armor_damage: u16,

    //body
    pub body_damage: u16,
    pub head_body_damage: u16,
    pub torso_body_damage: u16,
    pub limbs_body_damage: u16,

    //boss
    pub lojak_damage: u16,
    pub takedar_damage: u16,
    pub jukk_damage: u16,
    pub sterl_damage: u16,
    pub mohaca_damage: u16,
    pub terro_damage: u16,
    pub klonk_damage: u16,
    pub priker_damage: u16,

    //burst damage
    pub base_burst_damage: u16,
    pub burst_lojak_damage: u16,
    pub burst_takedar_damage: u16,
    pub burst_jukk_damage: u16,
    pub burst_sterl_damage: u16,
    pub burst_mohaca_damage: u16,
    pub burst_terro_damage: u16,
    pub burst_klonk_damage: u16,
    pub burst_priker_damage: u16,

    //affliction damage
    pub base_affliction_damage: u16,
    pub affliction_lojak_damage: u16,
    pub affliction_takedar_damage: u16,
    pub affliction_jukk_damage: u16,
    pub affliction_sterl_damage: u16,
    pub affliction_mohaca_damage: u16,
    pub affliction_terro_damage: u16,
    pub affliction_klonk_damage: u16,
    pub affliction_priker_damage: u16, 
}