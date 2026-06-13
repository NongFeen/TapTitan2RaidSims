use serde::Deserialize;
#[derive(Deserialize, Debug)]

//handle gamehive raid seed 
pub struct GameRaidPayload {
    pub spawn_sequence: Vec<String>,
    pub tier: u32,
    pub level: u32,
    pub titans: Vec<GameTitan>,
    pub area_buffs: Vec<GameBuff>,
}

#[derive(Deserialize, Debug)]
pub struct GameTitan {
    pub enemy_id: String,
    pub enemy_name: String,
    pub current_hp: f64,
    pub total_hp: f64,
    pub parts: Vec<GamePart>,
    pub area_debuffs: Vec<GameBuff>,
    pub cursed_debuffs: Vec<GameBuff>,
}

#[derive(Deserialize, Debug)]
pub struct GamePart {
    pub part_id: String, // "BodyHead", "ArmorHead", etc.
    pub current_hp: f64,
    pub total_hp: f64,
    pub cursed: bool,
}

#[derive(Deserialize, Debug)]
pub struct GameBuff {
    pub bonus_type: String,
    pub bonus_amount: f64,
}