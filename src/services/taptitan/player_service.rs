use crate::models::{
    cards::{Card, CardName},
    player_data::PlayerData,
    player_raid_data::{
        GemstoneResearch, PlayerRaidData, RaidCardResearch, RaidSet, TitanSoulResearch,
    },
};
use std::str::FromStr;

// maps game internal card names → our card IDs
// fn card_name_map() -> HashMap<&'static str, &'static str> {
//     HashMap::from([
//         //Burst
//         ("MoonBeam",        "moon_beam"),
//         ("Fragmentize",     "fragmentize"),
//         ("SkullBash",       "skull_bash"),
//         ("RazorWind",       "razor_wind"),
//         ("WhipOfLightning", "whip_of_lightning"),
//         ("BurstCount",      "clanship_barrage"),
//         ("Purify",          "purifying_blast"),
//         ("LimbBurst",       "psychic_shackles"),
//         ("FlakShot",        "flak_shot"),
//         ("Haymaker",        "cosmic_haymaker"),
//         ("ChainLightning",  "chain_of_vengeance"),
//         ("MirrorForce",     "mirror_force"),
//         ("CelestialStatic", "celestial_static"),
//         ("Weaken",          "guard_break"),
//         //Affliction
//         ("BurningAttack",   "blazing_inferno"),
//         ("PoisonAttack",    "acid_drench"),
//         ("DecayingAttack",  "decaying_strike"),
//         ("Fuse",            "fusion_bomb"),
//         ("Shadow",          "grim_shadow"),
//         ("PlagueAttack",    "thriving_plague"),
//         ("Disease",         "radioactivity"),
//         ("Swarm",           "ravenous_swarm"),
//         ("RuinousRust",     "ruinous_rain"),
//         ("PowerBubble",     "corrosive_bubbles"),
//         ("RuneAttack",      "maelstrom"),
//         ("MagicPotion",     "amplify"),
//         ("SandsOfTime",     "sands_of_time"),
//         ("CosmicBarb",      "electro_zap"),
//         //Support
//         ("ExecutionersAxe", "crushing_instinct"),
//         ("CrushingVoid",    "insanity_void"),
//         ("MentalFocus",     "rancid_gas"),
//         ("ImpactAttack",    "inspiring_force"),
//         ("InnerTruth",      "soul_fire"),
//         ("FinisherAttack",  "victory_march"),
//         ("SuperheatMetal",  "prismatic_rift"),
//         ("BurstBoost",      "ancestral_favor"),
//         ("LimbSupport",     "grasping_vines"),
//         ("TotemFairySkill", "totem_of_power"),
//         ("TeamTactics",     "team_tactics"),
//         ("SpinalTap",       "skeletal_smash"),
//         ("AstralEcho",      "astral_echo"),
//         ("TriangleSupport", "radiant_kaleidoscope"),
//     ])
// }

// fn card_name_map() -> Ha shMap<&'static str, CardName> {
//     HashMap::from([
//         // Burst
//         ("MoonBeam",        CardName::MoonBeam),
//         ("Fragmentize",     CardName::Fragmentize),
//         ("SkullBash",       CardName::SkullBash),
//         ("RazorWind",       CardName::RazorWind),
//         ("WhipOfLightning", CardName::WhipOfLightning),
//         ("BurstCount",      CardName::ClanshipBarrage),
//         ("Purify",          CardName::PurifyingBlast),
//         ("LimbBurst",       CardName::PsychicShackles),
//         ("FlakShot",        CardName::FlakShot),
//         ("Haymaker",        CardName::CosmicHaymaker),
//         ("ChainLightning",  CardName::ChainOfVengeance),
//         ("MirrorForce",     CardName::MirrorForce),
//         ("CelestialStatic", CardName::CelestialStatic),
//         ("Weaken",          CardName::GuardBreak),
//         // Affliction
//         ("BurningAttack",   CardName::BlazingInferno),
//         ("PoisonAttack",    CardName::AcidDrench),
//         ("DecayingAttack",  CardName::DecayingStrike),
//         ("Fuse",            CardName::FusionBomb),
//         ("Shadow",          CardName::GrimShadow),
//         ("PlagueAttack",    CardName::ThrivingPlague),
//         ("Disease",         CardName::Radioactivity),
//         ("Swarm",           CardName::RavenousSwarm),
//         ("RuinousRust",     CardName::RuinousRain),
//         ("PowerBubble",     CardName::CorrosiveBubbles),
//         ("RuneAttack",      CardName::Maelstrom),
//         ("MagicPotion",     CardName::Amplify),
//         ("SandsOfTime",     CardName::SandsOfTime),
//         ("CosmicBarb",      CardName::ElectroZap),
//         // Support
//         ("ExecutionersAxe", CardName::CrushingInstinct),
//         ("CrushingVoid",    CardName::InsanityVoid),
//         ("MentalFocus",     CardName::RancidGas),
//         ("ImpactAttack",    CardName::InspiringForce),
//         ("InnerTruth",      CardName::SoulFire),
//         ("FinisherAttack",  CardName::VictoryMarch),
//         ("SuperheatMetal",  CardName::PrismaticRift),
//         ("BurstBoost",      CardName::AncestralFavor),
//         ("LimbSupport",     CardName::GraspingVines),
//         ("TotemFairySkill", CardName::TotemOfPower),
//         ("TeamTactics",     CardName::TeamTactics),
//         ("SpinalTap",       CardName::SkeletalSmash),
//         ("AstralEcho",      CardName::AstralEcho),
//         ("TriangleSupport", CardName::RadiantKaleidoscope),
//     ])
// }

mod conversion;

pub use conversion::clean_data;
