use crate::models::{
    affliction::Affliction,
    boss::{Boss, BossPartName},
    card_skill_data::card_skill_bonusamountC,
    cards::{Card, CardName},
};

use super::shared;

const TICK_INTERVAL_SECONDS: f64 = 1.0;

pub fn get_proc_chance(card: &Card, boss: &Boss) -> f64 {
    shared::get_proc_chance(card, boss)
}

pub fn on_proc(card: &Card, boss: &mut Boss, target_part: BossPartName, damage: f64) {
    let Some(mut affliction) = shared::build_affliction(card, boss, target_part, damage, 0.0)
    else {
        return;
    };
    affliction.tick_interval_seconds = TICK_INTERVAL_SECONDS;

    let bonus = card_skill_bonusamountC(card.card_id).unwrap_or(0.0);
    if let Some(stack) = affliction.stacks.first_mut() {
        let current_stacks = boss
            .part(target_part)
            .afflictions
            .iter()
            .find(|existing| existing.source_card == CardName::Radioactivity)
            .map(|existing| existing.stack_count())
            .unwrap_or(0);
        stack.damage_multiplier = 1.0 + (bonus * current_stacks as f64);
    }

    boss.apply_affliction(target_part, affliction);
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
