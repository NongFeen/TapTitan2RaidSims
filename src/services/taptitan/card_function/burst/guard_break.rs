use crate::{
    models::{
        affliction::AfflictionKind,
        boss::{Boss, BossPartName},
        card_skill_data::card_skill_value_a,
        cards::Card,
        damage_source::DamageSource,
    },
    services::taptitan::card_function::burst::guard_break,
};
pub fn get_proc_chance(_card: &Card, _boss: &Boss) -> f64 {
    0.1
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
    // boss.apply_affliction(target_part, AfflictionKind::GuardBreakDebuff);
}
