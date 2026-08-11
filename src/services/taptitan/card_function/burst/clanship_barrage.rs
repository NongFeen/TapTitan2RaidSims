use crate::models::{
    boss::{Boss, BossPartName},
    cards::Card,
    damage_source::DamageSource,
};

pub fn get_proc_chance(_card: &Card, _boss: &Boss) -> f64 {
    0.10
}

pub fn on_proc(
    card: &Card,
    boss: &mut Boss,
    target_part: BossPartName,
    damage: f64,
    burst_trigger_count: u32,
) -> f64 {
    let barrage_mult = card.skill.value_a.unwrap_or(1.0);
    let result_damage =
        (damage * barrage_mult * (1.0 + (0.04 * burst_trigger_count as f64))).max(0.0);
    boss.on_hit_with_source(target_part, result_damage, DamageSource::Card(card.card_id));
    result_damage
}
