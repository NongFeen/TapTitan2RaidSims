use crate::models::{
    affliction::{Affliction, AfflictionKind},
    boss::{Boss, BossPartName},
    card_skill_data::{card_skill_row, card_skill_value_a},
    cards::Card,
    damage_source::DamageSource,
};
pub fn get_proc_chance(_card: &Card, _boss: &Boss) -> f64 {
    // 0.1
    1.0
}
pub fn on_proc(card: &Card, boss: &mut Boss, target_part: BossPartName, damage: f64) {
    //apply it's damage first before apply buff
    let guard_break_mult = card_skill_value_a(card.card_id, card.level).unwrap_or(1.0);
    boss.on_hit_with_source(
        target_part,
        (damage * guard_break_mult).max(0.0) as u64,
        DamageSource::Card(card.card_id),
    );
    //apply debuff
    let Some(row) = card_skill_row(card.card_id) else {
        return;
    };

    let affliction = Affliction::new(
        AfflictionKind::GuardBreakDebuff,
        card.card_id,
        card.level,
        1,
        row.duration,
        0.0,
        0.0,
        1.0,
        1,
    );

    boss.apply_affliction(target_part, affliction);
}
