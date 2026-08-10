use super::*;
use std::collections::HashMap;

fn parse_scientific(s: &str) -> f64 {
    s.parse::<f64>().unwrap_or(0.0)
}

pub fn clean_data(player_data: &PlayerData) -> PlayerRaidData {
    // let map = card_name_map();

    // ── Build card_list from raw raidCards ─────────────────────────
    // ── Build card_list from raw raidCards ─────────────────────────
    let mut card_list: Vec<Card> = Vec::new();
    let boosted_levels = player_data.boosted_cards.iter().fold(
        HashMap::<CardName, u16>::new(),
        |mut levels, boosted| {
            if let Ok(card_name) = CardName::from_str(&boosted.skill_name) {
                levels
                    .entry(card_name)
                    .and_modify(|level| *level = (*level).max(boosted.boost_level))
                    .or_insert(boosted.boost_level);
            }
            levels
        },
    );

    for (raw_name, raw_card) in &player_data.raid_cards {
        match CardName::from_str(raw_name.as_str()) {
            Ok(parsed_enum_id) => {
                card_list.push(Card {
                    card_id: parsed_enum_id, // <-- Updated field assignment matching your rename
                    cardtype: parsed_enum_id.card_type(),
                    level: boosted_levels
                        .get(&parsed_enum_id)
                        .copied()
                        .map(|boosted| boosted.max(raw_card.lv))
                        .unwrap_or(raw_card.lv),
                    enabled: true,
                    tap_count: 0,
                    chained_parts: Default::default(),
                    celestial_stacks: Default::default(),
                    skill: Default::default(),
                    proc_chance_cache: 0.0,
                });
            }
            Err(_) => {
                // Log unknown card — should never happen (expected exactly 42)
                tracing::warn!(card = %raw_name, "Ignoring unknown card in player data");
            }
        }
    }
    card_list.sort_by_key(|card| card.card_id);
    // Warn if card count is off
    if card_list.len() != 44 {
        tracing::warn!(
            cards = card_list.len(),
            expected = 44,
            "Player data has an unexpected known-card count"
        );
    }

    // ── Build raid stats ───────────────────────────────────────────
    let raid_stats = &player_data.raid_stats;

    // ── Build raid card research ───────────────────────────────────
    let rc = &player_data.raid_card_research;
    let raid_card_research = RaidCardResearch {
        base_damage: parse_scientific(rc.get("RaidBaseDamage").map(|s| s.as_str()).unwrap_or("0"))
            as u16,
        head_damage: parse_scientific(rc.get("HeadBaseDamage").map(|s| s.as_str()).unwrap_or("0"))
            as u16,
        torso_damage: parse_scientific(rc.get("TorsoBaseDamage").map(|s| s.as_str()).unwrap_or("0"))
            as u16,
        limbs_damage: parse_scientific(rc.get("LimbBaseDamage").map(|s| s.as_str()).unwrap_or("0"))
            as u16,
        armor_damage: parse_scientific(rc.get("ArmorBaseDamage").map(|s| s.as_str()).unwrap_or("0"))
            as u16,
        head_armor_damage: parse_scientific(
            rc.get("HeadArmorBaseDamage")
                .map(|s| s.as_str())
                .unwrap_or("0"),
        ) as u16,
        torso_armor_damage: parse_scientific(
            rc.get("TorsoArmorBaseDamage")
                .map(|s| s.as_str())
                .unwrap_or("0"),
        ) as u16,
        limbs_armor_damage: parse_scientific(
            rc.get("LimbArmorBaseDamage")
                .map(|s| s.as_str())
                .unwrap_or("0"),
        ) as u16,
        body_damage: parse_scientific(rc.get("BodyBaseDamage").map(|s| s.as_str()).unwrap_or("0"))
            as u16,
        head_body_damage: parse_scientific(
            rc.get("HeadBodyBaseDamage")
                .map(|s| s.as_str())
                .unwrap_or("0"),
        ) as u16,
        torso_body_damage: parse_scientific(
            rc.get("TorsoBodyBaseDamage")
                .map(|s| s.as_str())
                .unwrap_or("0"),
        ) as u16,
        limbs_body_damage: parse_scientific(
            rc.get("LimbBodyBaseDamage")
                .map(|s| s.as_str())
                .unwrap_or("0"),
        ) as u16,
        lojak_damage: parse_scientific(
            rc.get("Enemy1BaseDamage")
                .map(|s| s.as_str())
                .unwrap_or("0"),
        ) as u16,
        takedar_damage: parse_scientific(
            rc.get("Enemy2BaseDamage")
                .map(|s| s.as_str())
                .unwrap_or("0"),
        ) as u16,
        jukk_damage: parse_scientific(
            rc.get("Enemy3BaseDamage")
                .map(|s| s.as_str())
                .unwrap_or("0"),
        ) as u16,
        sterl_damage: parse_scientific(
            rc.get("Enemy4BaseDamage")
                .map(|s| s.as_str())
                .unwrap_or("0"),
        ) as u16,
        mohaca_damage: parse_scientific(
            rc.get("Enemy5BaseDamage")
                .map(|s| s.as_str())
                .unwrap_or("0"),
        ) as u16,
        terro_damage: parse_scientific(
            rc.get("Enemy6BaseDamage")
                .map(|s| s.as_str())
                .unwrap_or("0"),
        ) as u16,
        klonk_damage: parse_scientific(
            rc.get("Enemy7BaseDamage")
                .map(|s| s.as_str())
                .unwrap_or("0"),
        ) as u16,
        priker_damage: parse_scientific(
            rc.get("Enemy8BaseDamage")
                .map(|s| s.as_str())
                .unwrap_or("0"),
        ) as u16,
        base_burst_damage: parse_scientific(
            rc.get("BurstBaseDamage").map(|s| s.as_str()).unwrap_or("0"),
        ) as u16,
        burst_lojak_damage: parse_scientific(
            rc.get("Enemy1BurstBaseDamage")
                .map(|s| s.as_str())
                .unwrap_or("0"),
        ) as u16,
        burst_takedar_damage: parse_scientific(
            rc.get("Enemy2BurstBaseDamage")
                .map(|s| s.as_str())
                .unwrap_or("0"),
        ) as u16,
        burst_jukk_damage: parse_scientific(
            rc.get("Enemy3BurstBaseDamage")
                .map(|s| s.as_str())
                .unwrap_or("0"),
        ) as u16,
        burst_sterl_damage: parse_scientific(
            rc.get("Enemy4BurstBaseDamage")
                .map(|s| s.as_str())
                .unwrap_or("0"),
        ) as u16,
        burst_mohaca_damage: parse_scientific(
            rc.get("Enemy5BurstBaseDamage")
                .map(|s| s.as_str())
                .unwrap_or("0"),
        ) as u16,
        burst_terro_damage: parse_scientific(
            rc.get("Enemy6BurstBaseDamage")
                .map(|s| s.as_str())
                .unwrap_or("0"),
        ) as u16,
        burst_klonk_damage: parse_scientific(
            rc.get("Enemy7BurstBaseDamage")
                .map(|s| s.as_str())
                .unwrap_or("0"),
        ) as u16,
        burst_priker_damage: parse_scientific(
            rc.get("Enemy8BurstBaseDamage")
                .map(|s| s.as_str())
                .unwrap_or("0"),
        ) as u16,
        base_affliction_damage: parse_scientific(
            rc.get("AfflictionBaseDamage")
                .map(|s| s.as_str())
                .unwrap_or("0"),
        ) as u16,
        affliction_lojak_damage: parse_scientific(
            rc.get("Enemy1AfflictionBaseDamage")
                .map(|s| s.as_str())
                .unwrap_or("0"),
        ) as u16,
        affliction_takedar_damage: parse_scientific(
            rc.get("Enemy2AfflictionBaseDamage")
                .map(|s| s.as_str())
                .unwrap_or("0"),
        ) as u16,
        affliction_jukk_damage: parse_scientific(
            rc.get("Enemy3AfflictionBaseDamage")
                .map(|s| s.as_str())
                .unwrap_or("0"),
        ) as u16,
        affliction_sterl_damage: parse_scientific(
            rc.get("Enemy4AfflictionBaseDamage")
                .map(|s| s.as_str())
                .unwrap_or("0"),
        ) as u16,
        affliction_mohaca_damage: parse_scientific(
            rc.get("Enemy5AfflictionBaseDamage")
                .map(|s| s.as_str())
                .unwrap_or("0"),
        ) as u16,
        affliction_terro_damage: parse_scientific(
            rc.get("Enemy6AfflictionBaseDamage")
                .map(|s| s.as_str())
                .unwrap_or("0"),
        ) as u16,
        affliction_klonk_damage: parse_scientific(
            rc.get("Enemy7AfflictionBaseDamage")
                .map(|s| s.as_str())
                .unwrap_or("0"),
        ) as u16,
        affliction_priker_damage: parse_scientific(
            rc.get("Enemy8AfflictionBaseDamage")
                .map(|s| s.as_str())
                .unwrap_or("0"),
        ) as u16,
    };

    // ── Build titan soul research ──────────────────────────────────
    let tr = &player_data.titan_research;
    let titan_soul_research = TitanSoulResearch {
        head_mult: parse_scientific(tr.get("HeadDamage").map(|s| s.as_str()).unwrap_or("0")) as f32,
        torso_mult: parse_scientific(tr.get("ChestDamage").map(|s| s.as_str()).unwrap_or("0"))
            as f32,
        limbs_mult: parse_scientific(tr.get("LimbDamage").map(|s| s.as_str()).unwrap_or("0"))
            as f32,
        armor_mult: parse_scientific(tr.get("ArmorDamage").map(|s| s.as_str()).unwrap_or("0"))
            as f32,
        body_mult: parse_scientific(tr.get("BodyDamage").map(|s| s.as_str()).unwrap_or("0")) as f32,
        lojak_mult: parse_scientific(
            tr.get("RaidEnemy1Damage")
                .map(|s| s.as_str())
                .unwrap_or("0"),
        ) as f32,
        takedar_mult: parse_scientific(
            tr.get("RaidEnemy2Damage")
                .map(|s| s.as_str())
                .unwrap_or("0"),
        ) as f32,
        jukk_mult: parse_scientific(
            tr.get("RaidEnemy3Damage")
                .map(|s| s.as_str())
                .unwrap_or("0"),
        ) as f32,
        sterl_mult: parse_scientific(
            tr.get("RaidEnemy4Damage")
                .map(|s| s.as_str())
                .unwrap_or("0"),
        ) as f32,
        mohaca_mult: parse_scientific(
            tr.get("RaidEnemy5Damage")
                .map(|s| s.as_str())
                .unwrap_or("0"),
        ) as f32,
        terro_mult: parse_scientific(
            tr.get("RaidEnemy6Damage")
                .map(|s| s.as_str())
                .unwrap_or("0"),
        ) as f32,
        klonk_mult: parse_scientific(
            tr.get("RaidEnemy7Damage")
                .map(|s| s.as_str())
                .unwrap_or("0"),
        ) as f32,
        priker_mult: parse_scientific(
            tr.get("RaidEnemy8Damage")
                .map(|s| s.as_str())
                .unwrap_or("0"),
        ) as f32,
    };

    let rs = &player_data.equip_set;
    let raid_set = RaidSet {
        jade_anniversary: rs.contains(&"Jade".to_string()), // +4% All Raid Damage
        jukk_juggernaut: rs.contains(&"Jukk".to_string()),  // +100 Raid Base Damage
        airforce_ace: rs.contains(&"Airforce".to_string()), // +120 Raid Burst Base Damage
        dancer_venom: rs.contains(&"Dancer".to_string()),   // +120 Raid Affliction Base Damage
        rose_anniversary: rs.contains(&"RoseAnniversary".to_string()), // +100 Raid Base Damage
    };
    let gmr = &player_data.gem_research;
    let gem_stone_research = GemstoneResearch {
        base_damage: parse_scientific(gmr.get("RaidBaseDamage").map(|s| s.as_str()).unwrap_or("0"))
            as u16,
        head_damage: parse_scientific(gmr.get("HeadBaseDamage").map(|s| s.as_str()).unwrap_or("0"))
            as u16,
        torso_damage: parse_scientific(
            gmr.get("TorsoBaseDamage")
                .map(|s| s.as_str())
                .unwrap_or("0"),
        ) as u16,
        limbs_damage: parse_scientific(gmr.get("LimbBaseDamage").map(|s| s.as_str()).unwrap_or("0"))
            as u16,
        armor_damage: parse_scientific(
            gmr.get("ArmorBaseDamage")
                .map(|s| s.as_str())
                .unwrap_or("0"),
        ) as u16,
        head_armor_damage: parse_scientific(
            gmr.get("HeadArmorBaseDamage")
                .map(|s| s.as_str())
                .unwrap_or("0"),
        ) as u16,
        torso_armor_damage: parse_scientific(
            gmr.get("TorsoArmorBaseDamage")
                .map(|s| s.as_str())
                .unwrap_or("0"),
        ) as u16,
        limbs_armor_damage: parse_scientific(
            gmr.get("LimbArmorBaseDamage")
                .map(|s| s.as_str())
                .unwrap_or("0"),
        ) as u16,
        body_damage: parse_scientific(gmr.get("BodyBaseDamage").map(|s| s.as_str()).unwrap_or("0"))
            as u16,
        head_body_damage: parse_scientific(
            gmr.get("HeadBodyBaseDamage")
                .map(|s| s.as_str())
                .unwrap_or("0"),
        ) as u16,
        torso_body_damage: parse_scientific(
            gmr.get("TorsoBodyBaseDamage")
                .map(|s| s.as_str())
                .unwrap_or("0"),
        ) as u16,
        limbs_body_damage: parse_scientific(
            gmr.get("LimbBodyBaseDamage")
                .map(|s| s.as_str())
                .unwrap_or("0"),
        ) as u16,
        lojak_damage: parse_scientific(
            gmr.get("Enemy1BaseDamage")
                .map(|s| s.as_str())
                .unwrap_or("0"),
        ) as u16,
        takedar_damage: parse_scientific(
            gmr.get("Enemy2BaseDamage")
                .map(|s| s.as_str())
                .unwrap_or("0"),
        ) as u16,
        jukk_damage: parse_scientific(
            gmr.get("Enemy3BaseDamage")
                .map(|s| s.as_str())
                .unwrap_or("0"),
        ) as u16,
        sterl_damage: parse_scientific(
            gmr.get("Enemy4BaseDamage")
                .map(|s| s.as_str())
                .unwrap_or("0"),
        ) as u16,
        mohaca_damage: parse_scientific(
            gmr.get("Enemy5BaseDamage")
                .map(|s| s.as_str())
                .unwrap_or("0"),
        ) as u16,
        terro_damage: parse_scientific(
            gmr.get("Enemy6BaseDamage")
                .map(|s| s.as_str())
                .unwrap_or("0"),
        ) as u16,
        klonk_damage: parse_scientific(
            gmr.get("Enemy7BaseDamage")
                .map(|s| s.as_str())
                .unwrap_or("0"),
        ) as u16,
        priker_damage: parse_scientific(
            gmr.get("Enemy8BaseDamage")
                .map(|s| s.as_str())
                .unwrap_or("0"),
        ) as u16,
        base_burst_damage: parse_scientific(
            gmr.get("BurstBaseDamage")
                .map(|s| s.as_str())
                .unwrap_or("0"),
        ) as u16,
        burst_lojak_damage: parse_scientific(
            gmr.get("Enemy1BurstBaseDamage")
                .map(|s| s.as_str())
                .unwrap_or("0"),
        ) as u16,
        burst_takedar_damage: parse_scientific(
            gmr.get("Enemy2BurstBaseDamage")
                .map(|s| s.as_str())
                .unwrap_or("0"),
        ) as u16,
        burst_jukk_damage: parse_scientific(
            gmr.get("Enemy3BurstBaseDamage")
                .map(|s| s.as_str())
                .unwrap_or("0"),
        ) as u16,
        burst_sterl_damage: parse_scientific(
            gmr.get("Enemy4BurstBaseDamage")
                .map(|s| s.as_str())
                .unwrap_or("0"),
        ) as u16,
        burst_mohaca_damage: parse_scientific(
            gmr.get("Enemy5BurstBaseDamage")
                .map(|s| s.as_str())
                .unwrap_or("0"),
        ) as u16,
        burst_terro_damage: parse_scientific(
            gmr.get("Enemy6BurstBaseDamage")
                .map(|s| s.as_str())
                .unwrap_or("0"),
        ) as u16,
        burst_klonk_damage: parse_scientific(
            gmr.get("Enemy7BurstBaseDamage")
                .map(|s| s.as_str())
                .unwrap_or("0"),
        ) as u16,
        burst_priker_damage: parse_scientific(
            gmr.get("Enemy8BurstBaseDamage")
                .map(|s| s.as_str())
                .unwrap_or("0"),
        ) as u16,
        base_affliction_damage: parse_scientific(
            gmr.get("AfflictionBaseDamage")
                .map(|s| s.as_str())
                .unwrap_or("0"),
        ) as u16,
        affliction_lojak_damage: parse_scientific(
            gmr.get("Enemy1AfflictionBaseDamage")
                .map(|s| s.as_str())
                .unwrap_or("0"),
        ) as u16,
        affliction_takedar_damage: parse_scientific(
            gmr.get("Enemy2AfflictionBaseDamage")
                .map(|s| s.as_str())
                .unwrap_or("0"),
        ) as u16,
        affliction_jukk_damage: parse_scientific(
            gmr.get("Enemy3AfflictionBaseDamage")
                .map(|s| s.as_str())
                .unwrap_or("0"),
        ) as u16,
        affliction_sterl_damage: parse_scientific(
            gmr.get("Enemy4AfflictionBaseDamage")
                .map(|s| s.as_str())
                .unwrap_or("0"),
        ) as u16,
        affliction_mohaca_damage: parse_scientific(
            gmr.get("Enemy5AfflictionBaseDamage")
                .map(|s| s.as_str())
                .unwrap_or("0"),
        ) as u16,
        affliction_terro_damage: parse_scientific(
            gmr.get("Enemy6AfflictionBaseDamage")
                .map(|s| s.as_str())
                .unwrap_or("0"),
        ) as u16,
        affliction_klonk_damage: parse_scientific(
            gmr.get("Enemy7AfflictionBaseDamage")
                .map(|s| s.as_str())
                .unwrap_or("0"),
        ) as u16,
        affliction_priker_damage: parse_scientific(
            gmr.get("Enemy8AfflictionBaseDamage")
                .map(|s| s.as_str())
                .unwrap_or("0"),
        ) as u16,
    };

    // ── Assemble final struct ──────────────────────────────────────
    PlayerRaidData {
        player_raid_level: parse_scientific(&raid_stats.raid_level) as u16,
        player_raid_base_damage: parse_scientific(&raid_stats.raid_level_base_damage) as u16,
        raid_set,
        titan_soul_research,
        raid_card_research,
        gem_stone_research,
        card_list,
        title: 0.0, // fill in later
    }
}
