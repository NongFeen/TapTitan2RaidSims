use crate::models::{
    boss::{Boss, BossPartName},
    cards::Card,
    damage_source::DamageSource,
};
pub fn get_proc_chance(_card: &Card, _boss: &Boss) -> f64 {
    0.12
}
pub fn on_proc(card: &Card, boss: &mut Boss, target_part: BossPartName, damage: f64) {
    let psychic_mult = card.skill.value_a.unwrap_or(1.0);
    let mut total_mult = psychic_mult;
    if target_part.is_limb() {
        total_mult *= card.skill.bonus_c.unwrap_or(1.0);
    }
    boss.on_hit_with_source(
        target_part,
        (damage * total_mult).max(0.0) as u64,
        DamageSource::Card(card.card_id),
    );
}
