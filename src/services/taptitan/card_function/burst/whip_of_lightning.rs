use crate::models::{
    boss::{Boss, BossPartName}, card_skill_data::card_skill_value_a, cards::Card, damage_source::DamageSource,
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
){
    let whip_mult = card_skill_value_a(card.card_id, card.level).unwrap_or(1.0);
    boss.on_hit_with_source(
        target_part,
        (damage * whip_mult).max(0.0).round() as u64,
        DamageSource::Card(card.card_id),
    );
}
