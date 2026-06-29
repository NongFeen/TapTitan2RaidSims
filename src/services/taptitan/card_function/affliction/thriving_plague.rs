use crate::models::{
    affliction::Affliction,
    boss::{Boss, BossPartName},
    card_skill_data::{card_skill_bonusamountC, card_skill_row},
    cards::{Card, CardName},
};

use super::shared;

const TICK_INTERVAL_SECONDS: f64 = 0.2;

pub fn get_proc_chance(card: &Card, boss: &Boss) -> f64 {
    // shared::get_proc_chance(card, boss)
    1.0
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
    let afflicted_parts = boss
        .parts()
        .iter()
        .filter(|part| {
            part.afflictions
                .iter()
                .any(|existing| existing.source_card == CardName::ThrivingPlague)
        })
        .count() as f64;
    let bonus = card_skill_bonusamountC(affliction.source_card).unwrap_or(0.0);
    let max_parts = card_skill_row(affliction.source_card)
        .map(|row| row.bonus_amount_d)
        .unwrap_or(6.0);
    let multiplier = stack_multiplier * (1.0 + bonus * afflicted_parts.min(max_parts));

    shared::on_tick(affliction, boss, part_name, multiplier, elapsed_seconds)
}

pub fn on_remove(affliction: &Affliction, attached_duration: f64) -> u64 {
    shared::on_remove(affliction, attached_duration)
}
