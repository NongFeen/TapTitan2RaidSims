use crate::models::{
    boss::{Boss, BossPartName},
    cards::Card,
    damage_source::DamageSource,
};
pub fn get_proc_chance(_card: &Card, _boss: &Boss) -> f64 {
    0.12
}
pub fn on_proc(card: &Card, boss: &mut Boss, target_part: BossPartName, damage: f64) -> u64 {
    let skull_bash = card.skill.value_a.unwrap_or(1.0);
    let mut bonus_correct_part = 1.5f64;
    if target_part == BossPartName::Head
        || target_part == BossPartName::LeftLeg
        || target_part == BossPartName::RightLeg
    {
        bonus_correct_part = card.skill.bonus_c.unwrap_or(1.0);
    }
    let result_damage = (damage * skull_bash * bonus_correct_part).max(0.0) as u64;
    boss.on_hit_with_source(target_part, result_damage, DamageSource::Card(card.card_id));
    result_damage
}
