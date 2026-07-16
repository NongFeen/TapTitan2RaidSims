use crate::models::{
    affliction::Affliction,
    boss::{Boss, BossPartName, BossTickView},
    cards::{Card, CardType},
    damage_source::DamageSource,
    support_modifier::SupportModifiers,
};

mod affliction;
mod burst;
pub mod support;

#[derive(Debug, Clone)]
pub struct AfflictionDamageEvent {
    pub part_name: BossPartName,
    pub damage: u64,
    pub source: DamageSource,
}

pub fn get_proc_chance(card: &Card, boss: &Boss) -> f64 {
    match card.cardtype {
        CardType::Burst => burst::get_proc_chance(card, boss),
        CardType::Affliction => affliction::get_proc_chance(card, boss),
        CardType::Support => 0.0,
    }
}

pub fn on_proc(
    card: &mut Card,
    boss: &mut Boss,
    target_part: BossPartName,
    damage: f64,
    mirror_force_boost: u32,
    burst_trigger_count: u32,
) -> u64 {
    match card.cardtype {
        CardType::Burst => burst::on_proc(
            card,
            boss,
            target_part,
            damage,
            mirror_force_boost,
            burst_trigger_count,
        ),
        CardType::Affliction => affliction::on_proc(card, boss, target_part, damage),
        CardType::Support => 0,
    }
}

pub fn get_support_modifiers(card: &mut Card, boss: &Boss, deck: Vec<Card>) -> SupportModifiers {
    debug_assert_eq!(card.cardtype, CardType::Support);
    support::get_support_modifiers(card, boss, deck)
}

pub fn tick_affliction(
    affliction: &mut Affliction,
    boss: &BossTickView,
    part_name: BossPartName,
    elapsed_seconds: f64,
) -> Vec<AfflictionDamageEvent> {
    affliction::on_tick(affliction, boss, part_name, elapsed_seconds)
}
