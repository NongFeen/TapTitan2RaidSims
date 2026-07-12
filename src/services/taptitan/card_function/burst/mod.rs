use crate::models::{
    boss::{Boss, BossPartName},
    cards::{Card, CardName},
};

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

fn default_damage(_card: &Card, _boss: &Boss, _target_part: BossPartName, damage: f64) -> f64 {
    damage
}

pub fn get_proc_chance(card: &Card, boss: &Boss) -> f64 {
    match card.card_id {
        CardName::CelestialStatic => celestial_static::get_proc_chance(card, boss),
        CardName::ChainOfVengeance => chain_of_vengeance::get_proc_chance(card, boss),
        CardName::ClanshipBarrage => clanship_barrage::get_proc_chance(card, boss),
        CardName::CosmicHaymaker => cosmic_haymaker::get_proc_chance(card, boss),
        CardName::FlakShot => flak_shot::get_proc_chance(card, boss),
        CardName::Fragmentize => fragmentize::get_proc_chance(card, boss),
        CardName::GuardBreak => guard_break::get_proc_chance(card, boss),
        CardName::MirrorForce => mirror_force::get_proc_chance(card, boss),
        CardName::MoonBeam => moon_beam::get_proc_chance(card, boss),
        CardName::PsychicShackles => psychic_shackles::get_proc_chance(card, boss),
        CardName::PurifyingBlast => purifying_blast::get_proc_chance(card, boss),
        CardName::RazorWind => razor_wind::get_proc_chance(card, boss),
        CardName::SkullBash => skull_bash::get_proc_chance(card, boss),
        CardName::WhipOfLightning => whip_of_lightning::get_proc_chance(card, boss),
        _ => 0.00,
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
        CardName::ClanshipBarrage => {
            clanship_barrage::on_proc(card, boss, target_part, damage, burst_trigger_count)
        }
        CardName::MoonBeam => moon_beam::on_proc(card, boss, target_part, damage),
        CardName::PurifyingBlast => purifying_blast::on_proc(card, boss, target_part, damage),
        CardName::RazorWind => razor_wind::on_proc(card, boss, target_part, damage),
        CardName::SkullBash => skull_bash::on_proc(card, boss, target_part, damage),
        CardName::Fragmentize => fragmentize::on_proc(card, boss, target_part, damage),
        CardName::WhipOfLightning => whip_of_lightning::on_proc(card, boss, target_part, damage),
        CardName::PsychicShackles => psychic_shackles::on_proc(card, boss, target_part, damage),
        CardName::ChainOfVengeance => chain_of_vengeance::on_proc(card, boss, target_part, damage),
        CardName::CosmicHaymaker => cosmic_haymaker::on_proc(card, boss, target_part, damage),
        CardName::FlakShot => flak_shot::on_proc(card, boss, target_part, damage),
        CardName::MirrorForce => mirror_force::on_proc(card, boss, target_part, damage,mirror_force_boost),
        CardName::CelestialStatic => celestial_static::on_proc(card, boss, target_part, damage),
        CardName::GuardBreak => guard_break::on_proc(card, boss, target_part, damage),
        _ => {}
    }
}
