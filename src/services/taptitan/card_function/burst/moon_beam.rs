use crate::models::{
    boss::{Boss, BossPartName},
    card_skill_data::{card_skill_bonusamountC, card_skill_bonustypeC, card_skill_value_a},
    cards::Card,
};

pub fn get_proc_chance(_card: &Card, _boss: &Boss) -> f64 {
    0.12
}

pub fn on_proc(
    card: &Card,
    _boss: &mut Boss,
    target_part: BossPartName,
    damage: f64,
) -> f64 {
    let moonbeam_mult = card_skill_value_a(card.card_id, card.level).unwrap_or(1.0);
    let mut bonus_correct_part = 1.0f64;
    if target_part == BossPartName::Torso||
    target_part == BossPartName::LeftHand||
    target_part == BossPartName::RightHand||
    target_part == BossPartName::RightShoulder||
    target_part == BossPartName::LeftShoulder{
        bonus_correct_part = card_skill_bonusamountC(card.card_id).unwrap_or(1.0);
    }
    return  damage * moonbeam_mult * bonus_correct_part;
}
