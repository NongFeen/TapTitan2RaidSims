use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum PartState {
    Armored,
    Body,
    Cursed,
    Broken,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum PartId {
    LeftHand,
    RightHand,
    LeftShoulder,
    RightShoulder,
    LeftLeg,
    RightLeg,
    Head,
    Torso,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct BossPart {
    pub id: PartId,
    pub state: PartState,
    pub max_hp: f64,
    pub current_hp: f64,
    pub max_armor: f64,
    pub current_armor: f64,
    pub is_cursed: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct DamageResult {
    pub armor_damage: f64,
    pub hp_damage: f64,
    pub armor_broken: bool,
    pub part_broken: bool,
    pub was_blocked: bool,
    pub real_hp_remaining: f64,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Boss {
    pub name: String,
    pub real_hp: f64,
    pub max_real_hp: f64,
    pub parts: Vec<BossPart>,
}