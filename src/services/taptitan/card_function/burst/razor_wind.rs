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
    let can_hit = matches!(boss.part(target_part).part_state, PartState::Body | PartState::Skeleton);
    if can_hit { damage * 1.5 } else { damage }
}
