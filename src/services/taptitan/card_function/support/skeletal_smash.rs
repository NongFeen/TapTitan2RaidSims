use crate::models::{
    boss::{Boss, PartState},
    cards::Card,
    support_modifier::SupportModifiers,
};

pub fn get_modifiers(card: &mut Card, boss: &Boss) -> SupportModifiers {
    let mut armor_bonus = card.skill.value_a.unwrap_or(0.0);
    let has_skeleton = boss
        .parts()
        .iter()
        .any(|part| boss.get_state_from_part(part.part_name) == PartState::Skeleton);

    // 3. Add value B if the condition is met
    if has_skeleton {
        armor_bonus += card.skill.value_b.unwrap_or(0.0);
    }

    return SupportModifiers {
        armor_damage_add: armor_bonus,
        ..Default::default()
    };
}
