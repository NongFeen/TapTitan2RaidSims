use crate::models::{
    affliction::Affliction,
    boss::{Boss, BossPartName, BossTickView, PartState},
    cards::Card,
};

use super::shared;

const TICK_INTERVAL_SECONDS: f64 = 0.2;
const MAX_DAMAGE_PERCENT: f64 = 0.70;
const MIN_TICK_DAMAGE: f64 = 1.0;

pub fn get_proc_chance(card: &Card, boss: &Boss) -> f64 {
    shared::get_proc_chance(card, boss)
    // 1.0
}
pub fn on_proc(card: &Card, boss: &mut Boss, target_part: BossPartName, damage: f64) {
    shared::on_proc_with_tick_interval(card, boss, target_part, damage, TICK_INTERVAL_SECONDS)
}
// 1 tap lv48 start at max value = 282.271k - tap dmg ~=~ 277.49k

pub fn on_tick(
    affliction: &Affliction,
    boss: &BossTickView,
    part_name: BossPartName,
    stack_multiplier: f64,
    elapsed_seconds: f64,
) -> u64 {
    let part = boss.part(part_name);
    let resource_left = match part.part_state {
        PartState::Armor | PartState::Cursed => {
            if part.max_armor == 0 {
                1.0
            } else {
                part.current_armor as f64 / part.max_armor as f64
            }
        }
        PartState::Body => {
            if part.max_health == 0 {
                1.0
            } else {
                part.current_health as f64 / part.max_health as f64
            }
        }
        PartState::Skeleton => 0.0,
    };
    let damage_percent = (1.0 - resource_left).clamp(0.0, MAX_DAMAGE_PERCENT);
    let tick_damage =
        affliction.damage_per_second * damage_percent * stack_multiplier * elapsed_seconds;

    let tick_damage = tick_damage.max(MIN_TICK_DAMAGE);

    tick_damage as u64
}

pub fn on_remove(affliction: &Affliction, attached_duration: f64) -> u64 {
    shared::on_remove(affliction, attached_duration)
}
