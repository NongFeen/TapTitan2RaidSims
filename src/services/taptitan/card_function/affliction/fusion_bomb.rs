use crate::models::{
    affliction::Affliction,
    boss::{Boss, BossPartName, BossTickView},
    cards::Card,
};

use super::{AfflictionRemoveView, shared};

const TICK_INTERVAL_SECONDS: f64 = 0.2;

pub fn get_proc_chance(card: &Card, boss: &Boss) -> f64 {
    shared::get_proc_chance(card, boss)
    // 1.0
}
//mult 73.72
//bomb dmg : 400300 ~~~ 400460
pub fn on_proc(card: &Card, boss: &mut Boss, target_part: BossPartName, damage: f64) {
    let remove_damage = damage * card.skill.value_a.unwrap_or(1.0);
    let Some(affliction) = shared::build_affliction(card, boss, target_part, damage, remove_damage)
    else {
        return;
    };
    let mut affliction = affliction;
    affliction.tick_interval_seconds = TICK_INTERVAL_SECONDS;
    // println!("remove_damage {} damage {}", remove_damage, damage);

    boss.apply_affliction(target_part, affliction);
}

pub fn on_tick(
    _affliction: &Affliction,
    _boss: &BossTickView,
    _part_name: BossPartName,
    _stack_multiplier: f64,
    _elapsed_seconds: f64,
) -> u64 {
    // 0
    // println!("Current if remove damage :{} ", affliction.remove_damage * 0.5 * affliction.stacks[0].elapsed_attached_duration);
    0
}

pub fn on_remove(affliction: &AfflictionRemoveView, total_attached_duration: f64) -> u64 {
    remove_damage(
        affliction.bonus_c,
        affliction.remove_damage,
        total_attached_duration,
    )
}

pub fn remove_damage(
    bonus_c: Option<f64>,
    base_remove_damage: f64,
    total_attached_duration: f64,
) -> u64 {
    let per_second_bonus = bonus_c.unwrap_or(0.5);
    // println!(
    //     "per_second_bonus {} total_attached_duration {}",
    //     per_second_bonus, total_attached_duration
    // );

    (base_remove_damage * (per_second_bonus * total_attached_duration)).max(0.0) as u64
}
