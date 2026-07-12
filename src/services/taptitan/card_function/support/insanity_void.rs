use crate::models::{
    boss::{Boss, PartState},
    cards::Card,
    support_modifier::SupportModifiers,
};

pub fn get_modifiers(card: &mut Card, boss: &Boss) -> SupportModifiers {
    let max_body_count = card.skill.bonus_c.unwrap_or(1.0);
    let mut body_count = 0.0;

    for part in boss.parts() {
        if part.part_state == PartState::Body && body_count < max_body_count {
            body_count += 1.0;
        }
    }

    return SupportModifiers {
        all_damage_add: card.skill.value_a.unwrap_or(1.0)
            + (card.skill.value_b.unwrap_or(0.0) * body_count),
        ..Default::default()
    };
}
