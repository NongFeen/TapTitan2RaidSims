use crate::models::{
    boss::{Boss, BossPartName},
    cards::Card,
};

pub fn get_proc_chance(_card: &Card, _boss: &Boss) -> f64 {
    0.10
}

pub fn on_proc(
    _card: &Card,
    _boss: &mut Boss,
    _target_part: BossPartName,
    damage: f64,
    burst_trigger_count: u32,
) -> f64 {
    damage * (1.0 + (0.04 * burst_trigger_count as f64))
}
