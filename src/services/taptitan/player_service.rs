use std::{collections::HashMap, str::FromStr};
use crate::models::{player_data::PlayerData, player_raid_data::{
        PlayerRaidData, RaidSet, TitanSoulResearch, RaidCardResearch, GemstoneResearch
    },
    cards::{Card,CardName}};

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

fn parse_scientific(s: &str) -> f64 {
    s.parse::<f64>().unwrap_or(0.0)
}

pub fn clean_data(player_data: &PlayerData) -> PlayerRaidData {
    // let map = card_name_map();

    // ── Build card_list from raw raidCards ─────────────────────────
    // ── Build card_list from raw raidCards ─────────────────────────
    let mut card_list: Vec<Card> = Vec::new();

    for (raw_name, raw_card) in &player_data.raid_cards {
        match CardName::from_str(raw_name.as_str()) {
            Ok(parsed_enum_id) => {
                card_list.push(Card {
                    card_id: parsed_enum_id, // <-- Updated field assignment matching your rename
                    cardtype: parsed_enum_id.card_type(),
                    level: raw_card.lv as u16, // cast u32 raw level safely if u16 target
                });
            }
            Err(_) => {
                // Log unknown card — should never happen (expected exactly 42)
                println!("[WARN] Unknown card in raw data: '{}'", raw_name);
            }
        }
    }

    // Warn if card count is off
    if card_list.len() != 42 {
        println!(
            "[WARN] Expected 42 cards, got {}. Check card_name_map.",
            card_list.len()
        );
    }

    // ── Build raid stats ───────────────────────────────────────────
    let raid_stats = &player_data.raid_stats;

    // ── Build raid card research ───────────────────────────────────
    let rc = &player_data.raid_card_research;
    let raid_card_research = RaidCardResearch {
        base_damage:               parse_scientific(rc.get("RaidBaseDamage").map(|s| s.as_str()).unwrap_or("0")) as u16,
        head_damage:               parse_scientific(rc.get("HeadBaseDamage").map(|s| s.as_str()).unwrap_or("0")) as u16,
        torso_damage:              parse_scientific(rc.get("TorsoBaseDamage").map(|s| s.as_str()).unwrap_or("0")) as u16,
        limbs_damage:              parse_scientific(rc.get("LimbBaseDamage").map(|s| s.as_str()).unwrap_or("0")) as u16,
        armor_damage:              parse_scientific(rc.get("ArmorBaseDamage").map(|s| s.as_str()).unwrap_or("0")) as u16,
        head_armor_damage:         parse_scientific(rc.get("HeadArmorBaseDamage").map(|s| s.as_str()).unwrap_or("0")) as u16,
        torso_armor_damage:        parse_scientific(rc.get("TorsoArmorBaseDamage").map(|s| s.as_str()).unwrap_or("0")) as u16,
        limbs_armor_damage:        parse_scientific(rc.get("LimbArmorBaseDamage").map(|s| s.as_str()).unwrap_or("0")) as u16,
        body_damage:               parse_scientific(rc.get("BodyBaseDamage").map(|s| s.as_str()).unwrap_or("0")) as u16,
        head_body_damage:          parse_scientific(rc.get("HeadBodyBaseDamage").map(|s| s.as_str()).unwrap_or("0")) as u16,
        torso_body_damage:         parse_scientific(rc.get("TorsoBodyBaseDamage").map(|s| s.as_str()).unwrap_or("0")) as u16,
        limbs_body_damage:         parse_scientific(rc.get("LimbBodyBaseDamage").map(|s| s.as_str()).unwrap_or("0")) as u16,
        lojak_damage:              parse_scientific(rc.get("Enemy1BaseDamage").map(|s| s.as_str()).unwrap_or("0")) as u16,
        takedar_damage:            parse_scientific(rc.get("Enemy2BaseDamage").map(|s| s.as_str()).unwrap_or("0")) as u16,
        jukk_damage:               parse_scientific(rc.get("Enemy3BaseDamage").map(|s| s.as_str()).unwrap_or("0")) as u16,
        sterl_damage:              parse_scientific(rc.get("Enemy4BaseDamage").map(|s| s.as_str()).unwrap_or("0")) as u16,
        mohaca_damage:             parse_scientific(rc.get("Enemy5BaseDamage").map(|s| s.as_str()).unwrap_or("0")) as u16,
        terro_damage:              parse_scientific(rc.get("Enemy6BaseDamage").map(|s| s.as_str()).unwrap_or("0")) as u16,
        klonk_damage:              parse_scientific(rc.get("Enemy7BaseDamage").map(|s| s.as_str()).unwrap_or("0")) as u16,
        priker_damage:             parse_scientific(rc.get("Enemy8BaseDamage").map(|s| s.as_str()).unwrap_or("0")) as u16,
        base_burst_damage:         parse_scientific(rc.get("BurstBaseDamage").map(|s| s.as_str()).unwrap_or("0")) as u16,
        burst_lojak_damage:        parse_scientific(rc.get("Enemy1BurstBaseDamage").map(|s| s.as_str()).unwrap_or("0")) as u16,
        burst_takedar_damage:      parse_scientific(rc.get("Enemy2BurstBaseDamage").map(|s| s.as_str()).unwrap_or("0")) as u16,
        burst_jukk_damage:         parse_scientific(rc.get("Enemy3BurstBaseDamage").map(|s| s.as_str()).unwrap_or("0")) as u16,
        burst_sterl_damage:        parse_scientific(rc.get("Enemy4BurstBaseDamage").map(|s| s.as_str()).unwrap_or("0")) as u16,
        burst_mohaca_damage:       parse_scientific(rc.get("Enemy5BurstBaseDamage").map(|s| s.as_str()).unwrap_or("0")) as u16,
        burst_terro_damage:        parse_scientific(rc.get("Enemy6BurstBaseDamage").map(|s| s.as_str()).unwrap_or("0")) as u16,
        burst_klonk_damage:        parse_scientific(rc.get("Enemy7BurstBaseDamage").map(|s| s.as_str()).unwrap_or("0")) as u16,
        burst_priker_damage:       parse_scientific(rc.get("Enemy8BurstBaseDamage").map(|s| s.as_str()).unwrap_or("0")) as u16,
        base_affliction_damage:    parse_scientific(rc.get("AfflictionBaseDamage").map(|s| s.as_str()).unwrap_or("0")) as u16,
        affliction_lojak_damage:   parse_scientific(rc.get("Enemy1AfflictionBaseDamage").map(|s| s.as_str()).unwrap_or("0")) as u16,
        affliction_takedar_damage: parse_scientific(rc.get("Enemy2AfflictionBaseDamage").map(|s| s.as_str()).unwrap_or("0")) as u16,
        affliction_jukk_damage:    parse_scientific(rc.get("Enemy3AfflictionBaseDamage").map(|s| s.as_str()).unwrap_or("0")) as u16,
        affliction_sterl_damage:   parse_scientific(rc.get("Enemy4AfflictionBaseDamage").map(|s| s.as_str()).unwrap_or("0")) as u16,
        affliction_mohaca_damage:  parse_scientific(rc.get("Enemy5AfflictionBaseDamage").map(|s| s.as_str()).unwrap_or("0")) as u16,
        affliction_terro_damage:   parse_scientific(rc.get("Enemy6AfflictionBaseDamage").map(|s| s.as_str()).unwrap_or("0")) as u16,
        affliction_klonk_damage:   parse_scientific(rc.get("Enemy7AfflictionBaseDamage").map(|s| s.as_str()).unwrap_or("0")) as u16,
        affliction_priker_damage:  parse_scientific(rc.get("Enemy8AfflictionBaseDamage").map(|s| s.as_str()).unwrap_or("0")) as u16,
    };

    // ── Build titan soul research ──────────────────────────────────
    let tr = &player_data.titan_research;
    let titan_soul_research = TitanSoulResearch {
        head_mult:    parse_scientific(tr.get("HeadDamage").map(|s| s.as_str()).unwrap_or("0")) as f32,
        torso_mult:   parse_scientific(tr.get("TorsoDamage").map(|s| s.as_str()).unwrap_or("0")) as f32,
        limbs_mult:   parse_scientific(tr.get("LimbDamage").map(|s| s.as_str()).unwrap_or("0")) as f32,
        armor_mult:   parse_scientific(tr.get("ArmorDamage").map(|s| s.as_str()).unwrap_or("0")) as f32,
        body_mult:    parse_scientific(tr.get("BodyDamage").map(|s| s.as_str()).unwrap_or("0")) as f32,
        lojak_mult:   parse_scientific(tr.get("RaidEnemy1Damage").map(|s| s.as_str()).unwrap_or("0")) as f32,
        takedar_mult: parse_scientific(tr.get("RaidEnemy2Damage").map(|s| s.as_str()).unwrap_or("0")) as f32,
        jukk_mult:    parse_scientific(tr.get("RaidEnemy3Damage").map(|s| s.as_str()).unwrap_or("0")) as f32,
        sterl_mult:   parse_scientific(tr.get("RaidEnemy4Damage").map(|s| s.as_str()).unwrap_or("0")) as f32,
        mohaca_mult:  parse_scientific(tr.get("RaidEnemy5Damage").map(|s| s.as_str()).unwrap_or("0")) as f32,
        terro_mult:   parse_scientific(tr.get("RaidEnemy6Damage").map(|s| s.as_str()).unwrap_or("0")) as f32,
        klonk_mult:   parse_scientific(tr.get("RaidEnemy7Damage").map(|s| s.as_str()).unwrap_or("0")) as f32,
        priker_mult:  parse_scientific(tr.get("RaidEnemy8Damage").map(|s| s.as_str()).unwrap_or("0")) as f32,
    };

    let rs = &player_data.equip_set;
    let raid_set = RaidSet {
        jade_anniversary: rs.contains(&"Jade".to_string()),      // +4% All Raid Damage
        jukk_juggernaut: rs.contains(&"Jukk".to_string()),       // +100 Raid Base Damage
        airforce_ace: rs.contains(&"Airforce".to_string()),          // +100 Raid Burst Base Damage
        dancer_venom: rs.contains(&"Dancer".to_string()),          // +100 Raid Affliction Base Damage
        rose_anniversary: rs.contains(&"RoseAnniversary".to_string()),      // +100 Raid Base Damage
    };
    let gmr = &player_data.gem_research;
    let gem_stone_research = GemstoneResearch {
        base_damage:               parse_scientific(gmr.get("RaidBaseDamage").map(|s| s.as_str()).unwrap_or("0")) as u16,
        head_damage:               parse_scientific(gmr.get("HeadBaseDamage").map(|s| s.as_str()).unwrap_or("0")) as u16,
        torso_damage:              parse_scientific(gmr.get("TorsoBaseDamage").map(|s| s.as_str()).unwrap_or("0")) as u16,
        limbs_damage:              parse_scientific(gmr.get("LimbBaseDamage").map(|s| s.as_str()).unwrap_or("0")) as u16,
        armor_damage:              parse_scientific(gmr.get("ArmorBaseDamage").map(|s| s.as_str()).unwrap_or("0")) as u16,
        head_armor_damage:         parse_scientific(gmr.get("HeadArmorBaseDamage").map(|s| s.as_str()).unwrap_or("0")) as u16,
        torso_armor_damage:        parse_scientific(gmr.get("TorsoArmorBaseDamage").map(|s| s.as_str()).unwrap_or("0")) as u16,
        limbs_armor_damage:        parse_scientific(gmr.get("LimbArmorBaseDamage").map(|s| s.as_str()).unwrap_or("0")) as u16,
        body_damage:               parse_scientific(gmr.get("BodyBaseDamage").map(|s| s.as_str()).unwrap_or("0")) as u16,
        head_body_damage:          parse_scientific(gmr.get("HeadBodyBaseDamage").map(|s| s.as_str()).unwrap_or("0")) as u16,
        torso_body_damage:         parse_scientific(gmr.get("TorsoBodyBaseDamage").map(|s| s.as_str()).unwrap_or("0")) as u16,
        limbs_body_damage:         parse_scientific(gmr.get("LimbBodyBaseDamage").map(|s| s.as_str()).unwrap_or("0")) as u16,
        lojak_damage:              parse_scientific(gmr.get("Enemy1BaseDamage").map(|s| s.as_str()).unwrap_or("0")) as u16,
        takedar_damage:            parse_scientific(gmr.get("Enemy2BaseDamage").map(|s| s.as_str()).unwrap_or("0")) as u16,
        jukk_damage:               parse_scientific(gmr.get("Enemy3BaseDamage").map(|s| s.as_str()).unwrap_or("0")) as u16,
        sterl_damage:              parse_scientific(gmr.get("Enemy4BaseDamage").map(|s| s.as_str()).unwrap_or("0")) as u16,
        mohaca_damage:             parse_scientific(gmr.get("Enemy5BaseDamage").map(|s| s.as_str()).unwrap_or("0")) as u16,
        terro_damage:              parse_scientific(gmr.get("Enemy6BaseDamage").map(|s| s.as_str()).unwrap_or("0")) as u16,
        klonk_damage:              parse_scientific(gmr.get("Enemy7BaseDamage").map(|s| s.as_str()).unwrap_or("0")) as u16,
        priker_damage:             parse_scientific(gmr.get("Enemy8BaseDamage").map(|s| s.as_str()).unwrap_or("0")) as u16,
        base_burst_damage:         parse_scientific(gmr.get("BurstBaseDamage").map(|s| s.as_str()).unwrap_or("0")) as u16,
        burst_lojak_damage:        parse_scientific(gmr.get("Enemy1BurstBaseDamage").map(|s| s.as_str()).unwrap_or("0")) as u16,
        burst_takedar_damage:      parse_scientific(gmr.get("Enemy2BurstBaseDamage").map(|s| s.as_str()).unwrap_or("0")) as u16,
        burst_jukk_damage:         parse_scientific(gmr.get("Enemy3BurstBaseDamage").map(|s| s.as_str()).unwrap_or("0")) as u16,
        burst_sterl_damage:        parse_scientific(gmr.get("Enemy4BurstBaseDamage").map(|s| s.as_str()).unwrap_or("0")) as u16,
        burst_mohaca_damage:       parse_scientific(gmr.get("Enemy5BurstBaseDamage").map(|s| s.as_str()).unwrap_or("0")) as u16,
        burst_terro_damage:        parse_scientific(gmr.get("Enemy6BurstBaseDamage").map(|s| s.as_str()).unwrap_or("0")) as u16,
        burst_klonk_damage:        parse_scientific(gmr.get("Enemy7BurstBaseDamage").map(|s| s.as_str()).unwrap_or("0")) as u16,
        burst_priker_damage:       parse_scientific(gmr.get("Enemy8BurstBaseDamage").map(|s| s.as_str()).unwrap_or("0")) as u16,
        base_affliction_damage:    parse_scientific(gmr.get("AfflictionBaseDamage").map(|s| s.as_str()).unwrap_or("0")) as u16,
        affliction_lojak_damage:   parse_scientific(gmr.get("Enemy1AfflictionBaseDamage").map(|s| s.as_str()).unwrap_or("0")) as u16,
        affliction_takedar_damage: parse_scientific(gmr.get("Enemy2AfflictionBaseDamage").map(|s| s.as_str()).unwrap_or("0")) as u16,
        affliction_jukk_damage:    parse_scientific(gmr.get("Enemy3AfflictionBaseDamage").map(|s| s.as_str()).unwrap_or("0")) as u16,
        affliction_sterl_damage:   parse_scientific(gmr.get("Enemy4AfflictionBaseDamage").map(|s| s.as_str()).unwrap_or("0")) as u16,
        affliction_mohaca_damage:  parse_scientific(gmr.get("Enemy5AfflictionBaseDamage").map(|s| s.as_str()).unwrap_or("0")) as u16,
        affliction_terro_damage:   parse_scientific(gmr.get("Enemy6AfflictionBaseDamage").map(|s| s.as_str()).unwrap_or("0")) as u16,
        affliction_klonk_damage:   parse_scientific(gmr.get("Enemy7AfflictionBaseDamage").map(|s| s.as_str()).unwrap_or("0")) as u16,
        affliction_priker_damage:  parse_scientific(gmr.get("Enemy8AfflictionBaseDamage").map(|s| s.as_str()).unwrap_or("0")) as u16,
    };

    // ── Assemble final struct ──────────────────────────────────────
    PlayerRaidData {
        player_raid_level:   parse_scientific(&raid_stats.raid_level) as u16,
        player_raid_base_damage: parse_scientific(&raid_stats.raid_level_base_damage) as u16,
        raid_set,
        titan_soul_research,
        raid_card_research,
        gem_stone_research,
        card_list,
        title: 0,  // fill in later
    }
}
