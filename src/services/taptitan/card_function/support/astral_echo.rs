use crate::models::{boss::Boss, cards::Card, support_modifier::SupportModifiers};

pub fn get_modifiers(card: &mut Card, _boss: &Boss) -> SupportModifiers {
    return SupportModifiers {
        all_damage_add: card.skill.value_a.unwrap_or(1.0),
        bonus_tap_proc_chance_mult: card.skill.bonus_d.unwrap_or(0.5),
        ..Default::default()
    };
}
