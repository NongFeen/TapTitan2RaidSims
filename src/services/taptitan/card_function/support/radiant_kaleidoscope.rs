use crate::models::{boss::Boss, card_skill_data::{ card_skill_value_a, card_skill_value_b}, cards::{Card, CardType}, support_modifier::SupportModifiers};

pub fn get_modifiers(card: &mut Card,boss: &Boss,deck: Vec<Card>) -> SupportModifiers{
    let has_burst = deck.iter().any(|c| c.cardtype == CardType::Burst);
    let has_affliction = deck.iter().any(|c| c.cardtype == CardType::Affliction);

    if !has_burst || !has_affliction {
        return SupportModifiers::default();
    }

    return SupportModifiers { 
        burst_damage_add: card_skill_value_b(card.card_id, card.level).unwrap_or(1.0),
        affliction_damage_add: card_skill_value_a(card.card_id, card.level).unwrap_or(1.0),
        ..Default::default()
    };
}