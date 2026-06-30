use crate::models::{
    boss::{Boss, BossPartName, PartState},
    card_skill_data::{card_skill_bonusamountC, card_skill_value_a},
    cards::Card,
    damage_source::DamageSource,
};

pub fn get_proc_chance(_card: &Card, _boss: &Boss) -> f64 {
    0.12
    // 1.0
}

pub fn on_proc(card: &Card, boss: &mut Boss, target_part: BossPartName, damage: f64) {
    let razor_wind_mult = card_skill_value_a(card.card_id, card.level).unwrap_or(1.0);
    let mut total_mult = razor_wind_mult;
    if (boss.get_state_from_part(target_part) == PartState::Body) {
        total_mult *= card_skill_bonusamountC(card.card_id).unwrap_or(1.0);
    }
    boss.on_hit_with_source(
        target_part,
        (damage * total_mult).max(0.0) as u64,
        DamageSource::Card(card.card_id),
    );
}
