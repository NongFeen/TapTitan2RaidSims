use crate::models::{
    boss::{Boss, BossPartName},
    cards::{Card, CardName, CardType},
};

mod burst;

#[derive(Debug, Clone)]
pub struct CardProcSnapshot {
    pub card_id: CardName,
    pub proc_chance: f64,
    pub damage_multiplier: f64,
    pub notes: Vec<String>,
}

pub struct CardFunction;

impl CardFunction {
    pub fn roll_proc_chance(card: &Card, boss: &Boss) -> f64 {
        debug_assert_eq!(card.cardtype, CardType::Burst);
        burst::roll_proc_chance(card, boss)
    }

    pub fn on_proc(card: &Card, boss: &mut Boss, target_part: BossPartName) -> CardProcSnapshot {
        debug_assert_eq!(card.cardtype, CardType::Burst);
        burst::on_proc(card, boss, target_part)
    }
}
