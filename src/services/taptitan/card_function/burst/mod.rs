use crate::models::{
    boss::{Boss, BossPartName},
    cards::{Card, CardName},
};

use super::CardProcSnapshot;

mod celestial_static;
mod chain_of_vengeance;
mod clanship_barrage;
mod cosmic_haymaker;
mod flak_shot;
mod fragmentize;
mod guard_break;
mod mirror_force;
mod moon_beam;
mod psychic_shackles;
mod purifying_blast;
mod razor_wind;
mod skull_bash;
mod whip_of_lightning;

fn default_snapshot(card: &Card, boss: &Boss, target_part: BossPartName) -> CardProcSnapshot {
    CardProcSnapshot {
        card_id: card.card_id,
        proc_chance: roll_proc_chance(card, boss),
        damage_multiplier: 1.0,
        notes: vec![format!(
            "No special burst override for {:?} on {:?}.",
            card.card_id, target_part
        )],
    }
}

pub fn roll_proc_chance(card: &Card, boss: &Boss) -> f64 {
    match card.card_id {
        CardName::ClanshipBarrage => clanship_barrage::roll_proc_chance(card, boss),
        CardName::WhipOfLightning => whip_of_lightning::roll_proc_chance(card, boss),
        CardName::CosmicHaymaker => cosmic_haymaker::roll_proc_chance(card, boss),
        _ => 0.12,
    }
}

pub fn on_proc(card: &Card, boss: &mut Boss, target_part: BossPartName) -> CardProcSnapshot {
    match card.card_id {
        CardName::ClanshipBarrage => clanship_barrage::on_proc(card, boss, target_part),
        CardName::MoonBeam => moon_beam::on_proc(card, boss, target_part),
        CardName::PurifyingBlast => purifying_blast::on_proc(card, boss, target_part),
        CardName::RazorWind => razor_wind::on_proc(card, boss, target_part),
        CardName::SkullBash => skull_bash::on_proc(card, boss, target_part),
        CardName::Fragmentize => fragmentize::on_proc(card, boss, target_part),
        CardName::WhipOfLightning => whip_of_lightning::on_proc(card, boss, target_part),
        CardName::PsychicShackles => psychic_shackles::on_proc(card, boss, target_part),
        CardName::ChainOfVengeance => chain_of_vengeance::on_proc(card, boss, target_part),
        CardName::CosmicHaymaker => cosmic_haymaker::on_proc(card, boss, target_part),
        CardName::FlakShot => flak_shot::on_proc(card, boss, target_part),
        CardName::MirrorForce => mirror_force::on_proc(card, boss, target_part),
        CardName::CelestialStatic => celestial_static::on_proc(card, boss, target_part),
        CardName::GuardBreak => guard_break::on_proc(card, boss, target_part),
        _ => default_snapshot(card, boss, target_part),
    }
}
