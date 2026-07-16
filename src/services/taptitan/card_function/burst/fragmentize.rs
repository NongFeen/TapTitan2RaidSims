use crate::models::{
    boss::{Boss, BossPartName, PartState},
    cards::Card,
    damage_source::DamageSource,
};

pub fn get_proc_chance(_card: &Card, _boss: &Boss) -> f64 {
    0.12
}
pub fn on_proc(card: &Card, boss: &mut Boss, target_part: BossPartName, damage: f64) -> u64 {
    let frag_mult = card.skill.value_a.unwrap_or(1.0);
    let curse_mult = card.skill.value_b.unwrap_or(1.0);
    let armor_mult = card.skill.bonus_c.unwrap_or(1.0);
    let mut total_card_mult = frag_mult;
    if boss.get_state_from_part(target_part) == PartState::Cursed {
        total_card_mult *= curse_mult * armor_mult;
    } else if boss.get_state_from_part(target_part) == PartState::Armor {
        total_card_mult *= armor_mult;
    }
    // println!("{} {} {} {}",frag_mult,curse_mult,armor_mult,total_card_mult);
    let result_damage = (damage * total_card_mult).max(0.0) as u64;
    boss.on_hit_with_source(target_part, result_damage, DamageSource::Card(card.card_id));
    result_damage
}
