use itertools::Itertools;
use serde::{Deserialize, Serialize};
use crate::models::boss::{Boss, BossPartName};
use crate::models::cards::{Card, CardName, CardType};
use crate::models::player_raid_data::{self, PlayerRaidData, RaidCardResearch, RaidSet, TitanSoulResearch};
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
        //generate deck 
        let valid_deck = generate_deck(&sim_stats.player_stat.card_list, &sim_stats.usable_card);
        //for each deck
        for deck in &valid_deck{
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
                //loop all pattern
                    //loop 20 try 
                        //simulate deck to boss
                        //store total damage of the deck
                    // calculate average damage of the deck and save
        }
        println!("Total synergistic decks created: {}", valid_deck.len());
    }
}

pub fn generate_deck(card_list: &[Card], usable_card: &[CardName]) -> Vec<Vec<Card>>{
    // 1. Only pick cards that are in the user's explicit usable list
    let filtered_cards: Vec<Card> = card_list
        .iter()
        .filter(|card| usable_card.contains(&card.card_id))
        .cloned()
        .collect();

    let mut deck_combinations = Vec::new();

    // 2. Form groups of exactly 3 unique cards
    for combo in filtered_cards.iter().combinations(3) {
        let c1 = combo[0];
        let c2 = combo[1];
        let c3 = combo[2];

        // 3. Keep the deck only if it is synergistic!
        if is_deck_synergistic(c1, c2, c3) {
            // Dereference the pointers to store clean Card values
            deck_combinations.push(vec![c1.clone(), c2.clone(), c3.clone()]);
        }
    }

    deck_combinations
}

fn is_deck_synergistic(c1: &Card, c2: &Card, c3: &Card) -> bool {
    let deck = [c1, c2, c3];
    let burst_count = deck.iter().filter(|c| c.cardtype == CardType::Burst).count();
    let affliction_count = deck.iter().filter(|c| c.cardtype == CardType::Affliction).count();
    let support_count = deck.iter().filter(|c| c.cardtype == CardType::Support).count();
    
    //total deck without any rule = 42*41*40/3/2 = 11480
    // Rule 1: Deck must include a support card or maelstrom or GuardBreak
    let has_support = support_count > 0;
    let has_maelstrom = deck.iter().any(|c| c.card_id == CardName::Maelstrom);
    let has_guard_break = deck.iter().any(|c| c.card_id == CardName::GuardBreak);
    if !has_support && !has_maelstrom && !has_guard_break{
        return false;
    }
    //deck with rule 1 = 8880
    
    true 
}