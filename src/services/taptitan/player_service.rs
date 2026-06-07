use std::collections::HashMap;

// maps game internal card names → our card IDs
fn card_name_map() -> HashMap<&'static str, &'static str> {
    HashMap::from([
        //Burst
        ("MoonBeam",        "moon_beam"),
        ("Fragmentize",     "fragmentize"),
        ("SkullBash",       "skull_bash"),
        ("RazorWind",       "razor_wind"),
        ("WhipOfLightning", "whip_of_lightning"),
        ("BurstCount",      "clanship_barrage"),
        ("Purify",          "purifying_blast"),
        ("LimbBurst",       "psychic_shackles"),
        ("FlakShot",        "flak_shot"),
        ("Haymaker",        "cosmic_haymaker"),
        ("ChainLightning",  "chain_of_vengeance"),
        ("MirrorForce",     "mirror_force"),
        ("CelestialStatic", "celestial_static"),
        ("Weaken",          "guard_break"),
        //Affliction
        ("BurningAttack",   "blazing_inferno"),
        ("PoisonAttack",    "acid_drench"),
        ("DecayingAttack",  "decaying_strike"),
        ("Fuse",            "fusion_bomb"),
        ("Shadow",          "grim_shadow"),
        ("PlagueAttack",    "thriving_plague"),
        ("Disease",         "radioactivity"),
        ("Swarm",           "ravenous_swarm"),
        ("RuinousRust",     "ruinous_rain"),
        ("PowerBubble",     "corrosive_bubbles"),
        ("RuneAttack",      "maelstrom"),
        ("MagicPotion",     "amplify"),
        ("SandsOfTime",     "sands_of_time"),
        ("CosmicBarb",      "electro_zap"),
        //Support
        ("ExecutionersAxe", "crushing_instinct"),
        ("CrushingVoid",    "insanity_void"),
        ("MentalFocus",     "rancid_gas"),
        ("ImpactAttack",    "inspiring_force"),
        ("InnerTruth",      "soul_fire"),
        ("FinisherAttack",  "victory_march"),
        ("SuperheatMetal",  "prismatic_rift"),
        ("BurstBoost",      "ancestral_favor"),
        ("LimbSupport",     "grasping_vines"),
        ("TotemFairySkill", "totem_of_power"),
        ("TeamTactics",     "team_tactics"),
        ("SpinalTap",       "skeletal_smash"),
        ("AstralEcho",      "astral_echo"),
        ("TriangleSupport", "radiant_kaleidoscope"),
        
    ])
}

fn parse_scientific(s: &str) -> f64 {
    s.parse::<f64>().unwrap_or(0.0)
}