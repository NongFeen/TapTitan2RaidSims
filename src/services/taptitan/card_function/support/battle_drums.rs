use crate::models::{boss::Boss, cards::Card, support_modifier::SupportModifiers};

pub fn get_modifiers(card: &mut Card, _boss: &Boss) -> SupportModifiers {
    SupportModifiers {
        all_damage_add: card.skill.value_a.unwrap_or(0.0),
        ..Default::default()
    }
}
