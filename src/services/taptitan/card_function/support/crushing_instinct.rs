use crate::models::{boss::Boss, card_skill_data::{card_skill_bonusamountC, card_skill_bonustypeC, card_skill_value_a, card_skill_value_b}, cards::Card, support_modifier::SupportModifiers};

pub fn get_modifiers(card: &mut Card,boss: &Boss,) -> SupportModifiers{

    println!("{} {} {}",card_skill_value_a(card.card_id, card.level).unwrap_or(1.0),card_skill_value_b(card.card_id, card.level).unwrap_or(1.0),card_skill_bonusamountC(card.card_id).unwrap_or(1.0) );

    return SupportModifiers { 
        head_damage_add: card_skill_value_a(card.card_id, card.level).unwrap_or(1.0),
        torso_damage_add: card_skill_value_b(card.card_id, card.level).unwrap_or(1.0),
        burst_chance_mult: card_skill_bonusamountC(card.card_id).unwrap_or(1.0),
        ..Default::default()
    };
}