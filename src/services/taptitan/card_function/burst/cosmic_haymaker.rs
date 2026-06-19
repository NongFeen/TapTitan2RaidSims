use crate::models::{
    boss::{Boss, BossPartName},
    cards::Card,
};

pub fn get_proc_chance(_card: &Card, _boss: &Boss) -> f64 {
    0.0
}

pub fn on_proc(
    _card: &Card,
    _boss: &mut Boss,
    _target_part: BossPartName,
    damage: f64,
) -> f64 {
    damage
}
