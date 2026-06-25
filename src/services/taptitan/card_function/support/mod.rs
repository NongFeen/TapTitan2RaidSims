use crate::models::{
    boss::{Boss, BossPartName}, cards::{Card, CardName}, support_modifier::SupportModifiers,
};
mod crushing_instinct;

pub fn get_proc_chance(card: &Card, boss: &Boss) -> f64 {
    match card.card_id {
        _ => 0.0
    }
}

pub fn on_proc(
    card: &mut Card,
    boss: &mut Boss,
    target_part: BossPartName,
    damage: f64,
    round_index: u32,
    burst_trigger_count: u32,
){
    match card.card_id {
        _ => {}
    }
}

pub fn get_support_modifiers(card: &mut Card,boss: &Boss) -> SupportModifiers{
    match card.card_id{
        CardName::CrushingInstinct => crushing_instinct::get_modifiers(card,boss),
        _ => SupportModifiers::default()
    }
}
