use crate::models::{
    boss::{Boss, BossPartName},
    cards::Card,
};

pub fn on_proc(
    _card: &Card,
    _boss: &mut Boss,
    _target_part: BossPartName,
    damage: f64,
    round_index: u32,
) -> f64 {
    if round_index == 2 { damage * 1.35 } else { damage }
}
