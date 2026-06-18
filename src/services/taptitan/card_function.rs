use crate::models::{
    boss::{Boss, BossPartName},
    cards::{Card, CardType},
};

mod burst;

pub struct CardFunction;

impl CardFunction {
    pub fn roll_proc_chance(card: &Card, boss: &Boss, tap_count: u32) -> f64 {
        debug_assert_eq!(card.cardtype, CardType::Burst);
        burst::roll_proc_chance(card, boss, tap_count)
    }

    pub fn on_proc(
        card: &Card,
        boss: &mut Boss,
        target_part: BossPartName,
        damage: f64,
        round_index: u32,
        tap_count: u32,
        burst_trigger_count: u32,
    ) -> f64 {
        debug_assert_eq!(card.cardtype, CardType::Burst);
        burst::on_proc(
            card,
            boss,
            target_part,
            damage,
            round_index,
            tap_count,
            burst_trigger_count,
        )
    }
}
