use crate::models::{
    boss::{Boss, BossPartName},
    cards::Card,
    damage_source::DamageSource,
};

pub fn get_proc_chance(_card: &Card, _boss: &Boss) -> f64 {
    1.0
}

pub fn on_proc(card: &mut Card, boss: &mut Boss, target_part: BossPartName, damage: f64) -> f64 {
    card.tap_count += 1;
    // let mut card_damage: f64 =0.0;
    let cosmic_hay_mult = card.skill.value_a.unwrap_or(1.0);
    let result_damage = (damage * cosmic_hay_mult).max(0.0);
    if card.tap_count >= 70 {
        // card_damage = damage * cosmic_hay_mult;
        boss.on_hit_with_source(target_part, result_damage, DamageSource::Card(card.card_id));
        card.tap_count = 0;
    }
    result_damage
}
