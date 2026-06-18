use crate::models::{
    boss::{Boss, BossPartName},
    cards::Card,
};

pub fn on_proc(
    _card: &Card,
    boss: &mut Boss,
    target_part: BossPartName,
    damage: f64,
) -> f64 {
    let target = boss.part_mut(target_part);
    let had_affliction = !target.afflictions.is_empty();
    if had_affliction {
        target.afflictions.clear();
    }
    if had_affliction { damage * 2.0 } else { damage }
}
