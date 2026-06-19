use crate::models::{
    boss::{Boss, BossPartName},
    cards::Card,
};

pub fn get_proc_chance(_card: &Card, boss: &Boss) -> f64 {
    let afflicted_parts = boss
        .parts()
        .into_iter()
        .filter(|part| !part.afflictions.is_empty())
        .count() as f64;

    (0.02 + (0.02 * afflicted_parts)).min(0.12)
}

pub fn on_proc(
    _card: &Card,
    _boss: &mut Boss,
    _target_part: BossPartName,
    damage: f64,
) -> f64 {
    damage
}
