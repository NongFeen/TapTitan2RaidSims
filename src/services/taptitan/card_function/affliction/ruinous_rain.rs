use crate::models::{
    affliction::Affliction,
    boss::{Boss, BossPartName, BossTickView, PartState},
    cards::Card,
};

use super::{AfflictionRemoveView, shared};

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
) -> f64 {
    let multiplier = if boss.part(part_name).part_state == PartState::Cursed {
        stack_multiplier * affliction.source_skill.bonus_c.unwrap_or(1.5)
    } else {
        stack_multiplier
    };

    shared::on_tick(affliction, boss, part_name, multiplier, elapsed_seconds)
}

pub fn on_remove(affliction: &AfflictionRemoveView, attached_duration: f64) -> f64 {
    shared::on_remove(affliction, attached_duration)
}
