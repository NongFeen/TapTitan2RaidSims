use crate::models::{
    affliction::Affliction,
    boss::{Boss, BossPartName, BossTickView},
    card_skill_data::{card_skill_row, card_skill_value_a, card_skill_value_b},
    cards::{Card, CardName},
};

pub(super) fn get_proc_chance(card: &Card, _boss: &Boss) -> f64 {
    let Some(row) = card_skill_row(card.card_id) else {
        return 0.0;
    };

    row.chance.min(row.max_chance.max(row.chance))
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
    let row = card_skill_row(card.card_id)?;
    let mut duration = row.duration;
    let mut sands_of_time_boosted = false;

    if card.card_id != CardName::SandsOfTime {
        if let Some(duration_bonus) = sands_duration_bonus(boss, target_part) {
            duration *= 1.0 + duration_bonus;
            sands_of_time_boosted = true;
        }
    }

    let damage_rate = card_skill_value_a(card.card_id, card.level).unwrap_or(1.0);
    let mut affliction = Affliction::new(
        kind,
        card.card_id,
        card.level,
        1,
        duration,
        damage * damage_rate,
        remove_damage,
        1.0,
        row.max_stacks.max(1),
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

pub(super) fn on_remove(_affliction: &Affliction, _attached_duration: f64) -> u64 {
    0
}

fn sands_duration_bonus(boss: &Boss, target_part: BossPartName) -> Option<f64> {
    boss.part(target_part)
        .afflictions
        .iter()
        .find(|affliction| {
            affliction.source_card == CardName::SandsOfTime
                && affliction
                    .stacks
                    .iter()
                    .any(|stack| stack.remaining_duration > 0.0)
        })
        .and_then(|affliction| {
            card_skill_value_b(affliction.source_card, affliction.source_level)
        })
}
