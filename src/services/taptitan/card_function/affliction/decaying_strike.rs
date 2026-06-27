use crate::models::{
    affliction::Affliction,
    boss::{Boss, BossPartName, PartState},
    cards::Card,
};

use super::shared;

const TICK_INTERVAL_SECONDS: f64 = 1.0;

pub fn get_proc_chance(card: &Card, boss: &Boss) -> f64 {
    shared::get_proc_chance(card, boss)
}
pub fn on_proc(card: &Card, boss: &mut Boss, target_part: BossPartName, damage: f64) {
    shared::on_proc_with_tick_interval(card, boss, target_part, damage, TICK_INTERVAL_SECONDS)
}

pub fn on_tick(
    affliction: &Affliction,
    boss: &Boss,
    part_name: BossPartName,
    stack_multiplier: f64,
    elapsed_seconds: f64,
) -> u64 {
    let part = boss.part(part_name);
    let mut multiplier = stack_multiplier;

    if part.part_state == PartState::Body && part.max_health > 0 {
        let missing_health = 1.0 - (part.current_health as f64 / part.max_health as f64);
        let cap = crate::models::card_skill_data::card_skill_row(affliction.source_card)
            .map(|row| row.bonus_amount_e)
            .unwrap_or(0.7);
        multiplier *= 1.0 + missing_health.min(cap);
    }

    shared::on_tick(affliction, boss, part_name, multiplier, elapsed_seconds)
}

pub fn on_remove(affliction: &Affliction, attached_duration: f64) -> u64 {
    shared::on_remove(affliction, attached_duration)
}
