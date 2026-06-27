use crate::models::{
    boss::{BossName, BossPartName, PartState},
    cards::{Card, CardType},
};
use serde::{Deserialize, Serialize};
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PlayerRaidData {
    pub player_raid_level: u16,
    pub player_raid_base_damage: u16,

    pub raid_set: RaidSet,
    pub titan_soul_research: TitanSoulResearch,
    pub raid_card_research: RaidCardResearch,
    pub gem_stone_research: GemstoneResearch,

    pub card_list: Vec<Card>,
    pub title: f32,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RaidSet {
    pub jade_anniversary: bool, // +4% All Raid Damage
    pub jukk_juggernaut: bool,  // +100 Raid Base Damage
    pub airforce_ace: bool,     // +120 Raid Burst Base Damage
    pub dancer_venom: bool,     // +120 Raid Affliction Base Damage
    pub rose_anniversary: bool, // +100 Raid Base Damage
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TitanSoulResearch {
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
impl TitanSoulResearch {
    // 1. Dynamic Boss Name lookup
    pub fn get_boss_mult(&self, boss_name: BossName) -> f32 {
        match boss_name {
            BossName::Lojak => self.lojak_mult,
            BossName::Takedar => self.takedar_mult,
            BossName::Jukk => self.jukk_mult,
            BossName::Sterl => self.sterl_mult,
            BossName::Mohaca => self.mohaca_mult,
            BossName::Terro => self.terro_mult,
            BossName::Klonk => self.klonk_mult,
            BossName::Priker => self.priker_mult,
            _ => 0.0,
        }
    }

    // 2. Dynamic Anatomical Part lookup (Head vs Torso vs Limbs)
    pub fn get_part_mult(&self, part: BossPartName) -> f32 {
        if part == BossPartName::Head {
            self.head_mult
        } else if part == BossPartName::Torso {
            self.torso_mult
        } else if part.is_limb() {
            self.limbs_mult
        } else {
            0.0
        }
    }
    pub fn get_state_mult(&self, state: PartState) -> f32 {
        if state == PartState::Body {
            self.body_mult
        } else {
            self.armor_mult
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RaidCardResearch {
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
impl RaidCardResearch {
    pub fn get_part_state_add(&self, part: BossPartName, state: PartState) -> f32 {
        let part_base = match part {
            BossPartName::Head => self.head_damage,
            BossPartName::Torso => self.torso_damage,
            _ if part.is_limb() => self.limbs_damage,
            _ => 0,
        };

        let layered_base = match (part, state) {
            (BossPartName::Head, PartState::Armor) => self.armor_damage + self.head_armor_damage,
            (BossPartName::Torso, PartState::Armor) => self.armor_damage + self.torso_armor_damage,
            (p, PartState::Armor) if p.is_limb() => self.armor_damage + self.limbs_armor_damage,
            (BossPartName::Head, _) => self.body_damage + self.head_body_damage,
            (BossPartName::Torso, _) => self.body_damage + self.torso_body_damage,
            _ => self.body_damage + self.limbs_body_damage,
        };

        (part_base + layered_base) as f32
    }

    pub fn get_boss_add(&self, boss_name: BossName) -> f32 {
        let amt = match boss_name {
            BossName::Lojak => self.lojak_damage,
            BossName::Takedar => self.takedar_damage,
            BossName::Jukk => self.jukk_damage,
            BossName::Sterl => self.sterl_damage,
            BossName::Mohaca => self.mohaca_damage,
            BossName::Terro => self.terro_damage,
            BossName::Klonk => self.klonk_damage,
            BossName::Priker => self.priker_damage,
            _ => 0,
        };
        amt as f32
    }

    pub fn get_card_type_boss_add(&self, boss_name: BossName, card_type: CardType) -> f32 {
        let amt = match card_type {
            CardType::Burst => match boss_name {
                BossName::Lojak => self.burst_lojak_damage,
                BossName::Takedar => self.burst_takedar_damage,
                BossName::Jukk => self.burst_jukk_damage,
                BossName::Sterl => self.burst_sterl_damage,
                BossName::Mohaca => self.burst_mohaca_damage,
                BossName::Terro => self.burst_terro_damage,
                BossName::Klonk => self.burst_klonk_damage,
                BossName::Priker => self.burst_priker_damage,
                _ => 0,
            },
            CardType::Affliction => match boss_name {
                BossName::Lojak => self.affliction_lojak_damage,
                BossName::Takedar => self.affliction_takedar_damage,
                BossName::Jukk => self.affliction_jukk_damage,
                BossName::Sterl => self.affliction_sterl_damage,
                BossName::Mohaca => self.affliction_mohaca_damage,
                BossName::Terro => self.affliction_terro_damage,
                BossName::Klonk => self.affliction_klonk_damage,
                BossName::Priker => self.affliction_priker_damage,
                _ => 0,
            },
            _ => 0,
        };
        amt as f32
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct GemstoneResearch {
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
impl GemstoneResearch {
    pub fn get_part_state_add(&self, part: BossPartName, state: PartState) -> f32 {
        let part_base = match part {
            BossPartName::Head => self.head_damage,
            BossPartName::Torso => self.torso_damage,
            _ if part.is_limb() => self.limbs_damage,
            _ => 0,
        };

        let layered_base = match (part, state) {
            (BossPartName::Head, PartState::Armor) => self.armor_damage + self.head_armor_damage,
            (BossPartName::Torso, PartState::Armor) => self.armor_damage + self.torso_armor_damage,
            (p, PartState::Armor) if p.is_limb() => self.armor_damage + self.limbs_armor_damage,
            (BossPartName::Head, _) => self.body_damage + self.head_body_damage,
            (BossPartName::Torso, _) => self.body_damage + self.torso_body_damage,
            _ => self.body_damage + self.limbs_body_damage,
        };

        (part_base + layered_base) as f32
    }

    pub fn get_boss_add(&self, boss_name: BossName) -> f32 {
        let amt = match boss_name {
            BossName::Lojak => self.lojak_damage,
            BossName::Takedar => self.takedar_damage,
            BossName::Jukk => self.jukk_damage,
            BossName::Sterl => self.sterl_damage,
            BossName::Mohaca => self.mohaca_damage,
            BossName::Terro => self.terro_damage,
            BossName::Klonk => self.klonk_damage,
            BossName::Priker => self.priker_damage,
            _ => 0,
        };
        amt as f32
    }

    pub fn get_card_type_boss_add(&self, boss_name: BossName, card_type: CardType) -> f32 {
        let amt = match card_type {
            CardType::Burst => match boss_name {
                BossName::Lojak => self.burst_lojak_damage,
                BossName::Takedar => self.burst_takedar_damage,
                BossName::Jukk => self.burst_jukk_damage,
                BossName::Sterl => self.burst_sterl_damage,
                BossName::Mohaca => self.burst_mohaca_damage,
                BossName::Terro => self.burst_terro_damage,
                BossName::Klonk => self.burst_klonk_damage,
                BossName::Priker => self.burst_priker_damage,
                _ => 0,
            },
            CardType::Affliction => match boss_name {
                BossName::Lojak => self.affliction_lojak_damage,
                BossName::Takedar => self.affliction_takedar_damage,
                BossName::Jukk => self.affliction_jukk_damage,
                BossName::Sterl => self.affliction_sterl_damage,
                BossName::Mohaca => self.affliction_mohaca_damage,
                BossName::Terro => self.affliction_terro_damage,
                BossName::Klonk => self.affliction_klonk_damage,
                BossName::Priker => self.affliction_priker_damage,
                _ => 0,
            },
            _ => 0,
        };
        amt as f32
    }
}

impl PlayerRaidData {
    pub fn get_total_part_state_add(&self, part: BossPartName, state: PartState) -> f32 {
        self.raid_card_research.get_part_state_add(part, state)
            + self.gem_stone_research.get_part_state_add(part, state)
    }

    pub fn get_total_boss_add(&self, boss_name: BossName) -> f32 {
        self.raid_card_research.get_boss_add(boss_name)
            + self.gem_stone_research.get_boss_add(boss_name)
    }

    pub fn get_total_card_type_boss_add(&self, boss_name: BossName, card_type: CardType) -> f32 {
        self.raid_card_research
            .get_card_type_boss_add(boss_name, card_type)
            + self
                .gem_stone_research
                .get_card_type_boss_add(boss_name, card_type)
    }
}
