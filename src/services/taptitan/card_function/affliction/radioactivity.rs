use crate::models::{
    affliction::Affliction,
    boss::{Boss, BossPartName},
    card_skill_data::card_skill_bonusamountC,
    cards::Card,
};

use super::shared;

const TICK_INTERVAL_SECONDS: f64 = 0.2;

pub fn get_proc_chance(card: &Card, boss: &Boss) -> f64 {
    shared::get_proc_chance(card, boss)
    // 1.0
}

pub fn on_proc(card: &Card, boss: &mut Boss, target_part: BossPartName, damage: f64) {
    let Some(mut affliction) = shared::build_affliction(card, boss, target_part, damage, 0.0)
    else {
        return;
    };
    affliction.tick_interval_seconds = TICK_INTERVAL_SECONDS;

    boss.apply_affliction(target_part, affliction);
}

pub fn on_tick(
    affliction: &Affliction,
    boss: &Boss,
    part_name: BossPartName,
    stack_multiplier: f64,
    elapsed_seconds: f64,
) -> u64 {
    let bonus_per_second = card_skill_bonusamountC(affliction.source_card).unwrap_or(0.0);
    let afflicted_seconds = boss.part(part_name).radioactivity_afflicted_seconds;
    let ramp_multiplier = 1.0 + (bonus_per_second * afflicted_seconds);

    shared::on_tick(
        affliction,
        boss,
        part_name,
        stack_multiplier * ramp_multiplier,
        elapsed_seconds,
    )
}
pub fn on_remove(affliction: &Affliction, attached_duration: f64) -> u64 {
    shared::on_remove(affliction, attached_duration)
}
