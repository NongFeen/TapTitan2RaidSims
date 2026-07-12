use crate::models::{
    affliction::{Affliction, AfflictionKind},
    boss::{Boss, BossPartName},
    cards::Card,
    damage_source::DamageSource,
};
pub fn get_proc_chance(_card: &Card, _boss: &Boss) -> f64 {
    0.1
    // 1.0
}
pub fn on_proc(card: &Card, boss: &mut Boss, target_part: BossPartName, damage: f64) {
    //apply it's damage first before apply buff
    let guard_break_mult = card.skill.value_a.unwrap_or(1.0);
    boss.on_hit_with_source(
        target_part,
        (damage * guard_break_mult).max(0.0) as u64,
        DamageSource::Card(card.card_id),
    );
    //apply debuff
    if !card.skill.has_row {
        return;
    }

    let affliction = Affliction::new_with_source_skill(
        AfflictionKind::GuardBreakDebuff,
        card.card_id,
        card.level,
        card.skill,
        1,
        card.skill.duration,
        0.0,
        0.0,
        1.0,
        1,
    );

    boss.apply_affliction(target_part, affliction);
}
