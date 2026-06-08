use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize, PartialEq, Eq, Hash, Clone)]
pub enum CardName {
    #[serde(rename = "moon_beam")] MoonBeam,
    #[serde(rename = "fragmentize")] Fragmentize,
    #[serde(rename = "skull_bash")] SkullBash,
    #[serde(rename = "razor_wind")] RazorWind,
    #[serde(rename = "whip_of_lightning")] WhipOfLightning,
    #[serde(rename = "clanship_barrage")] ClanshipBarrage,
    #[serde(rename = "purifying_blast")] PurifyingBlast,
    #[serde(rename = "psychic_shackles")] PsychicShackles,
    #[serde(rename = "flak_shot")] FlakShot,
    #[serde(rename = "cosmic_haymaker")] CosmicHaymaker,
    #[serde(rename = "chain_of_vengeance")] ChainOfVengeance,
    #[serde(rename = "mirror_force")] MirrorForce,
    #[serde(rename = "celestial_static")] CelestialStatic,
    #[serde(rename = "guard_break")] GuardBreak,
    
    // Affliction
    #[serde(rename = "blazing_inferno")] BlazingInferno,
    #[serde(rename = "acid_drench")] AcidDrench,
    #[serde(rename = "decaying_strike")] DecayingStrike,
    #[serde(rename = "fusion_bomb")] FusionBomb,
    #[serde(rename = "grim_shadow")] GrimShadow,
    #[serde(rename = "thriving_plague")] ThrivingPlague,
    #[serde(rename = "radioactivity")] Radioactivity,
    #[serde(rename = "ravenous_swarm")] RavenousSwarm,
    #[serde(rename = "ruinous_rain")] RuinousRain,
    #[serde(rename = "corrosive_bubbles")] CorrosiveBubbles,
    #[serde(rename = "maelstrom")] Maelstrom,
    #[serde(rename = "amplify")] Amplify,
    #[serde(rename = "sands_of_time")] SandsOfTime,
    #[serde(rename = "electro_zap")] ElectroZap,
    
    // Support
    #[serde(rename = "crushing_instinct")] CrushingInstinct,
    #[serde(rename = "insanity_void")] InsanityVoid,
    #[serde(rename = "rancid_gas")] RancidGas,
    #[serde(rename = "inspiring_force")] InspiringForce,
    #[serde(rename = "soul_fire")] SoulFire,
    #[serde(rename = "victory_march")] VictoryMarch,
    #[serde(rename = "prismatic_rift")] PrismaticRift,
    #[serde(rename = "ancestral_favor")] AncestralFavor,
    #[serde(rename = "grasping_vines")] GraspingVines,
    #[serde(rename = "totem_of_power")] TotemOfPower,
    #[serde(rename = "team_tactics")] TeamTactics,
    #[serde(rename = "skeletal_smash")] SkeletalSmash,
    #[serde(rename = "astral_echo")] AstralEcho,
    #[serde(rename = "radiant_kaleidoscope")] RadiantKaleidoscope,
}

#[derive(Debug, Deserialize, Serialize, PartialEq, Eq, Hash, Clone, Copy)]
pub enum CardType {
    Burst,
    Affliction,
    Support,
}

// Add a helper method directly to CardName to resolve its category
impl CardName {
    pub fn card_type(&self) -> CardType {
        match self {
            CardName::MoonBeam | CardName::Fragmentize | CardName::SkullBash | 
            CardName::RazorWind | CardName::WhipOfLightning | CardName::ClanshipBarrage | 
            CardName::PurifyingBlast | CardName::PsychicShackles | CardName::FlakShot | 
            CardName::CosmicHaymaker | CardName::ChainOfVengeance | CardName::MirrorForce | 
            CardName::CelestialStatic | CardName::GuardBreak => CardType::Burst,

            CardName::BlazingInferno | CardName::AcidDrench | CardName::DecayingStrike | 
            CardName::FusionBomb | CardName::GrimShadow | CardName::ThrivingPlague | 
            CardName::Radioactivity | CardName::RavenousSwarm | CardName::RuinousRain | 
            CardName::CorrosiveBubbles | CardName::Maelstrom | CardName::Amplify | 
            CardName::SandsOfTime | CardName::ElectroZap => CardType::Affliction,

            CardName::CrushingInstinct | CardName::InsanityVoid | CardName::RancidGas | 
            CardName::InspiringForce | CardName::SoulFire | CardName::VictoryMarch | 
            CardName::PrismaticRift | CardName::AncestralFavor | CardName::GraspingVines | 
            CardName::TotemOfPower | CardName::TeamTactics | CardName::SkeletalSmash | 
            CardName::AstralEcho | CardName::RadiantKaleidoscope => CardType::Support,
        }
    }
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Card {
    pub card_name: CardName,
    pub cardtype: CardType,
    pub level: u16,
}