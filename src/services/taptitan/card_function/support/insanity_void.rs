use crate::models::{
    boss::{Boss, PartState},
    card_skill_data::{card_skill_bonusamountC, card_skill_value_a, card_skill_value_b},
    cards::Card,
    support_modifier::SupportModifiers,
};

pub fn get_modifiers(card: &mut Card, boss: &Boss) -> SupportModifiers {
    let max_body_count = card_skill_bonusamountC(card.card_id).unwrap_or(1.0);
    let mut body_count = 0.0;

    for part in boss.parts() {
        if part.part_state == PartState::Body && body_count < max_body_count {
            body_count += 1.0;
        }
    }

    return SupportModifiers {
        all_damage_add: card_skill_value_a(card.card_id, card.level).unwrap_or(1.0)
            + (card_skill_value_b(card.card_id, card.level).unwrap_or(0.0) * body_count),
        ..Default::default()
    };
}
