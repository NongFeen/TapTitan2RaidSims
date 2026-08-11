use crate::models::{
    affliction::Affliction,
    boss::{Boss, BossPartName, BossTickView},
    cards::{Card, CardName, CardType},
};

use super::{AfflictionRemoveView, shared};

const TICK_INTERVAL_SECONDS: f64 = 0.2;

pub fn get_proc_chance(card: &Card, boss: &Boss) -> f64 {
    shared::get_proc_chance(card, boss)
    // 1.00
    // 0.50
    // 0.00
}

pub fn on_proc(card: &Card, boss: &mut Boss, target_part: BossPartName, damage: f64) {
    let boost = card.skill.value_b.unwrap_or(1.0);
    for other in boss.afflictions_mut(target_part) {
        if other.source_card == CardName::SandsOfTime
            || other.source_card.card_type() != CardType::Affliction
        {
            continue;
        }

        let base_duration = other.source_skill.duration;
        let duration_bonus = base_duration * boost;
        let boosted_max_duration = base_duration * (1.0 + boost);

        for stack in &mut other.stacks {
            if stack.sands_of_time_boosted {
                continue;
            }

            stack.remaining_duration =
                (stack.remaining_duration + duration_bonus).min(boosted_max_duration);
            stack.attached_duration = stack.attached_duration.max(boosted_max_duration);
            stack.sands_of_time_boosted = true;
        }
    }

    shared::on_proc_with_tick_interval(card, boss, target_part, damage, TICK_INTERVAL_SECONDS);
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
