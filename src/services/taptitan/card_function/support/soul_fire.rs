use crate::models::{boss::Boss, cards::Card, support_modifier::SupportModifiers};

pub fn get_modifiers(card: &mut Card, _boss: &Boss) -> SupportModifiers {
    // println!(
    //     "{} {} {}",
    //     card_skill_value_a(card.card_id, card.level).unwrap_or(1.0),
    //     card_skill_value_b(card.card_id, card.level).unwrap_or(1.0),
    //     card_skill_bonusamountC(card.card_id).unwrap_or(1.0)
    // );

    return SupportModifiers {
        head_damage_add: card.skill.value_a.unwrap_or(1.0),
        torso_damage_add: card.skill.value_b.unwrap_or(1.0),
        affliction_chance_mult: card.skill.bonus_c.unwrap_or(1.0),
        ..Default::default()
    };
}
