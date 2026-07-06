use super::attack_pattern::{AttackPattern, generate_attack_patterns};
use super::card_function::support::totem_of_power::{self, PendingTotem};
use crate::models::boss::{Boss, BossPartName, PartState};
use crate::models::card_skill_data::{
    card_skill_bonusamountC, card_skill_bonusamountD, card_skill_row,
};
use crate::models::cards::{Card, CardName, CardType};
use crate::models::damage_source::DamageSource;
use crate::models::player_raid_data::PlayerRaidData;
use crate::models::sim_payload::SimPayLoad;
use crate::models::support_modifier::SupportModifiers;
use itertools::Itertools;
use rand::random;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use strum::IntoEnumIterator;
// use super::super::sim_payload::SimPayLoad;

#[derive(Debug, Serialize, Deserialize)]
pub struct SimStats {
    pub player_stat: PlayerRaidData,
    pub boss_stat: Boss,
    pub attackable_part: Vec<BossPartName>,
    pub usable_card: Vec<CardName>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimRunResult {
    pub total_decks: usize,
    pub rounds_per_pattern: u64,
    pub ticks_per_round: u32,
    pub decks: Vec<SimDeckResult>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimDeckResult {
    pub deck: Vec<CardName>,
    pub deck_names: Vec<String>,
    pub total_attack_patterns: usize,
    pub best_pattern: Option<SimPatternResult>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimPatternResult {
    pub pattern: String,
    pub average_damage: u64,
    pub average_damage_display: String,
    pub lowest_round_damage: u64,
    pub lowest_round_damage_display: String,
    pub highest_round_damage: u64,
    pub highest_round_damage_display: String,
    pub card_damage: Vec<SimCardDamageResult>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimCardDamageResult {
    pub card: CardName,
    pub card_name: String,
    pub average_damage: u64,
    pub average_damage_display: String,
}

pub struct SimProgress {
    current_pattern: usize,
    total_patterns: usize,
}

const SIMS_ROUNDS: u64 = 1;
const TICKS_PER_ROUND: u32 = 600;

pub struct SimService;

impl SimService {
    pub fn run_simulation(payload: SimPayLoad) -> SimRunResult {
        let sim_stats = SimStats {
            player_stat: payload.player_raid_data,
            boss_stat: payload.boss_data,
            attackable_part: payload.attackable_part,
            usable_card: payload.usable_card,
        };

        let valid_decks = generate_deck(&sim_stats);
        let deck_patterns = valid_decks
            .into_iter()
            .map(|deck| {
                let attack_patterns = generate_attack_patterns(&sim_stats, &deck);
                (deck, attack_patterns)
            })
            .collect::<Vec<_>>();
        let mut progress = SimProgress {
            current_pattern: 0,
            total_patterns: deck_patterns
                .iter()
                .map(|(_, attack_patterns)| attack_patterns.len())
                .sum(),
        };
        let decks = deck_patterns
            .into_iter()
            .map(|(deck, attack_patterns)| {
                Self::run_deck_sim(
                    &sim_stats,
                    deck,
                    attack_patterns,
                    SIMS_ROUNDS,
                    Some(&mut progress),
                )
            })
            .collect::<Vec<_>>();

        return SimRunResult {
            total_decks: decks.len(),
            rounds_per_pattern: SIMS_ROUNDS,
            ticks_per_round: TICKS_PER_ROUND,
            decks,
        };
    }

    pub fn run_deck_simulation(payload: SimPayLoad) -> Option<SimDeckResult> {
        let sim_stats = SimStats {
            player_stat: payload.player_raid_data,
            boss_stat: payload.boss_data,
            attackable_part: payload.attackable_part,
            usable_card: payload.usable_card,
        };

        let mut select_deck: Vec<Card> = sim_stats
            .usable_card
            .iter()
            .filter_map(|card_name| {
                sim_stats
                    .player_stat
                    .card_list
                    .iter()
                    .find(|card| card.card_id == *card_name)
                    .cloned()
            })
            .collect();

        apply_amplify_level_sharing(&mut select_deck);

        if select_deck.len() != 3 {
            return None;
        }

        let attack_patterns = generate_attack_patterns(&sim_stats, &select_deck);
        Some(Self::run_deck_sim(
            &sim_stats,
            select_deck,
            attack_patterns,
            10,
            None,
        ))
    }

    pub fn run_deck_sim(
        sim_stats: &SimStats,
        select_deck: Vec<Card>,
        attack_patterns: Vec<AttackPattern>,
        round: u64,
        mut progress: Option<&mut SimProgress>,
    ) -> SimDeckResult {
        let deck = select_deck
            .iter()
            .map(|card| card.card_id)
            .collect::<Vec<_>>();
        let deck_names = select_deck
            .iter()
            .map(|card| card.card_id.display_name().to_string())
            .collect::<Vec<_>>();
        let total_attack_patterns = attack_patterns.len();

        if attack_patterns.is_empty() {
            return SimDeckResult {
                deck,
                deck_names,
                total_attack_patterns,
                best_pattern: None,
            };
        }

        let sim_rounds = round;
        let tap_count = TICKS_PER_ROUND;
        let mut pattern_results: Vec<SimPatternResult> = Vec::new();

        for (pattern_index, pattern) in attack_patterns.into_iter().enumerate() {
            let pattern_name = pattern.describe();
            let mut total_sim_damage: u64 = 0;
            let mut lowest_round_damage = u64::MAX;
            let mut highest_round_damage = 0;
            let mut card_damage_totals: HashMap<CardName, u64> = HashMap::new();
            let mut card_proc_totals: HashMap<CardName, u64> = HashMap::new();

            for _ in 1..=sim_rounds {
                let mut boss = sim_stats.boss_stat.clone();
                boss.set_player_raid_data(sim_stats.player_stat.clone());
                let mut total_burst_proc: u32 = 0;
                let mut deck = select_deck.clone();
                let totem_card = deck
                    .iter()
                    .find(|card| card.card_id == CardName::TotemOfPower)
                    .cloned();
                let mut pending_totems: Vec<PendingTotem> = Vec::new();
                let mut next_totem_spawn_tick = totem_of_power::first_spawn_tick();
                let mut last_target: Option<BossPartName> = None;

                for i in 0..TICKS_PER_ROUND {
                    if i < tap_count {
                        if let Some(current_target) = pattern.next_target(
                            &boss,
                            last_target,
                            &deck,
                            &sim_stats.attackable_part,
                        ) {
                            last_target = Some(current_target);

                            if let Some(totem_card) = &totem_card {
                                totem_of_power::update(
                                    &mut pending_totems,
                                    totem_card,
                                    &deck,
                                    &mut boss,
                                    i,
                                );
                            }

                            Self::tap_boss(
                                &mut boss,
                                current_target,
                                &mut deck,
                                &sim_stats.player_stat,
                                &mut total_burst_proc,
                                1.0,
                                &mut card_proc_totals,
                            );

                            if trigger_astral_echo_extra_tap(&mut deck) {
                                let astral_proc_chance_scale =
                                    card_skill_bonusamountD(CardName::AstralEcho).unwrap_or(0.5);
                                Self::tap_boss(
                                    &mut boss,
                                    current_target,
                                    &mut deck,
                                    &sim_stats.player_stat,
                                    &mut total_burst_proc,
                                    astral_proc_chance_scale,
                                    &mut card_proc_totals,
                                );
                            }

                            if let Some(totem_card) = &totem_card {
                                totem_of_power::try_spawn(
                                    &mut pending_totems,
                                    totem_card,
                                    &boss,
                                    current_target,
                                    i,
                                    &mut next_totem_spawn_tick,
                                );
                            }
                        }
                    }

                    boss.update();
                }

                let round_damage = boss.get_total_damage();
                total_sim_damage += round_damage;
                lowest_round_damage = lowest_round_damage.min(round_damage);
                highest_round_damage = highest_round_damage.max(round_damage);

                for result in boss.damage_results.iter() {
                    if let DamageSource::Card(card_name) = result.source {
                        *card_damage_totals.entry(card_name).or_insert(0) += result.damage;
                    }
                }
            }

            let average_damage = total_sim_damage / sim_rounds;
            let lowest_round_damage = if lowest_round_damage == u64::MAX {
                0
            } else {
                lowest_round_damage
            };

            let card_damage = select_deck
                .iter()
                .map(|card| {
                    let average_damage =
                        card_damage_totals.get(&card.card_id).copied().unwrap_or(0) / sim_rounds;
                    SimCardDamageResult {
                        card: card.card_id,
                        card_name: card.card_id.display_name().to_string(),
                        average_damage,
                        average_damage_display: format_compact(average_damage),
                    }
                })
                .collect();

            pattern_results.push(SimPatternResult {
                pattern: pattern_name.clone(),
                average_damage,
                average_damage_display: format_compact(average_damage),
                lowest_round_damage,
                lowest_round_damage_display: format_compact(lowest_round_damage),
                highest_round_damage,
                highest_round_damage_display: format_compact(highest_round_damage),
                card_damage,
            });

            let card_summary = select_deck
                .iter()
                .map(|card| {
                    let average_damage =
                        card_damage_totals.get(&card.card_id).copied().unwrap_or(0) / sim_rounds;
                    let average_proc_count = format_average_count(
                        card_proc_totals.get(&card.card_id).copied().unwrap_or(0),
                        sim_rounds,
                    );

                    format!(
                        "{} dmg {} proc {}",
                        card.card_id.display_name(),
                        format_compact(average_damage),
                        average_proc_count
                    )
                })
                .collect::<Vec<_>>()
                .join(" | ");

            let progress_summary = if let Some(progress) = progress.as_deref_mut() {
                progress.current_pattern += 1;
                let percent = if progress.total_patterns == 0 {
                    100.0
                } else {
                    (progress.current_pattern as f64 / progress.total_patterns as f64) * 100.0
                };

                format!(
                    " {}/{} ({:.2}%)",
                    progress.current_pattern, progress.total_patterns, percent
                )
            } else {
                format!("{}/{}", pattern_index + 1, total_attack_patterns)
            };

            println!(
                "[SIMs] {} | {} : avg {} l {} h {} | {}",
                progress_summary,
                pattern_name,
                format_compact(average_damage),
                format_compact(lowest_round_damage),
                format_compact(highest_round_damage),
                card_summary
            );
        }

        pattern_results.sort_by(|a, b| b.average_damage.cmp(&a.average_damage));

        SimDeckResult {
            deck,
            deck_names,
            total_attack_patterns,
            best_pattern: pattern_results.into_iter().next(),
        }
    }

    fn tap_boss(
        boss: &mut Boss,
        attack_part: BossPartName,
        deck: &mut [Card],
        player_raid_data: &PlayerRaidData,
        total_burst_proc: &mut u32,
        proc_chance_scale: f64,
        card_proc_totals: &mut HashMap<CardName, u64>,
    ) {
        if boss.get_state_from_part(attack_part) == PartState::Skeleton {
            return;
        }

        let current_state = boss.get_state_from_part(attack_part);

        // flat addition & card research
        let flat_part_state_add =
            player_raid_data.get_total_part_state_add(attack_part, current_state);
        let flat_boss_add = player_raid_data.get_total_boss_add(boss.boss_name);

        let base1_set = if player_raid_data.raid_set.jukk_juggernaut {
            100.0
        } else {
            0.0
        };
        let base2_set = if player_raid_data.raid_set.rose_anniversary {
            100.0
        } else {
            0.0
        };

        let base_add_total = (player_raid_data.raid_card_research.base_damage
            + player_raid_data.gem_stone_research.base_damage) as f32;

        let true_base_tap = (player_raid_data.player_raid_base_damage as f32)
            + base_add_total
            + flat_part_state_add
            + flat_boss_add
            + base1_set
            + base2_set;

        let burst_add_total = (player_raid_data.raid_card_research.base_burst_damage
            + player_raid_data.gem_stone_research.base_burst_damage)
            as f32
            + player_raid_data.get_total_card_type_boss_add(boss.boss_name, CardType::Burst)
            + (if player_raid_data.raid_set.airforce_ace {
                120.0
            } else {
                0.0
            });

        let affli_add_total = (player_raid_data.raid_card_research.base_affliction_damage
            + player_raid_data.gem_stone_research.base_affliction_damage)
            as f32
            + player_raid_data.get_total_card_type_boss_add(boss.boss_name, CardType::Affliction)
            + (if player_raid_data.raid_set.dancer_venom {
                120.0
            } else {
                0.0
            });

        //support card
        let deck_snapshot: Vec<Card> = deck.to_vec();
        let support_mods: Vec<SupportModifiers> = deck
            .iter_mut()
            .filter(|c| c.cardtype == CardType::Support)
            .map(|c| c.support_modifiers(boss, deck_snapshot.clone()))
            .collect();

        let combined_support = SupportModifiers::accumulate(&support_mods);
        // println!("Support Card {}", combined_support);

        boss.set_support_modifiers(combined_support.clone());

        // card proc
        for card in deck.iter_mut() {
            if !matches!(card.cardtype, CardType::Burst | CardType::Affliction) {
                continue;
            }

            let card_type_add_total = match card.cardtype {
                CardType::Burst => burst_add_total,
                CardType::Affliction => affli_add_total,
                CardType::Support => 0.0,
            };
            let card_base_damage = (true_base_tap + card_type_add_total) as f64;
            // println!("true_base_tap{} , burst_add_total{}, card_base_damage {}, ",true_base_tap,burst_add_total,card_base_damage);

            let chance_mult = match card.cardtype {
                CardType::Burst => combined_support.burst_chance_mult,
                CardType::Affliction => combined_support.affliction_chance_mult,
                _ => 1.0, // unreachable given the matches! filter above, but keeps the match exhaustive
            };
            // println!(
            //     "card_base_damage : {} true_base_tap {}",
            //     card_base_damage, true_base_tap
            // );
            let card_proc_chance = card.get_proc_chance(boss);
            let proc_chance = if proc_chance_scale < 1.0 && card_proc_chance >= 1.0 {
                1.0
            } else {
                card_proc_chance * chance_mult * proc_chance_scale
            };
            // println!("Proc Chance {} {} {} {} {}",proc_chance,card_proc_chance,chance_mult, proc_chance_scale, combined_support.burst_chance_mult);
            let roll: f64 = random(); // Assuming random() yields an f64 from rand crate
            if roll <= proc_chance {
                if card.cardtype == CardType::Burst {
                    *total_burst_proc += 1;
                }
                *card_proc_totals.entry(card.card_id).or_insert(0) += 1;
                card.on_proc(boss, attack_part, card_base_damage, 0, *total_burst_proc);
            }
        }

        // tap damage on boss
        let tap_damage = true_base_tap as u64;
        boss.on_hit_with_source(attack_part, tap_damage, DamageSource::Tap);
    }
}

pub fn generate_deck(sim_stats: &SimStats) -> Vec<Vec<Card>> {
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
        // println!(
        //     "Checking deck combination: {}, {}, {}",
        //     c1.card_id.display_name(),
        //     c2.card_id.display_name(),
        //     c3.card_id.display_name()
        // );
        // 3. Keep the deck only if it is synergistic and boss-compatible!
        if is_deck_synergistic(sim_stats, c1, c2, c3)
            && is_deck_boss_suitable(sim_stats, c1, c2, c3)
        {
            // Dereference the pointers to store clean Card values
            let mut deck = vec![c1.clone(), c2.clone(), c3.clone()];
            // apply_amplify_level_sharing(&mut deck);
            deck_combinations.push(deck);
        }
    }

    deck_combinations
}

fn is_deck_synergistic(_sim_stats: &SimStats, c1: &Card, c2: &Card, c3: &Card) -> bool {
    
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

    // Rule 1: Deck must include a support card or maelstrom or GuardBreak
    let has_support = support_count > 0;
    let has_maelstrom = deck.iter().any(|c| c.card_id == CardName::Maelstrom);
    let has_guard_break = deck.iter().any(|c| c.card_id == CardName::GuardBreak);
    if !has_support && !has_maelstrom && !has_guard_break {
        return false;
    }
    //deck with rule 1 = 8880

    // Rule 2 : Purify card require 1 alffication. but cannot be maelstrom
    let has_purify = deck.iter().any(|c| c.card_id == CardName::PurifyingBlast);
    let has_affliction = affliction_count > 0;
    if has_purify && !has_affliction  {
        return false;
    }
    if has_purify && has_maelstrom {
        return false;
    }
    //deck with rule 2 = 8595
    // Rule 3 : has Radiant also must have1 burst + 1 affliction
    let has_radiant_kaleidoscope = deck
        .iter()
        .any(|c| c.card_id == CardName::RadiantKaleidoscope);
    if has_radiant_kaleidoscope {
        if burst_count != 1 || affliction_count != 1 {
            return false;
        }
    }
    //deck with rule 3 = 7997
    //Rule 4 Burst support must use with burst card or other support card
    let has_ancestral_favor = deck.iter().any(|c| c.card_id == CardName::AncestralFavor);
    if has_ancestral_favor {
        if affliction_count >= 1 || support_count == 3 {
            return false;
        }
    }
    //deck with rule 4 = 7476
    //Rule 5 Affliction support must use with burst card or other support card
    let has_rancid_gas = deck.iter().any(|c| c.card_id == CardName::RancidGas);
    if has_rancid_gas {
        if burst_count >= 1 || support_count == 3 {
            return false;
        }
    }
    // //deck with rule 5 = 6991
    //Rule 6 never 3 support card
    if support_count == 3 {
        return false;
    }
    //deck with rule 6 = 6826
    // //Rule 7 : Sand of Time card must use with another debuff inflict card
    let has_sands_of_time = deck.iter().any(|c| c.card_id == CardName::SandsOfTime);
    if has_sands_of_time {
        if affliction_count <= 1 {
            return false;
        }
        if has_maelstrom && affliction_count == 2 {
            return false;
        }
    }
    //deck with rule 7 = 6553

    //rule 8 : celestial card not suit with limb support card
    let has_celestial_static = deck.iter().any(|c| c.card_id == CardName::CelestialStatic);
    let has_grasping_vines = deck.iter().any(|c| c.card_id == CardName::GraspingVines);
    if has_celestial_static && has_grasping_vines {
        return false;
    }

    //rule 9
    // have no damage card.
    if support_count == 3 || (support_count == 2 && has_maelstrom) || (support_count == 2 && has_guard_break) || (support_count == 1 && has_maelstrom && has_guard_break) {
        return false;
    }

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

fn format_compact(damage: u64) -> String {
    let damage_f = damage as f64;
    if damage >= 1_000_000_000_000 {
        format!("{:.3}T", damage_f / 1_000_000_000_000.0)
    } else if damage >= 1_000_000_000 {
        format!("{:.3}B", damage_f / 1_000_000_000.0)
    } else if damage >= 1_000_000 {
        format!("{:.3}M", damage_f / 1_000_000.0)
    } else if damage >= 1_000 {
        format!("{:.3}K", damage_f / 1_000.0)
    } else {
        damage.to_string()
    }
}

fn format_average_count(total_count: u64, rounds: u64) -> String {
    if rounds == 0 {
        return "0".to_string();
    }

    let average = total_count as f64 / rounds as f64;
    if (average.fract()).abs() < f64::EPSILON {
        format!("{:.0}", average)
    } else {
        format!("{:.2}", average)
    }
}

fn apply_amplify_level_sharing(deck: &mut [Card]) {
    let Some(amplify_level) = deck
        .iter()
        .find(|card| card.card_id == CardName::Amplify)
        .map(|card| card.level)
    else {
        return;
    };

    let share_rate = card_skill_bonusamountC(CardName::Amplify).unwrap_or(0.1);
    let shared_levels = (amplify_level as f64 * share_rate).ceil().max(1.0) as u16;

    for card in deck
        .iter_mut()
        .filter(|card| card.card_id != CardName::Amplify)
    {
        let max_level = card_skill_row(card.card_id)
            .map(|row| row.max_level)
            .unwrap_or(u16::MAX);
        card.level = card.level.saturating_add(shared_levels).min(max_level);
    }
}

fn trigger_astral_echo_extra_tap(deck: &mut [Card]) -> bool {
    let Some(astral_echo) = deck
        .iter_mut()
        .find(|card| card.card_id == CardName::AstralEcho)
    else {
        return false;
    };

    let max_charges = card_skill_bonusamountC(CardName::AstralEcho)
        .unwrap_or(5.0)
        .max(1.0) as u16;
    astral_echo.tap_count = astral_echo.tap_count.saturating_add(1);

    if astral_echo.tap_count < max_charges {
        return false;
    }

    astral_echo.tap_count = 0;
    true
}
