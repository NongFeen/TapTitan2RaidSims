use crate::models::{
    boss::{Boss, BossPartName, PartState},
    cards::Card,
};

pub fn on_proc(
    _card: &Card,
    boss: &mut Boss,
    target_part: BossPartName,
    damage: f64,
) -> f64 {
    let damage_multiplier = match boss.part(target_part).part_state {
        PartState::Armor => 1.1,
        PartState::Cursed => 1.5,
        _ => 1.0,
    };
    damage * damage_multiplier
}
