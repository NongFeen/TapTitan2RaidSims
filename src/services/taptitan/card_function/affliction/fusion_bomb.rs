use crate::models::{
    affliction::Affliction,
    boss::{Boss, BossPartName},
    card_skill_data::{card_skill_bonusamountC, card_skill_value_a},
    cards::Card,
};

use super::shared;

const TICK_INTERVAL_SECONDS: f64 = 1.0;

pub fn get_proc_chance(card: &Card, boss: &Boss) -> f64 {
    shared::get_proc_chance(card, boss)
}

pub fn on_proc(card: &Card, boss: &mut Boss, target_part: BossPartName, damage: f64) {
    let remove_damage = damage * card_skill_value_a(card.card_id, card.level).unwrap_or(1.0);
    let Some(affliction) = shared::build_affliction(card, boss, target_part, damage, remove_damage)
    else {
        return;
    };
    let mut affliction = affliction;
    affliction.tick_interval_seconds = TICK_INTERVAL_SECONDS;

    boss.apply_affliction(target_part, affliction);
}

pub fn on_tick(
    _affliction: &Affliction,
    _boss: &Boss,
    _part_name: BossPartName,
    _stack_multiplier: f64,
    _elapsed_seconds: f64,
) -> u64 {
    0
}

pub fn on_remove(affliction: &Affliction, attached_duration: f64) -> u64 {
    let per_second_bonus = card_skill_bonusamountC(affliction.source_card).unwrap_or(0.5);
    (affliction.remove_damage * (1.0 + per_second_bonus * attached_duration)).max(0.0) as u64
}
