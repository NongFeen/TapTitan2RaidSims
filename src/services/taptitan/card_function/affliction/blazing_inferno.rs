use crate::models::{
    affliction::Affliction,
    boss::{Boss, BossPartName},
    card_skill_data::{card_skill_bonusamountC, card_skill_row},
    cards::{Card, CardName},
};

use super::shared;

const TICK_INTERVAL_SECONDS: f64 = 0.2;

pub fn get_proc_chance(card: &Card, boss: &Boss) -> f64 {
    let Some(row) = card_skill_row(card.card_id) else {
        return 0.0;
    };

    let affected_parts = boss
        .parts()
        .iter()
        .filter(|part| {
            part.afflictions
                .iter()
                .any(|affliction| affliction.source_card == CardName::BlazingInferno)
        })
        .count() as f64;
    let bonus_per_part = card_skill_bonusamountC(card.card_id).unwrap_or(0.0);
    let max_parts = row.bonus_amount_d;
    let chance = row.chance + bonus_per_part * affected_parts.min(max_parts);

    // chance.min(row.max_chance.max(row.chance))
    // chance.min(row.max_chance.max(row.chance))
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
    shared::on_tick(
        affliction,
        boss,
        part_name,
        stack_multiplier,
        elapsed_seconds,
    )
}
pub fn on_remove(affliction: &Affliction, attached_duration: f64) -> u64 {
    shared::on_remove(affliction, attached_duration)
}
