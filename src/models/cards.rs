use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize, PartialEq, Eq, Hash, Clone)]
pub enum CardName{
    MoonBeam,
    Fragmentize,
    SkullBash,
    RazorWind,
    WhipOfLightning,
    ClanshipBarrage,
    PurifyingBlast,
    PsychicShackles,
    FlakShot,
    CosmicHaymaker,
    ChainOfVengeance,
    MirrorForce,
    CelestialStatic,
    GuardBreak,
    //Affliction
    BlazingInferno,
    AcidDrench,
    DecayingStrike,
    FusionBomb,
    GrimShadow,
    ThrivingPlague,
    Radioactivity,
    RavenousSwarm,
    RuinousRain,
    CorrosiveBubbles,
    Maelstrom,
    Amplify,
    SandsOfTime,
    ElectroZap,
    //Support
    CrushingInstinct,
    InsanityVoid,
    RancidGas,
    InspiringForce,
    SoulFire,
    VictoryMarch,
    PrismaticRift,
    AncestralFavor,
    GraspingVines,
    TotemOfPower,
    TeamTactics,
    SkeletalSmash,
    AstralEcho,
    RadiantKaleidoscope,
} 
#[derive(Debug, Deserialize, Serialize)]
pub struct Card{
    pub card_name: CardName,
    pub level: u16,
}