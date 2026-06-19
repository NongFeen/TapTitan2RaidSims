use itertools::Itertools;
use rand::random;
use serde::{Deserialize, Serialize};
use strum::IntoEnumIterator;
use crate::models::boss::{Boss, BossPartName, DamageSource, PartState};
use crate::models::cards::{Card, CardName, CardType};
use crate::models::player_raid_data::PlayerRaidData;
use crate::models::sim_payload::SimPayLoad;
use super::attack_pattern::generate_attack_patterns;
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
            //loop all pattern
            for pattern in &attack_patterns {
                // println!("  Pattern: {}", pattern.describe());
                // sysdex+=1;
                    //loop 20 try 
                            //simulate deck to boss
                    //store total damage of the deck
                        // calculate average damage of the deck and save
            }
            
        }
        println!(
            "Total synergistic decks created : {} and total pattern {}",
            // debug_card,
            index,
            sysdex
        );
    }
    pub fn run_deck_simulation(payload: SimPayLoad){
        let sim_stats = SimStats{
            player_stat : payload.player_raid_data,
            boss_stat : payload.boss_data,
            attackable_part: payload.attackable_part,
            usable_card : payload.usable_card,
        };

        let player_cards = &sim_stats.player_stat.card_list;
        let usable_cards = &sim_stats.usable_card;

        //current deck
        let deck: Vec<Card> = usable_cards
            .iter()
            .filter_map(|card_name| {
                player_cards
                    .iter()
                    .find(|card| card.card_id == *card_name)
                    .cloned()
            })
            .collect();

        if deck.len() != 3 {
            println!(
                "Deck simulation requires exactly 3 cards, but received {}.",
                deck.len()
            );
            return;
        }

        println!(
            "Deck ready: [{}, {}, {}]",
            deck[0].card_id.display_name(),
            deck[1].card_id.display_name(),
            deck[2].card_id.display_name()
        );

        let mut boss = sim_stats.boss_stat.clone();
        println!("Boss Head hp {}",boss.head.current_health);
        
        for _ in 1..=600 {
            Self::tap_boss(
                &mut boss,
                BossPartName::Head,
                &deck,
                &sim_stats.player_stat,
            );
        }
        println!("Boss Head hp {}",boss.head.current_health);
        println!("{}", boss.getDamageResult());
        
    }

    fn tap_boss(
        boss: &mut Boss,
        attack_part: BossPartName,
        deck: &[Card],
        player_raid_data: &PlayerRaidData,
    ) {
        let base_damage = player_raid_data.player_raid_base_damage as u64;
        boss.on_hit_with_source(attack_part, base_damage, DamageSource::Tap);
        for card in deck.iter() {
            if card.cardtype != CardType::Burst{
                
                continue;
            }

            let proc_chance = card.get_proc_chance(boss);
            let roll: f64 = random();

            if roll <= proc_chance {
                
                let proc_damage = card.on_proc(boss, attack_part, base_damage as f64, 0, 0);
                boss.on_hit_with_source(
                    attack_part,
                    proc_damage.max(0.0).round() as u64,
                    DamageSource::Card(card.card_id),
                );
            }
        }
    }

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
