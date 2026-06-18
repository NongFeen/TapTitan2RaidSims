use itertools::Itertools;
use serde::{Deserialize, Serialize};
use strum::IntoEnumIterator;
use crate::models::boss::{Boss, BossPartName, PartState};
use crate::models::cards::{Card, CardName, CardType};
use crate::models::player_raid_data::PlayerRaidData;
use crate::models::sim_payload::SimPayLoad;
// use super::super::sim_payload::SimPayLoad;

#[derive(Debug, Serialize, Deserialize)]
pub struct SimStats {
    pub player_stat : PlayerRaidData,
    pub boss_stat : Boss,
    pub attackable_part: Vec<BossPartName>,
    pub usable_card : Vec<CardName>,
}

pub struct SimService;

impl SimService {
    pub fn run_simulation(payload: SimPayLoad) {
        //set up stats
        let sim_stats = SimStats{
            player_stat : payload.player_raid_data,
            boss_stat : payload.boss_data,
            attackable_part: payload.attackable_part,
            usable_card : payload.usable_card,
        };
        let mut index = 0;
        let mut sysdex = 0;
        //debug card
        // let debug_card = CardName::SandsOfTime; // temporary debug filter
        //generate deck 
        let valid_deck = generate_deck(&sim_stats);
        //for each deck
        for deck in &valid_deck{
            //for debug deck
            // if !deck.iter().any(|card| card.card_id == debug_card) {
            //     continue;
            // }

            let card1 = &deck[0];
            let card2 = &deck[1];
            let card3 = &deck[2];
            index+=1;
            println!(
                "Deck #{}: [{}, {}, {}]",
                index, // Add 1 so it starts counting from 1 instead of 0
                card1.card_id.display_name(),
                card2.card_id.display_name(),
                card3.card_id.display_name()
            );
            //generate attack pattern
            let attack_patterns = generate_attack_patterns(&sim_stats, deck);
            for pattern in &attack_patterns {
                println!("  Pattern: {}", pattern.describe());
                sysdex+=1;
                println!(
                    "  Next target: {:?}",
                    pattern.next_target(&sim_stats.boss_stat, None, deck)
                );
            }
                //loop all pattern
                    //loop 20 try 
                        //simulate deck to boss
                //store total damage of the deck
                    // calculate average damage of the deck and save
            
        }
        println!(
            "Total synergistic decks created : {} and total pattern {}",
            // debug_card,
            index,
            sysdex
        );
    }
}

#[derive(Debug, Clone)]
pub enum AttackPattern {
    Single(BossPartName),
    Ordered(Vec<BossPartName>),
    AnyLimb,
}

impl AttackPattern {
    pub fn describe(&self) -> String {
        match self {
            AttackPattern::Single(part) => format!("Single({:?})", part),
            AttackPattern::Ordered(parts) => format!("Ordered({:?})", parts),
            AttackPattern::AnyLimb => "AnyLimb".to_string(),
        }
    }

    pub fn next_target(
        &self,
        boss: &Boss,
        last_target: Option<BossPartName>,
        deck: &[Card],
    ) -> Option<BossPartName> {
        let candidates = self.candidate_parts(boss);
        if candidates.is_empty() {
            return None;
        }

        if deck.iter().any(|card| card.card_id == CardName::FusionBomb) {
            if let Some(open_part) = candidates.iter().copied().find(|part| {
                !boss
                    .part(*part)
                    .afflictions
                    .iter()
                    .any(|aff| aff.kind == crate::models::affliction::AfflictionKind::Fusion)
            }) {
                return Some(open_part);
            }

            if let Some(last) = last_target {
                if candidates.contains(&last) {
                    return Some(last);
                }
            }

            return candidates.first().copied();
        }

        match last_target {
            Some(last) => {
                if let Some(index) = candidates.iter().position(|part| *part == last) {
                    candidates
                        .get((index + 1) % candidates.len())
                        .copied()
                        .or_else(|| candidates.first().copied())
                } else {
                    candidates.first().copied()
                }
            }
            None => candidates.first().copied(),
        }
    }

    fn candidate_parts(&self, boss: &Boss) -> Vec<BossPartName> {
        match self {
            AttackPattern::Single(part) => {
                if boss.part(*part).part_state != PartState::Skeleton {
                    vec![*part]
                } else {
                    Vec::new()
                }
            }
            AttackPattern::Ordered(parts) => parts
                .iter()
                .copied()
                .filter(|part| boss.part(*part).part_state != PartState::Skeleton)
                .collect(),
            AttackPattern::AnyLimb => BossPartName::iter()
                .filter(BossPartName::is_limb)
                .filter(|part| boss.part(*part).part_state != PartState::Skeleton)
                .collect(),
        }
    }
}

pub fn generate_attack_patterns(sim_stats: &SimStats, deck: &[Card]) -> Vec<AttackPattern> {
    let mut active_parts: Vec<BossPartName> = sim_stats
        .attackable_part
        .iter()
        .copied()
        .filter(|part| sim_stats.boss_stat.part(*part).part_state != PartState::Skeleton)
        .collect();

    let has_head_torso_support_focus = deck.iter().any(|card| {
        matches!(
            card.card_id,
            CardName::CrushingInstinct | CardName::SoulFire
        )
    });
    let has_limb_only_support_focus = deck.iter().any(|card| {
        matches!(card.card_id, CardName::GraspingVines)
    });
    let has_single_target_support_focus = deck.iter().any(|card| {
        matches!(card.card_id, CardName::TotemOfPower)
    });
    let has_body_only_support_focus = deck.iter().any(|card| {
        matches!(card.card_id, CardName::InspiringForce)
    });
    let has_armor_only_support_focus = deck.iter().any(|card| {
        matches!(card.card_id, CardName::PrismaticRift)
    });

    if has_head_torso_support_focus {
        active_parts.retain(|part| matches!(part, BossPartName::Head | BossPartName::Torso));
    }
    if has_limb_only_support_focus {
        active_parts.retain(BossPartName::is_limb);
    }
    if has_single_target_support_focus {
        // Keep the active parts, but block any multi-part cycling patterns below.
    }
    if has_body_only_support_focus {
        active_parts.retain(|part| {
            matches!(sim_stats.boss_stat.part(*part).part_state, PartState::Body)
        });
    }
    if has_armor_only_support_focus {
        active_parts.retain(|part| {
            matches!(sim_stats.boss_stat.part(*part).part_state, PartState::Armor)
        });
    }

    let mut patterns = Vec::new();

    for part in &active_parts {
        patterns.push(AttackPattern::Single(*part));
    }

    if active_parts.len() >= 2 && !has_single_target_support_focus {
        patterns.push(AttackPattern::Ordered(active_parts.clone()));
    }

    if !has_head_torso_support_focus
        && !has_single_target_support_focus
        && active_parts.iter().any(BossPartName::is_limb)
    {
        patterns.push(AttackPattern::AnyLimb);
    }

    if patterns.is_empty() && !active_parts.is_empty() {
        patterns.push(AttackPattern::Ordered(active_parts));
    }

    patterns
}

pub fn generate_deck(sim_stats: &SimStats) -> Vec<Vec<Card>>{
    // 1. Only pick cards that are in the user's explicit usable list
    let filtered_cards: Vec<Card> = sim_stats
        .player_stat
        .card_list
        .iter()
        .filter(|card| sim_stats.usable_card.contains(&card.card_id))
        .cloned()
        .collect();

    let mut deck_combinations = Vec::new();

    // 2. Form groups of exactly 3 unique cards
    for combo in filtered_cards.iter().combinations(3) {
        let c1 = combo[0];
        let c2 = combo[1];
        let c3 = combo[2];

        // 3. Keep the deck only if it is synergistic and boss-compatible!
        if is_deck_synergistic(sim_stats, c1, c2, c3)
            && is_deck_boss_suitable(sim_stats, c1, c2, c3)
        {
            // Dereference the pointers to store clean Card values
            deck_combinations.push(vec![c1.clone(), c2.clone(), c3.clone()]);
        }
    }

    deck_combinations
}

fn is_deck_synergistic(_sim_stats: &SimStats, c1: &Card, c2: &Card, c3: &Card) -> bool {
    let deck = [c1, c2, c3];
    let burst_count = deck.iter().filter(|c| c.cardtype == CardType::Burst).count();
    let affliction_count = deck.iter().filter(|c| c.cardtype == CardType::Affliction).count();
    let support_count = deck.iter().filter(|c| c.cardtype == CardType::Support).count();
    
    //total deck without any rule = 42*41*40/3/2 = 11480
    //Policy 1 : card must be synergy by it self

    // Rule 1: Deck must include a support card or maelstrom or GuardBreak
    let has_support = support_count > 0;
    let has_maelstrom = deck.iter().any(|c| c.card_id == CardName::Maelstrom);
    let has_guard_break = deck.iter().any(|c| c.card_id == CardName::GuardBreak);
    if !has_support && !has_maelstrom && !has_guard_break{
        return false;
    }
    //deck with rule 1 = 8880
    
    // Rule 2 : Purify card require 1 alffication card
    let has_purify = deck.iter().any(|c| c.card_id == CardName::PurifyingBlast);
    let has_affliction = affliction_count > 0;
    if has_purify && !has_affliction{
        return false;
    }
    //deck with rule 2 = 8595
    // Rule 3 : has Radiant also must have1 burst + 1 affliction
    let has_radiant_kaleidoscope = deck.iter().any(|c| c.card_id == CardName::RadiantKaleidoscope);
    if has_radiant_kaleidoscope {
        if burst_count != 1 || affliction_count != 1 {
        return false;
        }
    }
    //deck with rule 3 = 7997
    //Rule 4 Burst support must use with burst card or other support card
    let has_ancestral_favor = deck.iter().any(|c| c.card_id == CardName::AncestralFavor);
    if has_ancestral_favor{
        if affliction_count >= 1 || support_count ==3{
            return  false;
        }
    }
    //deck with rule 4 = 7476
    //Rule 5 Affliction support must use with burst card or other support card
    let has_rancid_gas = deck.iter().any(|c| c.card_id == CardName::RancidGas);
    if has_rancid_gas{
        if burst_count >= 1 || support_count ==3{
            return  false;
        }
    }
    // //deck with rule 5 = 6991
    //Rule 6 never 3 support card
    if support_count == 3{
        return false;
    }
    //deck with rule 6 = 6826
    // //Rule 7 : Sand of Time card must use with another debuff inflict card 
    let has_sands_of_time = deck.iter().any(|c| c.card_id == CardName::SandsOfTime);
    if has_sands_of_time{
        if affliction_count <= 1{
            return  false;
        }
        if has_maelstrom && affliction_count == 2{
            return  false;
        }
    }
    //deck with rule 7 = 6553
    true 
}

fn is_deck_boss_suitable(sim_stats: &SimStats, c1: &Card, c2: &Card, c3: &Card) -> bool {
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
        let boss_has_active_armor = sim_stats
            .attackable_part
            .iter()
            .copied()
            .any(|part_name| boss.part(part_name).part_state == PartState::Armor);

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
    //Rule 5 :if use Crushing Instinct or Soul Fire, boss must have attakable Head or Torso
    if has_crushing_instinct || has_soul_fire {
        let boss_has_active_head_or_torso = sim_stats
            .attackable_part
            .iter()
            .copied()
            .any(|part_name| {
                    (part_name == BossPartName::Head || part_name == BossPartName::Torso)
                    && boss.part(part_name).part_state != PartState::Skeleton
                });

        if !boss_has_active_head_or_torso {
            return false;
        }
    }
    true
}
