use crate::models::{
    boss::{Boss, BossPartName}, card_skill_data::card_skill_value_a, cards::Card,
};

pub fn get_proc_chance(_card: &Card, boss: &Boss) -> f64 {
    let afflicted_parts = boss
        .parts()
        .into_iter()
        .filter(|part| !part.afflictions.is_empty())
        .count() as f64;

    (0.02 + (0.02 * afflicted_parts)).min(0.12)
}

pub fn on_proc(
    card: &Card,
    boss: &mut Boss,
    target_part: BossPartName,
    damage: f64,
) -> f64 {
    let whip_mult = card_skill_value_a(card.card_id, card.level).unwrap_or(1.0);
    return  damage * whip_mult;
}
