use crate::models::{
    affliction::Affliction,
    boss::{Boss, BossPartName, BossTickView},
    card_skill_data::{card_skill_row, card_skill_value_b},
    cards::{Card, CardName},
};

use super::shared;

const TICK_INTERVAL_SECONDS: f64 = 0.2;

pub fn get_proc_chance(card: &Card, boss: &Boss) -> f64 {
    shared::get_proc_chance(card, boss)
    // 0.50
}

pub fn on_proc(card: &Card, boss: &mut Boss, target_part: BossPartName, damage: f64) {
    let boost = card_skill_value_b(card.card_id, card.level).unwrap_or(1.0);
    for other in &mut boss.part_mut(target_part).afflictions {
        if other.source_card == CardName::SandsOfTime {
            continue;
        }

        let Some(row) = card_skill_row(other.source_card) else {
            continue;
        };
        let base_duration = row.duration;
        let duration_bonus = base_duration * boost;
        let boosted_max_duration = base_duration * (1.0 + boost);

        for stack in &mut other.stacks {
            stack.remaining_duration =
                (stack.remaining_duration + duration_bonus).min(boosted_max_duration);
            stack.attached_duration = stack.attached_duration.max(boosted_max_duration);
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
