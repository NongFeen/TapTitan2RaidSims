use crate::models::{
    boss::{Boss, BossPartName},
    cards::Card,
};

pub fn roll_proc_chance(_card: &Card, _boss: &Boss, tap_count: u32) -> f64 {
    if tap_count > 0 && tap_count % 70 == 0 {
        1.0
    } else {
        0.0
    }
}

pub fn on_proc(
    _card: &Card,
    _boss: &mut Boss,
    _target_part: BossPartName,
    damage: f64,
    _tap_count: u32,
) -> f64 {
    damage
}
