use crate::models::{
    affliction::Affliction,
    boss::{Boss, BossPartName, BossTickView},
    cards::{Card, CardName},
};

use super::{AfflictionRemoveView, shared};

const TICK_INTERVAL_SECONDS: f64 = 0.2;

pub fn get_proc_chance(card: &Card, boss: &Boss) -> f64 {
    if !card.skill.has_row {
        return 0.0;
    }

    let affected_parts = boss
        .parts()
        .iter()
        .filter(|part| {
            boss.afflictions(part.part_name)
                .iter()
                .any(|affliction| affliction.source_card == CardName::BlazingInferno)
        })
        .count() as f64;
    let bonus_per_part = card.skill.bonus_c.unwrap_or(0.0);
    let max_parts = card.skill.bonus_d.unwrap_or(0.0);
    let chance = card.skill.chance + bonus_per_part * affected_parts.min(max_parts);

    // chance.min(row.max_chance.max(row.chance))
    chance.min(card.skill.max_chance.max(card.skill.chance))
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
    shared::on_tick(
        affliction,
        boss,
        part_name,
        stack_multiplier,
        elapsed_seconds,
    )
}
pub fn on_remove(affliction: &AfflictionRemoveView, attached_duration: f64) -> f64 {
    shared::on_remove(affliction, attached_duration)
}
