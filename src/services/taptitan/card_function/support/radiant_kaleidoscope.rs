use crate::models::{
    boss::Boss,
    cards::{Card, CardType},
    support_modifier::SupportModifiers,
};

pub fn get_modifiers(card: &mut Card, _boss: &Boss, deck: &[Card]) -> SupportModifiers {
    let has_burst = deck.iter().any(|c| c.cardtype == CardType::Burst);
    let has_affliction = deck.iter().any(|c| c.cardtype == CardType::Affliction);

    if !has_burst || !has_affliction {
        return SupportModifiers::default();
    }

    return SupportModifiers {
        burst_damage_add: card.skill.value_b.unwrap_or(1.0),
        affliction_damage_add: card.skill.value_a.unwrap_or(1.0),
        ..Default::default()
    };
}
