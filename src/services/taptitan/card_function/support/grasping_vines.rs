use crate::models::{boss::Boss, cards::Card, support_modifier::SupportModifiers};

pub fn get_modifiers(card: &mut Card, _boss: &Boss) -> SupportModifiers {
    return SupportModifiers {
        limb_damage_add: card.skill.value_a.unwrap_or(1.0),
        ..Default::default()
    };
}
