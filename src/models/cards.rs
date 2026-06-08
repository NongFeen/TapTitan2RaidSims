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

impl CardName {
    // 1. Resolves the internal API ID string (e.g. "moon_beam")
    pub fn id(&self) -> &'static str {
        match self {
            CardName::MoonBeam => "moon_beam",
            CardName::Fragmentize => "fragmentize",
            CardName::SkullBash => "skull_bash",
            CardName::RazorWind => "razor_wind",
            CardName::WhipOfLightning => "whip_of_lightning",
            CardName::ClanshipBarrage => "clanship_barrage",
            CardName::PurifyingBlast => "purifying_blast",
            CardName::PsychicShackles => "psychic_shackles",
            CardName::FlakShot => "flak_shot",
            CardName::CosmicHaymaker => "cosmic_haymaker",
            CardName::ChainOfVengeance => "chain_of_vengeance",
            CardName::MirrorForce => "mirror_force",
            CardName::CelestialStatic => "celestial_static",
            CardName::GuardBreak => "guard_break",
            CardName::BlazingInferno => "blazing_inferno",
            CardName::AcidDrench => "acid_drench",
            CardName::DecayingStrike => "decaying_strike",
            CardName::FusionBomb => "fusion_bomb",
            CardName::GrimShadow => "grim_shadow",
            CardName::ThrivingPlague => "thriving_plague",
            CardName::Radioactivity => "radioactivity",
            CardName::RavenousSwarm => "ravenous_swarm",
            CardName::RuinousRain => "ruinous_rain",
            CardName::CorrosiveBubbles => "corrosive_bubbles",
            CardName::Maelstrom => "maelstrom",
            CardName::Amplify => "amplify",
            CardName::SandsOfTime => "sands_of_time",
            CardName::ElectroZap => "electro_zap",
            CardName::CrushingInstinct => "crushing_instinct",
            CardName::InsanityVoid => "insanity_void",
            CardName::RancidGas => "rancid_gas",
            CardName::InspiringForce => "inspiring_force",
            CardName::SoulFire => "soul_fire",
            CardName::VictoryMarch => "victory_march",
            CardName::PrismaticRift => "prismatic_rift",
            CardName::AncestralFavor => "ancestral_favor",
            CardName::GraspingVines => "grasping_vines",
            CardName::TotemOfPower => "totem_of_power",
            CardName::TeamTactics => "team_tactics",
            CardName::SkeletalSmash => "skeletal_smash",
            CardName::AstralEcho => "astral_echo",
            CardName::RadiantKaleidoscope => "radiant_kaleidoscope",
        }
    }

    // 2. Returns UI-friendly strings (e.g. "Moon Beam")
    pub fn display_name(&self) -> &'static str {
        match self {
            CardName::MoonBeam => "Moon Beam",
            CardName::Fragmentize => "Fragmentize",
            CardName::SkullBash => "Skull Bash",
            CardName::RazorWind => "Razor Wind",
            CardName::WhipOfLightning => "Whip of Lightning",
            CardName::ClanshipBarrage => "Clanship Barrage",
            CardName::PurifyingBlast => "Purifying Blast",
            CardName::PsychicShackles => "Psychic Shackles",
            CardName::FlakShot => "Flak Shot",
            CardName::CosmicHaymaker => "Cosmic Haymaker",
            CardName::ChainOfVengeance => "Chain of Vengeance",
            CardName::MirrorForce => "Mirror Force",
            CardName::CelestialStatic => "Celestial Static",
            CardName::GuardBreak => "Guard Break",
            CardName::BlazingInferno => "Blazing Inferno",
            CardName::AcidDrench => "Acid Drench",
            CardName::DecayingStrike => "Decaying Strike",
            CardName::FusionBomb => "Fusion Bomb",
            CardName::GrimShadow => "Grim Shadow",
            CardName::ThrivingPlague => "Thriving Plague",
            CardName::Radioactivity => "Radioactivity",
            CardName::RavenousSwarm => "Ravenous Swarm",
            CardName::RuinousRain => "Ruinous Rain",
            CardName::CorrosiveBubbles => "Corrosive Bubbles",
            CardName::Maelstrom => "Maelstrom",
            CardName::Amplify => "Amplify",
            CardName::SandsOfTime => "Sands of Time",
            CardName::ElectroZap => "Electro Zap",
            CardName::CrushingInstinct => "Crushing Instinct",
            CardName::InsanityVoid => "Insanity Void",
            CardName::RancidGas => "Rancid Gas",
            CardName::InspiringForce => "Inspiring Force",
            CardName::SoulFire => "Soul Fire",
            CardName::VictoryMarch => "Victory March",
            CardName::PrismaticRift => "Prismatic Rift",
            CardName::AncestralFavor => "Ancestral Favor",
            CardName::GraspingVines => "Grasping Vines",
            CardName::TotemOfPower => "Totem of Power",
            CardName::TeamTactics => "Team Tactics",
            CardName::SkeletalSmash => "Skeletal Smash",
            CardName::AstralEcho => "Astral Echo",
            CardName::RadiantKaleidoscope => "Radiant Kaleidoscope",
        }
    }

    // 3. Dynamically paths your WebP assets folder using the item key
    pub fn image_url(&self) -> String {
        format!("/assets/taptitan/cards/{}.webp", self.id())
    }

    // 4. Returns an iterable array containing all 42 card variants
    pub fn all_variants() -> &'static [CardName] {
        &[
            CardName::MoonBeam, CardName::Fragmentize, CardName::SkullBash, CardName::RazorWind,
            CardName::WhipOfLightning, CardName::ClanshipBarrage, CardName::PurifyingBlast,
            CardName::PsychicShackles, CardName::FlakShot, CardName::CosmicHaymaker,
            CardName::ChainOfVengeance, CardName::MirrorForce, CardName::CelestialStatic, CardName::GuardBreak,
            CardName::BlazingInferno, CardName::AcidDrench, CardName::DecayingStrike, CardName::FusionBomb,
            CardName::GrimShadow, CardName::ThrivingPlague, CardName::Radioactivity, CardName::RavenousSwarm,
            CardName::RuinousRain, CardName::CorrosiveBubbles, CardName::Maelstrom, CardName::Amplify,
            CardName::SandsOfTime, CardName::ElectroZap,
            CardName::CrushingInstinct, CardName::InsanityVoid, CardName::RancidGas, CardName::InspiringForce,
            CardName::SoulFire, CardName::VictoryMarch, CardName::PrismaticRift, CardName::AncestralFavor,
            CardName::GraspingVines, CardName::TotemOfPower, CardName::TeamTactics, CardName::SkeletalSmash,
            CardName::AstralEcho, CardName::RadiantKaleidoscope
        ]
    }
}