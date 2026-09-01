use crate::models::{
    boss::{Boss, BossPartName},
    cards::Card,
    damage_source::DamageSource,
};

pub fn get_proc_chance(_card: &Card, _boss: &Boss) -> f64 {
    0.12
}

pub fn bonus_multiplier(target_part: BossPartName, bonus_c: Option<f64>, bonus_d: Option<f64>) -> f64 {
    match target_part {
        BossPartName::Torso => bonus_c.unwrap_or(1.0),
        BossPartName::LeftHand
        | BossPartName::RightHand
        | BossPartName::RightShoulder
        | BossPartName::LeftShoulder => bonus_d.unwrap_or(1.0),
        _ => 1.0,
    }
}

pub fn on_proc(card: &Card, boss: &mut Boss, target_part: BossPartName, damage: f64) -> f64 {
    let moonbeam_mult = card.skill.value_a.unwrap_or(1.0);
    let bonus_correct_part = bonus_multiplier(target_part, card.skill.bonus_c, card.skill.bonus_d);
    let result_damage = (damage * moonbeam_mult * bonus_correct_part).max(0.0);

    boss.on_hit_with_source(target_part, result_damage, DamageSource::Card(card.card_id));
    result_damage
}

#[cfg(test)]
#[path = "../../../../../tests/unit/services/taptitan/card_function/burst/moon_beam_tests.rs"]
mod tests;
