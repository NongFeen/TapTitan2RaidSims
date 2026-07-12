use crate::models::{
    boss::{Boss, BossPartName},
    cards::{Card, CardName},
    support_modifier::SupportModifiers,
};
mod ancestral_favor;
mod astral_echo;
mod crushing_instinct;
mod grasping_vines;
mod insanity_void;
mod inspiring_force;
mod prismatic_rift;
mod radiant_kaleidoscope;
mod rancid_gas;
mod skeletal_smash;
mod soul_fire;
mod team_tactics;
pub mod totem_of_power;
mod victory_march;

pub fn get_proc_chance(card: &Card, boss: &Boss) -> f64 {
    match card.card_id {
        _ => 0.0,
    }
}

pub fn on_proc(
    card: &mut Card,
    boss: &mut Boss,
    target_part: BossPartName,
    damage: f64,
    mirror_force_boost: u32,
    burst_trigger_count: u32,
) {
    match card.card_id {
        _ => {}
    }
}

pub fn get_support_modifiers(card: &mut Card, boss: &Boss, deck: Vec<Card>) -> SupportModifiers {
    match card.card_id {
        CardName::AncestralFavor => ancestral_favor::get_modifiers(card, boss),
        CardName::AstralEcho => astral_echo::get_modifiers(card, boss),
        CardName::CrushingInstinct => crushing_instinct::get_modifiers(card, boss),
        CardName::GraspingVines => grasping_vines::get_modifiers(card, boss),
        CardName::InsanityVoid => insanity_void::get_modifiers(card, boss),
        CardName::InspiringForce => inspiring_force::get_modifiers(card, boss),
        CardName::PrismaticRift => prismatic_rift::get_modifiers(card, boss),
        CardName::RadiantKaleidoscope => radiant_kaleidoscope::get_modifiers(card, boss, deck),
        CardName::RancidGas => rancid_gas::get_modifiers(card, boss),
        CardName::SkeletalSmash => skeletal_smash::get_modifiers(card, boss),
        CardName::SoulFire => soul_fire::get_modifiers(card, boss),
        CardName::TeamTactics => team_tactics::get_modifiers(card, boss),
        // CardName::TotemOfPower => totem_of_power::get_modifiers(card, boss),
        CardName::VictoryMarch => victory_march::get_modifiers(card, boss),

        // CardName::InsanityVoid =>
        _ => SupportModifiers::default(),
    }
}
