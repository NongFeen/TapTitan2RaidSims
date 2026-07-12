use crate::models::{
    boss::{Boss, PartState},
    cards::Card,
    support_modifier::SupportModifiers,
};

pub fn get_modifiers(card: &mut Card, boss: &Boss) -> SupportModifiers {
    let max_skeleton_count = card.skill.bonus_d.unwrap_or(1.0);
    let mut skeleton_count = 0.0;

    for part in boss.parts() {
        if part.part_state == PartState::Skeleton && skeleton_count < max_skeleton_count {
            skeleton_count += 1.0;
        }
    }

    return SupportModifiers {
        all_damage_add: card.skill.value_a.unwrap_or(1.0)
            + (card.skill.value_b.unwrap_or(0.0) * skeleton_count),
        ..Default::default()
    };
}
