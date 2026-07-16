use crate::models::{boss::Boss, cards::Card, support_modifier::SupportModifiers};

const DEFAULT_ATTACK_DURATION_ADD_SECONDS: f64 = -10.0;

pub fn get_modifiers(card: &mut Card, _boss: &Boss) -> SupportModifiers {
    SupportModifiers {
        all_damage_add: card.skill.value_a.unwrap_or(0.0),
        attack_duration_add_seconds: card
            .skill
            .value_b
            .unwrap_or(DEFAULT_ATTACK_DURATION_ADD_SECONDS),
        ..Default::default()
    }
}
