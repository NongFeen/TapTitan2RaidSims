use crate::models::{
    boss::Boss,
    card_skill_data::{
        card_skill_bonusamountC, card_skill_bonustypeC, card_skill_value_a, card_skill_value_b,
    },
    cards::Card,
    support_modifier::SupportModifiers,
};

pub fn get_modifiers(card: &mut Card, boss: &Boss) -> SupportModifiers {
    return SupportModifiers {
        armor_damage_add: card_skill_value_a(card.card_id, card.level).unwrap_or(1.0),
        ..Default::default()
    };
}
