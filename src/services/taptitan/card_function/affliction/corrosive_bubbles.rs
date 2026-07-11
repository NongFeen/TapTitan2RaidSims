use crate::models::{
    affliction::Affliction,
    boss::{Boss, BossPartName, BossTickView},
    card_skill_data::{card_skill_row, card_skill_value_b},
    cards::{Card, CardName},
    damage_source::DamageSource,
};

use super::shared;

const TICK_INTERVAL_SECONDS: f64 = 0.2;

pub fn get_proc_chance(card: &Card, boss: &Boss) -> f64 {
    shared::get_proc_chance(card, boss)
    // 1.0
}

pub fn on_proc(card: &Card, boss: &mut Boss, target_part: BossPartName, damage: f64) {
    shared::on_proc_with_tick_interval(card, boss, target_part, damage, TICK_INTERVAL_SECONDS);
    refresh_bubble_stacks(boss, target_part);

    let max_stacks = card_skill_row(card.card_id)
        .map(|row| row.max_stacks as usize)
        .unwrap_or(5);
    let pop_multiplier = card_skill_value_b(card.card_id, card.level).unwrap_or(26.0);
    let bubble_affliction = boss
        .part(target_part)
        .afflictions
        .iter()
        .find(|affliction| affliction.source_card == CardName::CorrosiveBubbles);
    let should_pop = bubble_affliction
        .map(|affliction| affliction.stack_count() >= max_stacks)
        .unwrap_or(false);

    if should_pop {
        let affliction_tick_damage = bubble_affliction
            .map(|affliction| affliction.damage_per_second * TICK_INTERVAL_SECONDS)
            .unwrap_or(0.0);
        let pop_damage = (affliction_tick_damage * pop_multiplier * max_stacks as f64) as u64;
        // println!(
        //     "[AFF POP] card={:?} part={:?} damage={} affliction_tick_damage={:.2} proc_base_damage={:.2} pop_multiplier={:.4} max_stacks={}",
        //     card.card_id,
        //     target_part,
        //     pop_damage,
        //     affliction_tick_damage,
        //     damage,
        //     pop_multiplier,
        //     max_stacks,
        // );

        boss.part_mut(target_part)
            .afflictions
            .retain(|affliction| affliction.source_card != CardName::CorrosiveBubbles);
        boss.on_hit_with_source(target_part, pop_damage, DamageSource::Card(card.card_id));
    }
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

fn refresh_bubble_stacks(boss: &mut Boss, target_part: BossPartName) {
    let Some(affliction) = boss
        .part_mut(target_part)
        .afflictions
        .iter_mut()
        .find(|affliction| affliction.source_card == CardName::CorrosiveBubbles)
    else {
        return;
    };

    let duration = affliction
        .stacks
        .iter()
        .map(|stack| stack.attached_duration)
        .fold(0.0, f64::max);
    let sands_of_time_boosted = affliction
        .stacks
        .iter()
        .any(|stack| stack.sands_of_time_boosted);

    for stack in &mut affliction.stacks {
        stack.refresh_with_sands_of_time_boost(
            duration,
            stack.sands_of_time_boosted || sands_of_time_boosted,
        );
    }
}
