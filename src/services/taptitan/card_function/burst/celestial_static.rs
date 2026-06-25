use crate::models::{
    boss::{Boss, BossPartName},
    cards::Card,
};

pub fn on_proc(
    _card: &Card,
    _boss: &mut Boss,
    _target_part: BossPartName,
    damage: f64,
) {
}
