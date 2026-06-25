use crate::models::{
    boss::{Boss, BossPartName}, card_skill_data::card_skill_value_a, cards::Card,
};

pub fn get_proc_chance(_card: &Card, _boss: &Boss) -> f64 {
    1.0
}

pub fn on_proc(
    card: &mut Card,
    boss: &mut Boss,
    target_part: BossPartName,
    damage: f64,
) -> f64 {
    card.tap_count +=1;
    let mut card_damage: f64 =0.0; 
    if(card.tap_count >= 70){
        let cosmic_hay_mult = card_skill_value_a(card.card_id, card.level).unwrap_or(1.0);
        card_damage = damage * cosmic_hay_mult;
        card.tap_count = 0;
    }
    return  card_damage;
}
