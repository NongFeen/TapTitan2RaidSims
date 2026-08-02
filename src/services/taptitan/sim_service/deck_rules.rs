use super::*;

pub fn generate_deck(sim_stats: &SimStats) -> Vec<Vec<Card>> {
    // 1. Only pick cards that are in the user's explicit usable list

    let filtered_cards: Vec<Card> = sim_stats
        .player_stat
        .card_list
        .iter()
        .filter(|card| sim_stats.usable_card.contains(&card.card_id))
        .map(|card| {
            let mut card = card.clone();
            card.ensure_skill_cache();
            card
        })
        .collect();

    let mut deck_combinations = Vec::new();

    // 2. Form groups of exactly 3 unique cards
    for combo in filtered_cards.iter().combinations(3) {
        let c1 = combo[0];
        let c2 = combo[1];
        let c3 = combo[2];
        // println!(
        //     "Checking deck combination: {}, {}, {}",
        //     c1.card_id.display_name(),
        //     c2.card_id.display_name(),
        //     c3.card_id.display_name()
        // );
        // 3. Keep the deck only if it is synergistic and boss-compatible!
        let deck = [c1, c2, c3];
        if deck_pair_rules::deck_passes_pair_table(&deck)
            && is_deck_synergistic(sim_stats, c1, c2, c3)
            && is_deck_boss_suitable(sim_stats, c1, c2, c3)
        {
            // Dereference the pointers to store clean Card values
            let deck = vec![c1.clone(), c2.clone(), c3.clone()];
            deck_combinations.push(deck);
        }
    }

    deck_combinations
}

pub(super) const IS_CHECK_CARD_SYNERGY: bool = false;
pub(super) const PURIFY_PRIORITY_AFFLICTIONS: [CardName; 6] = [
    CardName::AcidDrench,
    CardName::RavenousSwarm,
    CardName::RuinousRain,
    CardName::Amplify,
    CardName::ElectroZap,
    CardName::BlazingInferno,
];

pub(super) fn is_purify_priority_affliction(card_name: CardName) -> bool {
    PURIFY_PRIORITY_AFFLICTIONS.contains(&card_name)
}

pub(super) fn is_deck_synergistic(sim_stats: &SimStats, c1: &Card, c2: &Card, c3: &Card) -> bool {
    let deck = [c1, c2, c3];
    let burst_count = deck
        .iter()
        .filter(|c| c.cardtype == CardType::Burst)
        .count();
    let affliction_count = deck
        .iter()
        .filter(|c| c.cardtype == CardType::Affliction)
        .count();
    let support_count = deck
        .iter()
        .filter(|c| c.cardtype == CardType::Support)
        .count();

    //total deck without any rule = 42*41*40/3/2 = 11480
    //Policy 1 : card must be synergy by it self
    let has_support = support_count > 0;
    let has_maelstrom = deck.iter().any(|c| c.card_id == CardName::Maelstrom);
    let has_guard_break = deck.iter().any(|c| c.card_id == CardName::GuardBreak);

    let has_purify = deck.iter().any(|c| c.card_id == CardName::PurifyingBlast);
    let has_affliction = affliction_count > 0;

    let has_radiant_kaleidoscope = deck
        .iter()
        .any(|c| c.card_id == CardName::RadiantKaleidoscope);

    let _has_ancestral_favor = deck.iter().any(|c| c.card_id == CardName::AncestralFavor);

    let _has_rancid_gas = deck.iter().any(|c| c.card_id == CardName::RancidGas);

    let _has_sands_of_time = deck.iter().any(|c| c.card_id == CardName::SandsOfTime);

    let has_whip = deck.iter().any(|c| c.card_id == CardName::WhipOfLightning);

    let _has_celestial_static = deck.iter().any(|c| c.card_id == CardName::CelestialStatic);
    let _has_grasping_vines = deck.iter().any(|c| c.card_id == CardName::GraspingVines);
    let has_totem_of_power = deck.iter().any(|c| c.card_id == CardName::TotemOfPower);
    let _has_corrosive_bubble = deck.iter().any(|c| c.card_id == CardName::CorrosiveBubbles);
    let has_ravenous_swarm = deck.iter().any(|c| c.card_id == CardName::RavenousSwarm);
    let _has_ruinous_rain = deck.iter().any(|c| c.card_id == CardName::RuinousRain);

    let has_fusion_bomb = deck.iter().any(|c| c.card_id == CardName::FusionBomb);
    let _has_soul_fire = deck.iter().any(|c| c.card_id == CardName::SoulFire);
    let _has_crushing_instinct = deck.iter().any(|c| c.card_id == CardName::CrushingInstinct);

    let has_blazing_inferno = deck.iter().any(|c| c.card_id == CardName::BlazingInferno);
    let has_amplify = deck.iter().any(|c| c.card_id == CardName::Amplify);
    let has_grim_shadow = deck.iter().any(|c| c.card_id == CardName::GrimShadow);
    let has_decaying_strike = deck.iter().any(|c| c.card_id == CardName::DecayingStrike);
    let has_radioactivity = deck.iter().any(|c| c.card_id == CardName::Radioactivity);
    let has_thriving_plague = deck.iter().any(|c| c.card_id == CardName::ThrivingPlague);
    let _has_electro_zap = deck.iter().any(|c| c.card_id == CardName::ElectroZap);
    let has_prismatic_rift = deck.iter().any(|c| c.card_id == CardName::PrismaticRift);
    let has_inspiring_force = deck.iter().any(|c| c.card_id == CardName::InspiringForce);

    // Rule 1: Deck must include a support card or maelstrom or GuardBreak
    if !has_support && !has_maelstrom && !has_guard_break {
        return false;
    }
    if IS_CHECK_CARD_SYNERGY {
        println!("Rule 1 PASS")
    }

    // Rule 2 : Purify card require 1 alffication. but cannot be maelstrom.
    // If any high proc chance affliction is usable, Purify should only use that bucket.
    if has_purify {
        if !has_affliction || has_maelstrom || has_fusion_bomb {
            return false;
        }
        let has_priority_affliction_available = sim_stats
            .usable_card
            .iter()
            .any(|card_name| is_purify_priority_affliction(*card_name));
        if has_priority_affliction_available
            && deck
                .iter()
                .filter(|card| card.cardtype == CardType::Affliction)
                .any(|card| !is_purify_priority_affliction(card.card_id))
        {
            return false;
        }
    }
    if IS_CHECK_CARD_SYNERGY {
        println!("Rule 2 PASS")
    }

    // Rule 3 : has Radiant also must have1 burst + 1 affliction
    if has_radiant_kaleidoscope {
        if burst_count != 1 || affliction_count != 1 {
            return false;
        }
    }
    if IS_CHECK_CARD_SYNERGY {
        println!("Rule 3 PASS")
    }

    // //Rule 4 Burst support must use with burst card or other support card
    // if has_ancestral_favor {
    //     if burst_count < 1 {
    //         return false;
    //     }
    //     if affliction_count == 1 && !has_maelstrom {
    //         return false;
    //     }
    // }
    // if IS_CHECK_CARD_SYNERGY {
    //     println!("Rule 4 PASS")
    // }

    //Rule 5 Affliction support must use with burst card or other support card
    // if has_rancid_gas {
    //     if affliction_count < 1 {
    //         return false;
    //     }
    //     if burst_count == 1 && !has_guard_break {
    //         return false;
    //     }
    // }
    // if IS_CHECK_CARD_SYNERGY {
    //     println!("Rule 5 PASS")
    // }

    //Rule 6 never 3 support card
    if support_count == 3 {
        return false;
    }
    if IS_CHECK_CARD_SYNERGY {
        println!("Rule 6 PASS")
    }

    // //Rule 7 : Sand of Time card must use with another debuff inflict card
    // if has_sands_of_time {
    //     if affliction_count <= 1 {
    //         return false;
    //     }
    //     if has_maelstrom && affliction_count == 2 {
    //         return false;
    //     }
    // }
    // if IS_CHECK_CARD_SYNERGY {
    //     println!("Rule 7 PASS")
    // }

    //rule 8 : celestial card not suit with limb support card
    // if has_celestial_static {
    //     if has_grasping_vines || has_totem_of_power {
    //         return false;
    //     }
    // }
    // if IS_CHECK_CARD_SYNERGY {
    //     println!("Rule 8 PASS")
    // }

    //rule 9
    // have no damage card.
    if support_count == 3
        || (support_count == 2 && has_maelstrom)
        || (support_count == 2 && has_guard_break)
        || (support_count == 1 && has_maelstrom && has_guard_break)
    {
        return false;
    }
    if IS_CHECK_CARD_SYNERGY {
        println!("Rule 9 PASS")
    }

    //rule 10
    // have whip must also have other afflcition
    if has_whip {
        if affliction_count < 1 {
            return false;
        }
        // if has_electro_zap {
        //     return false;
        // }
    }

    if IS_CHECK_CARD_SYNERGY {
        println!("Rule 10 PASS")
    }

    //rule 11
    //some affliction should not use with sot
    // if has_sands_of_time {
    //     if has_corrosive_bubble || has_ravenous_swarm || has_ruinous_rain || has_totem_of_power {
    //         return false;
    //     }
    // }

    //rule 12
    // if has_fusion_bomb {
    //     if has_totem_of_power || has_soul_fire || has_crushing_instinct {
    //         return false;
    //     }
    // }

    //rule 14
    //2 support cards must intersect some boss part
    // if has_soul_fire || has_crushing_instinct {
    //     if has_grasping_vines {
    //         return false;
    //     }
    // }
    //rule 15
    // has totem with spread type affliction without purify
    if has_totem_of_power && !has_purify {
        if has_blazing_inferno
            || has_amplify
            || has_grim_shadow
            || has_decaying_strike
            || has_fusion_bomb
            || has_radioactivity
            || has_ravenous_swarm
            || has_thriving_plague
        {
            return false;
        }
    }
    //rule 16
    if has_inspiring_force && has_prismatic_rift {
        return false;
    }
    true
}

pub(super) fn is_deck_boss_suitable(sim_stats: &SimStats, c1: &Card, c2: &Card, c3: &Card) -> bool {
    let boss = &sim_stats.boss_stat;
    let deck = [c1, c2, c3];

    // If every attackable part is already gone, there is no useful target left.
    let has_any_active_attackable_part = sim_stats
        .attackable_part
        .iter()
        .map(|part_name| boss.part(*part_name))
        .any(|part| part.part_state != PartState::Skeleton);

    if !has_any_active_attackable_part {
        return false;
    }
    //Policy 2 : card must be synergy to boss state

    let has_grasping_vines = deck.iter().any(|c| c.card_id == CardName::GraspingVines);
    let has_celestial_static = deck.iter().any(|c| c.card_id == CardName::CelestialStatic);
    let has_prismatic_rift = deck.iter().any(|c| c.card_id == CardName::PrismaticRift);
    let has_inspiring_force = deck.iter().any(|c| c.card_id == CardName::InspiringForce);
    let has_crushing_instinct = deck.iter().any(|c| c.card_id == CardName::CrushingInstinct);
    let has_soul_fire = deck.iter().any(|c| c.card_id == CardName::SoulFire);

    //Rule 1 : if have Limb Support, boss must have limb attackable or not skeleton
    if has_grasping_vines {
        let boss_has_active_limb = sim_stats
            .attackable_part
            .iter()
            .copied()
            .filter(BossPartName::is_limb)
            .any(|part_name| boss.part(part_name).part_state != PartState::Skeleton);

        if !boss_has_active_limb {
            return false;
        }
    }
    //Rule 2 : if have celestial_static, boss must have one limb that's not skeleton
    // (even is not select as target it can attack that to build stack)
    if has_celestial_static {
        let boss_has_any_limb = BossPartName::iter()
            .filter(BossPartName::is_limb)
            .any(|part_name| boss.part(part_name).part_state != PartState::Skeleton);

        if !boss_has_any_limb {
            return false;
        }
    }
    // Rule 3 : if use Prismatic Rift, boss must have attackable armor
    if has_prismatic_rift {
        let boss_has_active_armor = sim_stats.attackable_part.iter().copied().any(|part_name| {
            matches!(
                boss.part(part_name).part_state,
                PartState::Armor | PartState::Cursed
            )
        });

        if !boss_has_active_armor {
            return false;
        }
    }
    //Rule 4 : if use Inspiring Force, boss must have attackable body
    if has_inspiring_force {
        let boss_has_active_body = sim_stats
            .attackable_part
            .iter()
            .copied()
            .any(|part_name| boss.part(part_name).part_state == PartState::Body);

        if !boss_has_active_body {
            return false;
        }
    }
    if has_inspiring_force && has_prismatic_rift {
        return false;
    }
    //Rule 5 :if use Crushing Instinct or Soul Fire, boss must have attakable Head or Torso
    if has_crushing_instinct || has_soul_fire {
        let boss_has_active_head_or_torso =
            sim_stats.attackable_part.iter().copied().any(|part_name| {
                (part_name == BossPartName::Head || part_name == BossPartName::Torso)
                    && boss.part(part_name).part_state != PartState::Skeleton
            });

        if !boss_has_active_head_or_torso {
            return false;
        }
    }
    true
}
