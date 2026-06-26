use crate::models::{
    boss::{Boss, BossPartName}, card_skill_data::{card_skill_bonusamountC, card_skill_bonustypeC, card_skill_value_a}, cards::Card, damage_source::DamageSource,
};
pub fn get_proc_chance(_card: &Card, _boss: &Boss) -> f64 {
    0.12
}
pub fn on_proc(
    card: &Card,
    boss: &mut Boss,
    target_part: BossPartName,
    damage: f64,
){
    let skull_bash = card_skill_value_a(card.card_id, card.level).unwrap_or(1.0);
    let mut bonus_correct_part = 1.5f64;
    if target_part == BossPartName::Head||
    target_part == BossPartName::LeftLeg||
    target_part == BossPartName::RightLeg{
        bonus_correct_part = card_skill_bonusamountC(card.card_id).unwrap_or(1.0);
    }

    boss.on_hit_with_source(
        target_part,
        (damage * skull_bash * bonus_correct_part).max(0.0) as u64,
        DamageSource::Card(card.card_id),
    );
}
