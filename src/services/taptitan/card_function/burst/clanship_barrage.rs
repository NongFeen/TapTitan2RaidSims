use crate::models::{
    boss::{Boss, BossPartName}, card_skill_data::card_skill_value_a, cards::Card,
};

pub fn get_proc_chance(_card: &Card, _boss: &Boss) -> f64 {
    0.10
}

pub fn on_proc(
    card: &Card,
    boss: &mut Boss,
    target_part: BossPartName,
    damage: f64,
    burst_trigger_count: u32,
) -> f64 {
    let barrage_mult = card_skill_value_a(card.card_id, card.level).unwrap_or(1.0);
    damage * barrage_mult * (1.0 + (0.04 * burst_trigger_count as f64))
    
}
