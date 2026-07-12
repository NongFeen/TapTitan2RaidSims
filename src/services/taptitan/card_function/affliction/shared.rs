use crate::models::{
    affliction::Affliction,
    boss::{Boss, BossPartName, BossTickView},
    cards::{Card, CardName},
};

use super::AfflictionRemoveView;

pub(super) fn get_proc_chance(card: &Card, _boss: &Boss) -> f64 {
    if !card.skill.has_row {
        return 0.0;
    }

    card.skill
        .chance
        .min(card.skill.max_chance.max(card.skill.chance))
}

pub(super) fn on_proc(card: &Card, boss: &mut Boss, target_part: BossPartName, damage: f64) {
    on_proc_with_tick_interval(card, boss, target_part, damage, 1.0);
}

pub(super) fn on_proc_with_tick_interval(
    card: &Card,
    boss: &mut Boss,
    target_part: BossPartName,
    damage: f64,
    tick_interval_seconds: f64,
) {
    let Some(affliction) = build_affliction(card, boss, target_part, damage, 0.0) else {
        return;
    };
    let mut affliction = affliction;
    affliction.tick_interval_seconds = tick_interval_seconds;

    boss.apply_affliction(target_part, affliction);
}

pub(super) fn build_affliction(
    card: &Card,
    boss: &Boss,
    target_part: BossPartName,
    damage: f64,
    remove_damage: f64,
) -> Option<Affliction> {
    let kind = crate::models::affliction::AfflictionKind::from_card(card.card_id)?;
    if !card.skill.has_row {
        return None;
    }
    let mut duration = card.skill.duration;
    let mut sands_of_time_boosted = false;

    if card.card_id != CardName::SandsOfTime {
        if let Some(duration_bonus) = sands_duration_bonus(boss, target_part) {
            duration *= 1.0 + duration_bonus;
            sands_of_time_boosted = true;
        }
    }

    let damage_rate = card.skill.value_a.unwrap_or(1.0);
    let mut affliction = Affliction::new_with_source_skill(
        kind,
        card.card_id,
        card.level,
        card.skill,
        1,
        duration,
        damage * damage_rate,
        remove_damage,
        1.0,
        card.skill.max_stacks.max(1),
    );

    if sands_of_time_boosted {
        for stack in &mut affliction.stacks {
            stack.sands_of_time_boosted = true;
        }
    }

    Some(affliction)
}

pub(super) fn on_tick(
    affliction: &Affliction,
    _boss: &BossTickView,
    _part_name: BossPartName,
    stack_multiplier: f64,
    elapsed_seconds: f64,
) -> u64 {
    // println!("[AFF] {} Damage before bossmult ticks {}",affliction.source_card.display_name(),affliction.damage_per_second * stack_multiplier * elapsed_seconds);
    (affliction.damage_per_second * stack_multiplier * elapsed_seconds).max(0.0) as u64
}

pub(super) fn on_remove(_affliction: &AfflictionRemoveView, _attached_duration: f64) -> u64 {
    0
}

fn sands_duration_bonus(boss: &Boss, target_part: BossPartName) -> Option<f64> {
    boss.afflictions(target_part)
        .iter()
        .find(|affliction| {
            affliction.source_card == CardName::SandsOfTime
                && affliction
                    .stacks
                    .iter()
                    .any(|stack| stack.remaining_duration > 0.0)
        })
        .and_then(|affliction| {
            affliction.source_skill.value_b
        })
}
