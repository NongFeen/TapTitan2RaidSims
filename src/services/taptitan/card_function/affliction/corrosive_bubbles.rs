use crate::models::{
    affliction::Affliction,
    boss::{Boss, BossPartName},
    card_skill_data::{card_skill_row, card_skill_value_b},
    cards::{Card, CardName},
    damage_source::DamageSource,
};

use super::shared;

const TICK_INTERVAL_SECONDS: f64 = 1.0;

pub fn get_proc_chance(card: &Card, boss: &Boss) -> f64 {
    shared::get_proc_chance(card, boss)
}

pub fn on_proc(card: &Card, boss: &mut Boss, target_part: BossPartName, damage: f64) {
    shared::on_proc_with_tick_interval(card, boss, target_part, damage, TICK_INTERVAL_SECONDS);

    let max_stacks = card_skill_row(card.card_id)
        .map(|row| row.max_stacks as usize)
        .unwrap_or(5);
    let pop_multiplier = card_skill_value_b(card.card_id, card.level).unwrap_or(26.0);
    let should_pop = boss
        .part(target_part)
        .afflictions
        .iter()
        .find(|affliction| affliction.source_card == CardName::CorrosiveBubbles)
        .map(|affliction| affliction.stack_count() >= max_stacks)
        .unwrap_or(false);

    if should_pop {
        boss.part_mut(target_part)
            .afflictions
            .retain(|affliction| affliction.source_card != CardName::CorrosiveBubbles);
        boss.on_hit_with_source(
            target_part,
            (damage * pop_multiplier).max(0.0) as u64,
            DamageSource::Card(card.card_id),
        );
    }
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
