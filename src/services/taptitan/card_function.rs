use crate::models::{
    boss::{Boss, BossPartName}, cards::{Card, CardType}, support_modifier::SupportModifiers,
};

mod burst;
mod support;
mod affliction;

pub fn get_proc_chance(card: &Card, boss: &Boss) -> f64 {
    debug_assert_eq!(card.cardtype, CardType::Burst);
    burst::get_proc_chance(card, boss)
}

pub fn on_proc(
    card: &mut Card,
    boss: &mut Boss,
    target_part: BossPartName,
    damage: f64,
    round_index: u32,
    burst_trigger_count: u32,
) {
    debug_assert_eq!(card.cardtype, CardType::Burst);
    burst::on_proc(
        card,
        boss,
        target_part,
        damage,
        round_index,
        burst_trigger_count,
    )
}

pub fn get_support_modifiers(card: &mut Card,boss: &Boss,deck: Vec<Card>) -> SupportModifiers{
    debug_assert_eq!(card.cardtype, CardType::Support);
    support::get_support_modifiers(card,boss,deck)
}
