use serde::{Deserialize, Serialize};
use strum_macros::{EnumIter, EnumString};
use crate::models::{boss::{Boss, BossPartName}, support_modifier::SupportModifiers};

#[derive(Debug, Deserialize, Serialize, PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Copy, EnumIter, EnumString)]
pub enum CardName {
    // Burst
    #[serde(rename = "MoonBeam")] #[strum(serialize = "MoonBeam")] MoonBeam,
    #[serde(rename = "Fragmentize")] #[strum(serialize = "Fragmentize")] Fragmentize,
    #[serde(rename = "SkullBash")] #[strum(serialize = "SkullBash")] SkullBash,
    #[serde(rename = "RazorWind")] #[strum(serialize = "RazorWind")] RazorWind,
    #[serde(rename = "WhipOfLightning")] #[strum(serialize = "WhipOfLightning")] WhipOfLightning,
    #[serde(rename = "BurstCount")] #[strum(serialize = "BurstCount")] ClanshipBarrage,
    #[serde(rename = "Purify")] #[strum(serialize = "Purify")] PurifyingBlast,
    #[serde(rename = "LimbBurst")] #[strum(serialize = "LimbBurst")] PsychicShackles,
    #[serde(rename = "FlakShot")] #[strum(serialize = "FlakShot")] FlakShot,
    #[serde(rename = "Haymaker")] #[strum(serialize = "Haymaker")] CosmicHaymaker,
    #[serde(rename = "ChainLightning")] #[strum(serialize = "ChainLightning")] ChainOfVengeance,
    #[serde(rename = "MirrorForce")] #[strum(serialize = "MirrorForce")] MirrorForce,
    #[serde(rename = "CelestialStatic")] #[strum(serialize = "CelestialStatic")] CelestialStatic,
    #[serde(rename = "Weaken")] #[strum(serialize = "Weaken")] GuardBreak,
    
    // Affliction
    #[serde(rename = "BurningAttack")] #[strum(serialize = "BurningAttack")] BlazingInferno,
    #[serde(rename = "PoisonAttack")] #[strum(serialize = "PoisonAttack")] AcidDrench,
    #[serde(rename = "DecayingAttack")] #[strum(serialize = "DecayingAttack")] DecayingStrike,
    #[serde(rename = "Fuse")] #[strum(serialize = "Fuse")] FusionBomb,
    #[serde(rename = "Shadow")] #[strum(serialize = "Shadow")] GrimShadow,
    #[serde(rename = "PlagueAttack")] #[strum(serialize = "PlagueAttack")] ThrivingPlague,
    #[serde(rename = "Disease")] #[strum(serialize = "Disease")] Radioactivity,
    #[serde(rename = "Swarm")] #[strum(serialize = "Swarm")] RavenousSwarm,
    #[serde(rename = "RuinousRust")] #[strum(serialize = "RuinousRust")] RuinousRain,
    #[serde(rename = "PowerBubble")] #[strum(serialize = "PowerBubble")] CorrosiveBubbles,
    #[serde(rename = "RuneAttack")] #[strum(serialize = "RuneAttack")] Maelstrom,
    #[serde(rename = "MagicPotion")] #[strum(serialize = "MagicPotion")] Amplify,
    #[serde(rename = "SandsOfTime")] #[strum(serialize = "SandsOfTime")] SandsOfTime,
    #[serde(rename = "CosmicBarb")] #[strum(serialize = "CosmicBarb")] ElectroZap,
    
    // Support
    #[serde(rename = "ExecutionersAxe")] #[strum(serialize = "ExecutionersAxe")] CrushingInstinct,
    #[serde(rename = "CrushingVoid")] #[strum(serialize = "CrushingVoid")] InsanityVoid,
    #[serde(rename = "MentalFocus")] #[strum(serialize = "MentalFocus")] RancidGas,
    #[serde(rename = "ImpactAttack")] #[strum(serialize = "ImpactAttack")] InspiringForce,
    #[serde(rename = "InnerTruth")] #[strum(serialize = "InnerTruth")] SoulFire,
    #[serde(rename = "FinisherAttack")] #[strum(serialize = "FinisherAttack")] VictoryMarch,
    #[serde(rename = "SuperheatMetal")] #[strum(serialize = "SuperheatMetal")] PrismaticRift,
    #[serde(rename = "BurstBoost")] #[strum(serialize = "BurstBoost")] AncestralFavor,
    #[serde(rename = "LimbSupport")] #[strum(serialize = "LimbSupport")] GraspingVines,
    #[serde(rename = "TotemFairySkill")] #[strum(serialize = "TotemFairySkill")] TotemOfPower,
    #[serde(rename = "TeamTactics")] #[strum(serialize = "TeamTactics")] TeamTactics,
    #[serde(rename = "SpinalTap")] #[strum(serialize = "SpinalTap")] SkeletalSmash,
    #[serde(rename = "AstralEcho")] #[strum(serialize = "AstralEcho")] AstralEcho,
    #[serde(rename = "TriangleSupport")] #[strum(serialize = "TriangleSupport")] RadiantKaleidoscope,
}

#[derive(Debug, Deserialize, Serialize, PartialEq, Eq, Hash, Clone, Copy)]
pub enum CardType {
    Burst,
    Affliction,
    Support,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Card {
    pub card_id: CardName,
    pub cardtype: CardType,
    pub level: u16,
    #[serde(default)]
    pub tap_count : u16,
    #[serde(default)]
    pub chained_parts: Vec<BossPartName>,
    #[serde(default)]
    pub celestial_stacks: usize,
}

impl Card {
    pub fn get_proc_chance(&self, boss: &Boss) -> f64 {
        crate::services::taptitan::card_function::get_proc_chance(self, boss)
    }

    pub fn on_proc(
        &mut self,
        boss: &mut Boss,
        target_part: BossPartName,
        damage: f64,
        round_index: u32,
        burst_trigger_count: u32,
    ) {
        crate::services::taptitan::card_function::on_proc(
            self,
            boss,
            target_part,
            damage,
            round_index,
            burst_trigger_count,
        )
    }
    pub fn support_modifiers(&mut self,boss :&Boss,deck: Vec<Card>) -> SupportModifiers {
        crate::services::taptitan::card_function::get_support_modifiers(self,boss,deck)
    }
}

impl CardName {
    /// 1. Outputs EXACT raw data ID string keys (e.g. "MoonBeam", "FlakShot", "BurstCount")
    pub fn id(&self) -> &'static str {
        match self {
            CardName::MoonBeam => "MoonBeam",
            CardName::Fragmentize => "Fragmentize",
            CardName::SkullBash => "SkullBash",
            CardName::RazorWind => "RazorWind",
            CardName::WhipOfLightning => "WhipOfLightning",
            CardName::ClanshipBarrage => "BurstCount",
            CardName::PurifyingBlast => "Purify",
            CardName::PsychicShackles => "LimbBurst",
            CardName::FlakShot => "FlakShot",
            CardName::CosmicHaymaker => "Haymaker",
            CardName::ChainOfVengeance => "ChainLightning",
            CardName::MirrorForce => "MirrorForce",
            CardName::CelestialStatic => "CelestialStatic",
            CardName::GuardBreak => "Weaken",
            CardName::BlazingInferno => "BurningAttack",
            CardName::AcidDrench => "PoisonAttack",
            CardName::DecayingStrike => "DecayingAttack",
            CardName::FusionBomb => "Fuse",
            CardName::GrimShadow => "Shadow",
            CardName::ThrivingPlague => "PlagueAttack",
            CardName::Radioactivity => "Disease",
            CardName::RavenousSwarm => "Swarm",
            CardName::RuinousRain => "RuinousRust",
            CardName::CorrosiveBubbles => "PowerBubble",
            CardName::Maelstrom => "RuneAttack",
            CardName::Amplify => "MagicPotion",
            CardName::SandsOfTime => "SandsOfTime",
            CardName::ElectroZap => "CosmicBarb",
            CardName::CrushingInstinct => "ExecutionersAxe",
            CardName::InsanityVoid => "CrushingVoid",
            CardName::RancidGas => "MentalFocus",
            CardName::InspiringForce => "ImpactAttack",
            CardName::SoulFire => "InnerTruth",
            CardName::VictoryMarch => "FinisherAttack",
            CardName::PrismaticRift => "SuperheatMetal",
            CardName::AncestralFavor => "BurstBoost",
            CardName::GraspingVines => "LimbSupport",
            CardName::TotemOfPower => "TotemFairySkill",
            CardName::TeamTactics => "TeamTactics",
            CardName::SkeletalSmash => "SpinalTap",
            CardName::AstralEcho => "AstralEcho",
            CardName::RadiantKaleidoscope => "TriangleSupport",
        }
    }

    /// 2. Outputs clean custom display words (e.g. "Moon Beam", "Flak Shot", "Clanship Barrage")
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

    /// 3. Safely format image URLs using whichever casing your files use. 
    /// If your files are snake_case (e.g. flak_shot.webp), you can swap this function to utilize a lowercase helper or write them out.
    pub fn image_url(&self) -> String {
        // Example assumes images are titled matching variant names (e.g., FlakShot.webp)
        // If your image names are lowercased snake_case (flak_shot.webp), change self.id() to whatever matches your files.
        format!("/assets/taptitan/cards/{}.webp", self.id())
    }

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


