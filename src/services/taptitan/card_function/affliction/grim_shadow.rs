use crate::models::{
    affliction::Affliction,
    boss::{Boss, BossPartName, BossTickView},
    card_skill_data::card_skill_bonusamountC,
    cards::Card,
};

use super::shared;

const TICK_INTERVAL_SECONDS: f64 = 0.2;

pub fn get_proc_chance(card: &Card, boss: &Boss) -> f64 {
    shared::get_proc_chance(card, boss)
    // 1.0
}
pub fn on_proc(card: &Card, boss: &mut Boss, target_part: BossPartName, damage: f64) {
    shared::on_proc_with_tick_interval(card, boss, target_part, damage, TICK_INTERVAL_SECONDS)
}

pub fn on_tick(
    affliction: &Affliction,
    boss: &BossTickView,
    part_name: BossPartName,
    stack_multiplier: f64,
    elapsed_seconds: f64,
) -> u64 {
    let stack_mult = card_skill_bonusamountC(affliction.source_card).unwrap_or(1.2);
    let multiplier = stack_multiplier * stack_mult.powf(affliction.stack_count() as f64);

    shared::on_tick(affliction, boss, part_name, multiplier, elapsed_seconds)
} //3.16 / 7.59

pub fn on_remove(affliction: &Affliction, attached_duration: f64) -> u64 {
    shared::on_remove(affliction, attached_duration)
}
