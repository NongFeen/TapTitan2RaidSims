use super::attack_pattern::{AttackPattern, generate_attack_patterns};
use super::card_function::support::totem_of_power::{self, PendingTotem};
use crate::models::boss::{Boss, BossPartName, PartState};
use crate::models::cards::{Card, CardName, CardType};
use crate::models::damage_source::DamageSource;
use crate::models::player_raid_data::PlayerRaidData;
use crate::models::sim_payload::SimPayLoad;
use crate::models::support_modifier::SupportModifiers;
use itertools::Itertools;
use rand::random;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering as AtomicOrdering},
};
use strum::IntoEnumIterator;

const SIMS_ROUNDS: u64 = 20;
const TICKS_PER_ROUND: u32 = 600;
const TICKS_PER_SECOND: f64 = 20.0;
const PRINT_SIM_PATTERN_PROGRESS: bool = true;
const SIM_PATTERN_PROGRESS_STEP_PERCENT: usize = 10;
const PRINT_EVERY_SIM_PATTERN: bool = false;
// const PRINT_EVERY_SIM_PATTERN: bool = true;
const PRINT_PROC_CACHE: bool = false;
const ENABLE_PARALLEL_SIM: bool = true;
const SIM_WORKER_COUNT: usize = 0; // 0 = use available_parallelism()

const GLOBAL_RAID_BURST_DAMAGE_ENABLED: bool = false;
const GLOBAL_RAID_BURST_DAMAGE_MULT: f64 = 1.3;
const GLOBAL_RAID_BURST_CHANCE_ENABLED: bool = false;
const GLOBAL_RAID_BURST_CHANCE_MULT: f64 = 1.3;
const GLOBAL_RAID_SUPPORT_EFFECT_ENABLED: bool = false;
const GLOBAL_RAID_SUPPORT_EFFECT_MULT: f64 = 1.15;
const GLOBAL_RAID_AFFLICTION_CHANCE_ENABLED: bool = false;
const GLOBAL_RAID_AFFLICTION_CHANCE_MULT: f64 = 1.3;
const GLOBAL_RAID_AFFLICTION_DAMAGE_ENABLED: bool = false;
const GLOBAL_RAID_AFFLICTION_DAMAGE_MULT: f64 = 1.3;
const GLOBAL_RAID_ALL_DAMAGE_ENABLED: bool = false;
const GLOBAL_RAID_ALL_DAMAGE_MULT: f64 = 1.15;
const GLOBAL_RAID_ATTACK_DURATION_ENABLED: bool = false;
const GLOBAL_RAID_ATTACK_DURATION_ADD_SECONDS: f64 = 3.0;
const GLOBAL_RAID_AFFLICTION_DURATION_ENABLED: bool = false;
const GLOBAL_RAID_AFFLICTION_DURATION_MULT: f64 = 1.5;

const COSMIC_HAYMAKER_TAPS_PER_PROC: u16 = 70;
const CELESTIAL_STATIC_STACKS_PER_PROC: usize = 8;
const COSMIC_HAYMAKER_FAST_PROC_KEY: u16 = 20000;
const FAST_CALC_CARDS: [CardName; 20] = [
    CardName::MoonBeam,
    CardName::Fragmentize,
    CardName::SkullBash,
    CardName::RazorWind,
    CardName::PsychicShackles,
    CardName::FlakShot,
    CardName::CosmicHaymaker,
    CardName::BarbedMorningstar,
    CardName::CrushingInstinct,
    CardName::InsanityVoid,
    CardName::InspiringForce,
    CardName::SoulFire,
    CardName::VictoryMarch,
    CardName::PrismaticRift,
    CardName::AncestralFavor,
    CardName::GraspingVines,
    CardName::TeamTactics,
    CardName::SkeletalSmash,
    CardName::AstralEcho,
    CardName::BattleDrums,
];

#[derive(Debug, Clone, Copy)]
struct GlobalRaidModifiers {
    burst_damage_mult: f64,
    burst_chance_mult: f64,
    support_effect_mult: f64,
    affliction_chance_mult: f64,
    affliction_damage_mult: f64,
    all_damage_mult: f64,
    attack_duration_add_seconds: f64,
    affliction_duration_mult: f64,
}

fn global_raid_modifiers() -> GlobalRaidModifiers {
    GlobalRaidModifiers {
        burst_damage_mult: if GLOBAL_RAID_BURST_DAMAGE_ENABLED {
            GLOBAL_RAID_BURST_DAMAGE_MULT
        } else {
            1.0
        },
        burst_chance_mult: if GLOBAL_RAID_BURST_CHANCE_ENABLED {
            GLOBAL_RAID_BURST_CHANCE_MULT
        } else {
            1.0
        },
        support_effect_mult: if GLOBAL_RAID_SUPPORT_EFFECT_ENABLED {
            GLOBAL_RAID_SUPPORT_EFFECT_MULT
        } else {
            1.0
        },
        affliction_chance_mult: if GLOBAL_RAID_AFFLICTION_CHANCE_ENABLED {
            GLOBAL_RAID_AFFLICTION_CHANCE_MULT
        } else {
            1.0
        },
        affliction_damage_mult: if GLOBAL_RAID_AFFLICTION_DAMAGE_ENABLED {
            GLOBAL_RAID_AFFLICTION_DAMAGE_MULT
        } else {
            1.0
        },
        all_damage_mult: if GLOBAL_RAID_ALL_DAMAGE_ENABLED {
            GLOBAL_RAID_ALL_DAMAGE_MULT
        } else {
            1.0
        },
        attack_duration_add_seconds: if GLOBAL_RAID_ATTACK_DURATION_ENABLED {
            GLOBAL_RAID_ATTACK_DURATION_ADD_SECONDS
        } else {
            0.0
        },
        affliction_duration_mult: if GLOBAL_RAID_AFFLICTION_DURATION_ENABLED {
            GLOBAL_RAID_AFFLICTION_DURATION_MULT
        } else {
            1.0
        },
    }
}

#[derive(Debug, Clone)]
pub struct SimStats {
    pub player_stat: Arc<PlayerRaidData>,
    pub boss_stat: Boss,
    pub attackable_part: Vec<BossPartName>,
    pub usable_card: Vec<CardName>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimRunResult {
    pub total_decks: usize,
    pub total_attack_patterns: usize,
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
    current_pattern: AtomicUsize,
    total_patterns: usize,
}

type DeckPatternWork = (Vec<Card>, Vec<AttackPattern>);
type IndexedDeckPatternWork = (usize, Vec<Card>, Vec<AttackPattern>);

struct RoundSupportCache {
    support: SupportModifiers,
    state_signature: u16,
    dynamic: bool,
}

impl RoundSupportCache {
    fn new(deck: &mut [Card], boss: &mut Boss) -> Self {
        let support = combined_support_modifiers(deck, boss);
        boss.set_support_modifiers(support.clone());

        Self {
            support,
            state_signature: boss.part_state_signature(),
            dynamic: deck_has_dynamic_support_modifier(deck),
        }
    }

    fn current<'a>(&'a mut self, deck: &mut [Card], boss: &mut Boss) -> &'a SupportModifiers {
        if self.dynamic {
            let state_signature = boss.part_state_signature();
            if state_signature != self.state_signature {
                self.support = combined_support_modifiers(deck, boss);
                self.state_signature = state_signature;
                boss.set_support_modifiers(self.support.clone());
            }
        }

        &self.support
    }

    fn bonus_tap_proc_chance_mult(&self) -> f64 {
        self.support.bonus_tap_proc_chance_mult
    }
}

#[derive(Debug, Clone, Copy)]
struct SimDamageContext {
    base_tap_without_part_state: f32,
    burst_add_total: f32,
    affliction_add_total: f32,
    head_armor_add: f32,
    head_body_add: f32,
    torso_armor_add: f32,
    torso_body_add: f32,
    limb_armor_add: f32,
    limb_body_add: f32,
}

impl SimDamageContext {
    fn new(player_raid_data: &PlayerRaidData, boss: &Boss) -> Self {
        let flat_boss_add = player_raid_data.get_total_boss_add(boss.boss_name);
        let base_set_add = if player_raid_data.raid_set.jukk_juggernaut {
            100.0
        } else {
            0.0
        } + if player_raid_data.raid_set.rose_anniversary {
            100.0
        } else {
            0.0
        };
        let base_add_total = (player_raid_data.raid_card_research.base_damage
            + player_raid_data.gem_stone_research.base_damage) as f32;
        let base_tap_without_part_state = player_raid_data.player_raid_base_damage as f32
            + base_add_total
            + flat_boss_add
            + base_set_add;

        let burst_add_total = (player_raid_data.raid_card_research.base_burst_damage
            + player_raid_data.gem_stone_research.base_burst_damage)
            as f32
            + player_raid_data.get_total_card_type_boss_add(boss.boss_name, CardType::Burst)
            + if player_raid_data.raid_set.airforce_ace {
                120.0
            } else {
                0.0
            };

        let affliction_add_total = (player_raid_data.raid_card_research.base_affliction_damage
            + player_raid_data.gem_stone_research.base_affliction_damage)
            as f32
            + player_raid_data.get_total_card_type_boss_add(boss.boss_name, CardType::Affliction)
            + if player_raid_data.raid_set.dancer_venom {
                120.0
            } else {
                0.0
            };

        Self {
            base_tap_without_part_state,
            burst_add_total,
            affliction_add_total,
            head_armor_add: player_raid_data
                .get_total_part_state_add(BossPartName::Head, PartState::Armor),
            head_body_add: player_raid_data
                .get_total_part_state_add(BossPartName::Head, PartState::Body),
            torso_armor_add: player_raid_data
                .get_total_part_state_add(BossPartName::Torso, PartState::Armor),
            torso_body_add: player_raid_data
                .get_total_part_state_add(BossPartName::Torso, PartState::Body),
            limb_armor_add: player_raid_data
                .get_total_part_state_add(BossPartName::LeftHand, PartState::Armor),
            limb_body_add: player_raid_data
                .get_total_part_state_add(BossPartName::LeftHand, PartState::Body),
        }
    }

    fn true_base_tap(self, part_name: BossPartName, state: PartState) -> f32 {
        self.base_tap_without_part_state + self.part_state_add(part_name, state)
    }

    fn card_type_add(self, card_type: CardType) -> f32 {
        match card_type {
            CardType::Burst => self.burst_add_total,
            CardType::Affliction => self.affliction_add_total,
            CardType::Support => 0.0,
        }
    }

    fn part_state_add(self, part_name: BossPartName, state: PartState) -> f32 {
        let is_armor = matches!(state, PartState::Armor | PartState::Cursed);

        match part_name {
            BossPartName::Head if is_armor => self.head_armor_add,
            BossPartName::Head => self.head_body_add,
            BossPartName::Torso if is_armor => self.torso_armor_add,
            BossPartName::Torso => self.torso_body_add,
            _ if is_armor => self.limb_armor_add,
            _ => self.limb_body_add,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ProcScenario {
    pub proc_chance_basis_points: u16,
    pub is_cosmic_haymaker: bool,
    pub proc_chance_mult_basis_points: u16,
    pub has_astral_echo: bool,
    pub bonus_tap_proc_chance_mult_basis_points: u16,
    pub tap_count: u32,
}

impl ProcScenario {
    pub fn new(
        proc_chance: f32,
        proc_chance_mult: f32,
        has_astral_echo: bool,
        bonus_tap_proc_chance_mult: f32,
        tap_count: u32,
    ) -> Self {
        Self {
            proc_chance_basis_points: proc_chance_to_basis_points(proc_chance),
            is_cosmic_haymaker: false,
            proc_chance_mult_basis_points: mult_to_basis_points(proc_chance_mult),
            has_astral_echo,
            bonus_tap_proc_chance_mult_basis_points: mult_to_basis_points(
                bonus_tap_proc_chance_mult,
            ),
            tap_count,
        }
    }

    pub fn name(&self) -> String {
        format!(
            "chance_{}bp|haymaker_{}|chance_mult_{}bp|echo_{}|echo_scale_{}bp|taps_{}",
            self.proc_chance_basis_points,
            self.is_cosmic_haymaker,
            self.proc_chance_mult_basis_points,
            self.has_astral_echo,
            self.bonus_tap_proc_chance_mult_basis_points,
            self.tap_count
        )
    }

    fn base_proc_chance(&self) -> f32 {
        self.proc_chance_basis_points as f32 / 10_000.0
    }

    fn modified_proc_chance(&self, proc_chance_scale: f32) -> f32 {
        self.base_proc_chance() * self.proc_chance_mult() * proc_chance_scale
    }

    fn proc_chance_mult(&self) -> f32 {
        self.proc_chance_mult_basis_points as f32 / 10_000.0
    }

    fn bonus_tap_proc_chance_mult(&self) -> f32 {
        self.bonus_tap_proc_chance_mult_basis_points as f32 / 10_000.0
    }
}

#[derive(Debug, Clone, Default)]
pub struct PreDeterminedProc {
    pub proc_count_by_scenario: HashMap<ProcScenario, f32>,
}

impl PreDeterminedProc {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn generate_proc_count(&mut self, cards: &[Card], boss: &Boss, tap_count: u32) {
        let proc_chances = Self::fast_calc_burst_proc_chances(cards, boss);
        let support_modifiers = support_modifiers_for_deck(cards, boss);
        let has_astral_echo = cards
            .iter()
            .any(|card| card.card_id == CardName::AstralEcho);

        for (proc_chance_basis_points, is_cosmic_haymaker) in proc_chances {
            let scenario = ProcScenario {
                proc_chance_basis_points,
                is_cosmic_haymaker,
                proc_chance_mult_basis_points: mult_to_basis_points(
                    support_modifiers.burst_chance_mult as f32,
                ),
                has_astral_echo,
                bonus_tap_proc_chance_mult_basis_points: if has_astral_echo {
                    mult_to_basis_points(support_modifiers.bonus_tap_proc_chance_mult as f32)
                } else {
                    mult_to_basis_points(1.0)
                },
                tap_count,
            };

            self.generate_proc_count_for_scenario(scenario);
        }
    }

    pub fn generate_proc_count_for_scenario(&mut self, scenario: ProcScenario) -> f32 {
        if let Some(proc_count) = self.proc_count_by_scenario.get(&scenario).copied() {
            return proc_count;
        }

        if scenario.is_cosmic_haymaker {
            let echo_tap_count = if scenario.has_astral_echo {
                scenario.tap_count / 5
            } else {
                0
            };
            let proc_count = ((scenario.tap_count + echo_tap_count)
                / COSMIC_HAYMAKER_TAPS_PER_PROC as u32) as f32;

            self.proc_count_by_scenario.insert(scenario, proc_count);
            return proc_count;
        }

        let normal_proc_chance = scenario.modified_proc_chance(1.0).min(1.0);
        let normal_proc_count = normal_proc_chance * scenario.tap_count as f32;
        let echo_proc_count = if scenario.has_astral_echo {
            let echo_tap_count = scenario.tap_count / 5;
            let echo_proc_chance = if scenario.base_proc_chance() >= 1.0 {
                1.0
            } else {
                scenario
                    .modified_proc_chance(scenario.bonus_tap_proc_chance_mult())
                    .min(1.0)
            };

            echo_proc_chance * echo_tap_count as f32
        } else {
            0.0
        };

        let proc_count = normal_proc_count + echo_proc_count;
        self.proc_count_by_scenario.insert(scenario, proc_count);
        proc_count
    }

    pub fn fast_calc_burst_proc_chances(cards: &[Card], boss: &Boss) -> Vec<(u16, bool)> {
        let mut proc_chances = Vec::new();

        for card in cards.iter().filter(|card| {
            card.cardtype == CardType::Burst && FAST_CALC_CARDS.contains(&card.card_id)
        }) {
            let proc_chance = if card.card_id == CardName::CosmicHaymaker {
                (COSMIC_HAYMAKER_FAST_PROC_KEY, true)
            } else {
                (
                    proc_chance_to_basis_points(card.get_proc_chance(boss) as f32),
                    false,
                )
            };

            if !proc_chances.contains(&proc_chance) {
                proc_chances.push(proc_chance);
            }
        }

        proc_chances.sort_unstable();
        proc_chances
    }

    pub fn get_proc_count(&self, scenario: ProcScenario) -> Option<f32> {
        self.proc_count_by_scenario.get(&scenario).copied()
    }

    pub fn print_all(&self) {
        let mut proc_counts = self.proc_count_by_scenario.iter().collect::<Vec<_>>();
        proc_counts.sort_by(|(left_scenario, _), (right_scenario, _)| {
            left_scenario.name().cmp(&right_scenario.name())
        });

        println!("[PROC CACHE] total scenarios {}", proc_counts.len());
        for (scenario, proc_count) in proc_counts {
            println!("[PROC CACHE] {} => {:.2}", scenario.name(), proc_count);
        }
    }
}

fn proc_chance_to_basis_points(proc_chance: f32) -> u16 {
    (proc_chance.clamp(0.0, 1.0) * 10_000.0).round() as u16
}

fn mult_to_basis_points(mult: f32) -> u16 {
    (mult.max(0.0) * 10_000.0).round().min(u16::MAX as f32) as u16
}

fn sim_worker_count(work_items: usize) -> usize {
    if !ENABLE_PARALLEL_SIM || work_items <= 1 {
        return 1;
    }

    let available = std::thread::available_parallelism()
        .map(|count| count.get())
        .unwrap_or(1);
    let requested = if SIM_WORKER_COUNT == 0 {
        available
    } else {
        SIM_WORKER_COUNT
    };

    requested.clamp(1, work_items)
}

fn split_deck_pattern_work(
    deck_patterns: Vec<DeckPatternWork>,
    worker_count: usize,
) -> Vec<Vec<IndexedDeckPatternWork>> {
    let mut chunks = (0..worker_count).map(|_| Vec::new()).collect::<Vec<_>>();
    let mut loads = vec![0usize; worker_count];
    let mut indexed_work = deck_patterns
        .into_iter()
        .enumerate()
        .map(|(index, (deck, attack_patterns))| {
            let pattern_count = attack_patterns.len();
            (index, deck, attack_patterns, pattern_count)
        })
        .collect::<Vec<_>>();

    indexed_work.sort_by(|left, right| right.3.cmp(&left.3));

    for (index, deck, attack_patterns, pattern_count) in indexed_work {
        let worker_index = loads
            .iter()
            .enumerate()
            .min_by_key(|(_, load)| **load)
            .map(|(index, _)| index)
            .unwrap_or(0);

        loads[worker_index] += pattern_count;
        chunks[worker_index].push((index, deck, attack_patterns));
    }

    chunks
}

//release version 20R all cards 2m 1.56 sec
pub struct SimService;

impl SimService {
    pub fn is_fast_calc_deck(deck: &[Card]) -> bool {
        !deck.is_empty()
            && deck
                .iter()
                .all(|card| FAST_CALC_CARDS.contains(&card.card_id))
    }

    fn deck_tick_count(deck: &[Card], boss: &Boss) -> u32 {
        let support_modifiers = support_modifiers_for_deck(deck, boss);
        let base_duration_seconds = TICKS_PER_ROUND as f64 / TICKS_PER_SECOND;
        let duration_seconds =
            (base_duration_seconds + support_modifiers.attack_duration_add_seconds).max(0.0);

        (duration_seconds * TICKS_PER_SECOND).round() as u32
    }

    pub fn run_simulation(payload: SimPayLoad) -> SimRunResult {
        let sim_stats = SimStats {
            player_stat: Arc::new(payload.player_raid_data),
            boss_stat: payload.boss_data,
            attackable_part: payload.attackable_part,
            usable_card: payload.usable_card,
        };

        let valid_decks = generate_deck(&sim_stats);
        let deck_patterns = valid_decks
            .into_iter()
            .filter_map(|deck| {
                let attack_patterns = generate_attack_patterns(&sim_stats, &deck);
                if attack_patterns.is_empty() {
                    None
                } else {
                    Some((deck, attack_patterns))
                }
            })
            .collect::<Vec<_>>();
        let progress = SimProgress {
            current_pattern: AtomicUsize::new(0),
            total_patterns: deck_patterns
                .iter()
                .map(|(_, attack_patterns)| attack_patterns.len())
                .sum(),
        };

        let mut card_proc_cache = PreDeterminedProc::new();
        for (deck, _) in &deck_patterns {
            let tap_count = Self::deck_tick_count(deck, &sim_stats.boss_stat);
            card_proc_cache.generate_proc_count(deck, &sim_stats.boss_stat, tap_count);
        }
        if PRINT_PROC_CACHE {
            card_proc_cache.print_all();
        }

        let fast_calc_deck_count = deck_patterns
            .iter()
            .filter(|(deck, _)| Self::is_fast_calc_deck(deck))
            .count();
        let fast_calc_deck_percent = if deck_patterns.is_empty() {
            0.0
        } else {
            fast_calc_deck_count as f64 / deck_patterns.len() as f64 * 100.0
        };
        println!(
            "[SIMs] fast calc decks {}/{} ({:.2}%)",
            fast_calc_deck_count,
            deck_patterns.len(),
            fast_calc_deck_percent
        );

        let worker_count = sim_worker_count(deck_patterns.len());

        if PRINT_SIM_PATTERN_PROGRESS {
            println!(
                "[SIMs] start | decks {} | patterns {} | rounds {} | ticks {} | workers {}",
                deck_patterns.len(),
                progress.total_patterns,
                SIMS_ROUNDS,
                TICKS_PER_ROUND,
                worker_count
            );
        };

        let decks =
            Self::run_deck_pattern_work(&sim_stats, deck_patterns, &card_proc_cache, &progress);

        return SimRunResult {
            total_decks: decks.len(),
            total_attack_patterns: progress.total_patterns,
            rounds_per_pattern: SIMS_ROUNDS,
            ticks_per_round: TICKS_PER_ROUND,
            decks,
        };
    }

    fn run_deck_pattern_work(
        sim_stats: &SimStats,
        deck_patterns: Vec<DeckPatternWork>,
        proc_cache: &PreDeterminedProc,
        progress: &SimProgress,
    ) -> Vec<SimDeckResult> {
        let worker_count = sim_worker_count(deck_patterns.len());

        if worker_count <= 1 {
            return deck_patterns
                .into_iter()
                .map(|(deck, attack_patterns)| {
                    Self::run_single_deck_pattern_work(
                        sim_stats,
                        deck,
                        attack_patterns,
                        proc_cache,
                        Some(progress),
                    )
                })
                .collect();
        }

        let chunks = split_deck_pattern_work(deck_patterns, worker_count);
        let mut indexed_results = std::thread::scope(|scope| {
            let mut handles = Vec::with_capacity(chunks.len());

            for chunk in chunks.into_iter().filter(|chunk| !chunk.is_empty()) {
                handles.push(scope.spawn(move || {
                    let mut results = Vec::with_capacity(chunk.len());

                    for (index, deck, attack_patterns) in chunk {
                        let result = Self::run_single_deck_pattern_work(
                            sim_stats,
                            deck,
                            attack_patterns,
                            proc_cache,
                            Some(progress),
                        );
                        results.push((index, result));
                    }

                    results
                }));
            }

            let mut results = Vec::new();
            for handle in handles {
                results.extend(handle.join().expect("sim worker panicked"));
            }

            results
        });

        indexed_results.sort_by_key(|(index, _)| *index);
        indexed_results
            .into_iter()
            .map(|(_, result)| result)
            .collect()
    }

    fn run_single_deck_pattern_work(
        sim_stats: &SimStats,
        deck: Vec<Card>,
        attack_patterns: Vec<AttackPattern>,
        proc_cache: &PreDeterminedProc,
        progress: Option<&SimProgress>,
    ) -> SimDeckResult {
        if Self::is_fast_calc_deck(&deck) {
            Self::run_fast_calc_deck_sim(sim_stats, deck, attack_patterns, proc_cache, progress)
        } else {
            Self::run_deck_sim(sim_stats, deck, attack_patterns, SIMS_ROUNDS, progress)
        }
    }

    pub fn run_deck_simulation(payload: SimPayLoad) -> Option<SimDeckResult> {
        let sim_stats = SimStats {
            player_stat: Arc::new(payload.player_raid_data),
            boss_stat: payload.boss_data,
            attackable_part: payload.attackable_part,
            usable_card: payload.usable_card,
        };

        let select_deck: Vec<Card> = sim_stats
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

        if select_deck.len() != 3 {
            return None;
        }

        let attack_patterns = generate_attack_patterns(&sim_stats, &select_deck);
        if attack_patterns.is_empty() {
            return None;
        }

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
        progress: Option<&SimProgress>,
    ) -> SimDeckResult {
        let mut select_deck = select_deck;
        prepare_deck_for_sim(&mut select_deck);
        cache_deck_proc_chances(&mut select_deck, &sim_stats.boss_stat);

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
        let tap_count = Self::deck_tick_count(&select_deck, &sim_stats.boss_stat);
        let should_update_boss = deck_creates_timed_boss_effect(&select_deck);
        let deck_card_names = select_deck
            .iter()
            .map(|card| card.card_id)
            .collect::<Vec<_>>();
        let mut pattern_results: Vec<SimPatternResult> = Vec::new();

        for (pattern_index, pattern) in attack_patterns.into_iter().enumerate() {
            let pattern_name = pattern.describe();
            let mut total_sim_damage: u64 = 0;
            let mut lowest_round_damage = u64::MAX;
            let mut highest_round_damage = 0;
            let mut card_damage_totals = vec![0u64; select_deck.len()];
            let mut card_proc_totals = vec![0u64; select_deck.len()];

            for _ in 1..=sim_rounds {
                let mut boss = sim_stats.boss_stat.clone();
                boss.set_player_raid_data(Arc::clone(&sim_stats.player_stat));
                boss.prepare_card_damage_tracking(&deck_card_names);
                let damage_context = SimDamageContext::new(&sim_stats.player_stat, &boss);
                let mut total_burst_proc: u32 = 0;
                let mut deck = select_deck.clone();
                let totem_card = deck
                    .iter()
                    .find(|card| card.card_id == CardName::TotemOfPower)
                    .cloned();
                let mut support_cache = RoundSupportCache::new(&mut deck, &mut boss);
                let mut pending_totems: Vec<PendingTotem> = Vec::new();
                let mut next_totem_spawn_tick = totem_card
                    .as_ref()
                    .map(totem_of_power::first_spawn_tick)
                    .unwrap_or(f64::INFINITY);
                let mut last_target: Option<BossPartName> = None;
                let prepared_pattern = pattern.prepare(&boss, &deck, &sim_stats.attackable_part);

                for i in 0..tap_count {
                    if let Some(current_target) = prepared_pattern.next_target(
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
                            &damage_context,
                            &mut total_burst_proc,
                            1.0,
                            &mut card_proc_totals,
                            &mut support_cache,
                        );

                        if trigger_astral_echo_extra_tap(&mut deck) {
                            let astral_proc_chance_scale =
                                support_cache.bonus_tap_proc_chance_mult();
                            Self::tap_boss(
                                &mut boss,
                                current_target,
                                &mut deck,
                                &damage_context,
                                &mut total_burst_proc,
                                astral_proc_chance_scale,
                                &mut card_proc_totals,
                                &mut support_cache,
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

                    if should_update_boss {
                        boss.update();
                    }
                }

                let round_damage = boss.get_total_damage();
                total_sim_damage += round_damage;
                lowest_round_damage = lowest_round_damage.min(round_damage);
                highest_round_damage = highest_round_damage.max(round_damage);

                for (card_index, card_name) in deck_card_names.iter().enumerate() {
                    let damage = boss.card_damage_total(*card_name);
                    if damage > 0 {
                        card_damage_totals[card_index] =
                            card_damage_totals[card_index].saturating_add(damage);
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
                .enumerate()
                .map(|(card_index, card)| {
                    let average_damage = card_damage_totals[card_index] / sim_rounds;
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

            let (current_progress, total_progress) =
                advance_sim_progress(progress, pattern_index, total_attack_patterns);

            if should_print_sim_pattern_progress(current_progress, total_progress) {
                let card_summary = select_deck
                    .iter()
                    .enumerate()
                    .map(|(card_index, card)| {
                        let average_damage = card_damage_totals[card_index] / sim_rounds;
                        let average_proc_count =
                            format_average_count(card_proc_totals[card_index], sim_rounds);

                        format!(
                            "{} dmg {} proc {}",
                            card_display_with_level(card),
                            format_compact(average_damage),
                            average_proc_count
                        )
                    })
                    .collect::<Vec<_>>()
                    .join(" | ");

                println!(
                    "[SIMs] {} | {} : avg {} l {} h {} | {}",
                    sim_progress_summary(current_progress, total_progress),
                    pattern_name,
                    format_compact(average_damage),
                    format_compact(lowest_round_damage),
                    format_compact(highest_round_damage),
                    card_summary
                );
            }
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
        damage_context: &SimDamageContext,
        total_burst_proc: &mut u32,
        proc_chance_scale: f64,
        card_proc_totals: &mut [u64],
        support_cache: &mut RoundSupportCache,
    ) {
        if boss.get_state_from_part(attack_part) == PartState::Skeleton {
            return;
        }

        let current_state = boss.get_state_from_part(attack_part);
        let true_base_tap = damage_context.true_base_tap(attack_part, current_state);

        let combined_support = support_cache.current(deck, boss);

        // card proc
        for (card_index, card) in deck.iter_mut().enumerate() {
            if !matches!(card.cardtype, CardType::Burst | CardType::Affliction) {
                continue;
            }

            let card_type_add_total = damage_context.card_type_add(card.cardtype);
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
            let card_proc_chance = if card_has_dynamic_proc_chance(card.card_id) {
                card.get_proc_chance(boss)
            } else {
                card.proc_chance_cache
            };
            let proc_chance = if proc_chance_scale < 1.0 && card_proc_chance >= 1.0 {
                1.0
            } else {
                card_proc_chance * chance_mult * proc_chance_scale
            };
            // println!("Proc Chance {} {} {} {} {}",proc_chance,card_proc_chance,chance_mult, proc_chance_scale, combined_support.burst_chance_mult);
            let roll: f64 = random(); // Assuming random() yields an f64 from rand crate
            if roll <= proc_chance {
                let counts_as_card_proc = card_roll_counts_as_proc(card, boss, attack_part);

                if counts_as_card_proc {
                    if card.cardtype == CardType::Burst {
                        *total_burst_proc += 1;
                    }
                    if let Some(total) = card_proc_totals.get_mut(card_index) {
                        *total = total.saturating_add(1);
                    }
                }

                card.on_proc(boss, attack_part, card_base_damage, 0, *total_burst_proc);
            }
        }

        // tap damage on boss
        let tap_damage = true_base_tap as u64;
        boss.on_hit_with_source(attack_part, tap_damage, DamageSource::Tap);
    }
    pub fn run_fast_calc_deck_sim(
        sim_stats: &SimStats,
        select_deck: Vec<Card>,
        attack_patterns: Vec<AttackPattern>,
        proc_cache: &PreDeterminedProc,
        progress: Option<&SimProgress>,
    ) -> SimDeckResult {
        let mut select_deck = select_deck;
        prepare_deck_for_sim(&mut select_deck);

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

        let mut pattern_results = Vec::new();

        for (pattern_index, pattern) in attack_patterns.into_iter().enumerate() {
            let pattern_name = pattern.describe();
            let mut boss = sim_stats.boss_stat.clone();
            boss.set_player_raid_data(Arc::clone(&sim_stats.player_stat));

            let damage_context = SimDamageContext::new(&sim_stats.player_stat, &boss);
            let mut support_deck = select_deck.clone();
            let support = combined_support_modifiers(&mut support_deck, &boss);
            boss.set_support_modifiers(support.clone());

            let target_parts =
                pattern.fast_calc_target_parts(&boss, &select_deck, &sim_stats.attackable_part);
            let target_tap_counts =
                Self::fast_math_target_tap_counts(&pattern, &target_parts, &select_deck, &boss);
            let total_target_taps = target_tap_counts
                .iter()
                .map(|(_, tap_count)| *tap_count)
                .sum::<u32>();

            let mut total_damage = 0u64;
            let mut card_damage_totals: HashMap<CardName, u64> = HashMap::new();
            let mut card_proc_totals: HashMap<CardName, f32> = HashMap::new();

            for (target_part, tap_count) in &target_tap_counts {
                let current_state = boss.get_state_from_part(*target_part);
                let tap_damage = damage_context.true_base_tap(*target_part, current_state) as u64;
                let final_tap_damage =
                    boss.preview_damage_with_source(*target_part, tap_damage, &DamageSource::Tap);

                total_damage =
                    total_damage.saturating_add(final_tap_damage.saturating_mul(*tap_count as u64));
            }

            if total_target_taps > 0 {
                for card in select_deck
                    .iter()
                    .filter(|card| card.cardtype == CardType::Burst)
                {
                    let proc_count = Self::fast_proc_count_for_card(
                        card,
                        &boss,
                        &select_deck,
                        &support,
                        proc_cache,
                    );

                    if proc_count <= 0.0 {
                        continue;
                    }

                    card_proc_totals.insert(card.card_id, proc_count);

                    for (target_part, tap_count) in &target_tap_counts {
                        let target_proc_count =
                            proc_count * (*tap_count as f32 / total_target_taps as f32);

                        if target_proc_count <= 0.0 {
                            continue;
                        }

                        let current_state = boss.get_state_from_part(*target_part);
                        let true_base_tap =
                            damage_context.true_base_tap(*target_part, current_state);
                        let card_base_damage =
                            (true_base_tap + damage_context.card_type_add(card.cardtype)) as f64;
                        let final_damage_per_proc = Self::fast_card_final_damage_per_proc(
                            card,
                            &boss,
                            *target_part,
                            card_base_damage,
                        );
                        let card_damage =
                            (final_damage_per_proc as f32 * target_proc_count).max(0.0) as u64;

                        total_damage = total_damage.saturating_add(card_damage);
                        *card_damage_totals.entry(card.card_id).or_insert(0) += card_damage;
                    }
                }
            }

            let card_damage = select_deck
                .iter()
                .map(|card| {
                    let average_damage =
                        card_damage_totals.get(&card.card_id).copied().unwrap_or(0);
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
                average_damage: total_damage,
                average_damage_display: format_compact(total_damage),
                lowest_round_damage: total_damage,
                lowest_round_damage_display: format_compact(total_damage),
                highest_round_damage: total_damage,
                highest_round_damage_display: format_compact(total_damage),
                card_damage,
            });

            let (current_progress, total_progress) =
                advance_sim_progress(progress, pattern_index, total_attack_patterns);

            if should_print_sim_pattern_progress(current_progress, total_progress) {
                let card_summary = select_deck
                    .iter()
                    .map(|card| {
                        let average_damage =
                            card_damage_totals.get(&card.card_id).copied().unwrap_or(0);
                        let average_proc_count =
                            card_proc_totals.get(&card.card_id).copied().unwrap_or(0.0);

                        format!(
                            "{} dmg {} proc {}",
                            card_display_with_level(card),
                            format_compact(average_damage),
                            format_float_count(average_proc_count)
                        )
                    })
                    .collect::<Vec<_>>()
                    .join(" | ");

                println!(
                    "[SIMs] {} | {} : avg {} l {} h {} | {}",
                    sim_progress_summary(current_progress, total_progress),
                    pattern_name,
                    format_compact(total_damage),
                    format_compact(total_damage),
                    format_compact(total_damage),
                    card_summary
                );
            }
        }

        pattern_results.sort_by(|a, b| b.average_damage.cmp(&a.average_damage));

        SimDeckResult {
            deck,
            deck_names,
            total_attack_patterns,
            best_pattern: pattern_results.into_iter().next(),
        }
    }

    fn fast_math_target_tap_counts(
        pattern: &AttackPattern,
        target_parts: &[BossPartName],
        deck: &[Card],
        boss: &Boss,
    ) -> Vec<(BossPartName, u32)> {
        if target_parts.is_empty() {
            return Vec::new();
        }

        let total_taps = Self::fast_total_taps(deck, boss);

        if fast_calc_pattern_is_single_target(pattern) {
            return vec![(target_parts[0], total_taps)];
        }

        let target_count = target_parts.len() as u32;
        let base_taps = total_taps / target_count;
        let extra_taps = total_taps % target_count;

        target_parts
            .iter()
            .enumerate()
            .map(|(index, part)| {
                let tap_count = base_taps + u32::from((index as u32) < extra_taps);
                (*part, tap_count)
            })
            .filter(|(_, tap_count)| *tap_count > 0)
            .collect()
    }

    fn fast_total_taps(deck: &[Card], boss: &Boss) -> u32 {
        let base_taps = Self::deck_tick_count(deck, boss);
        let echo_taps = if deck.iter().any(|card| card.card_id == CardName::AstralEcho) {
            base_taps / 5
        } else {
            0
        };

        base_taps + echo_taps
    }

    fn fast_proc_count_for_card(
        card: &Card,
        boss: &Boss,
        deck: &[Card],
        support_modifiers: &SupportModifiers,
        proc_cache: &PreDeterminedProc,
    ) -> f32 {
        let proc_chance_basis_points = if card.card_id == CardName::CosmicHaymaker {
            COSMIC_HAYMAKER_FAST_PROC_KEY
        } else {
            proc_chance_to_basis_points(card.get_proc_chance(boss) as f32)
        };
        let scenario = ProcScenario {
            proc_chance_basis_points,
            is_cosmic_haymaker: card.card_id == CardName::CosmicHaymaker,
            proc_chance_mult_basis_points: mult_to_basis_points(
                support_modifiers.burst_chance_mult as f32,
            ),
            has_astral_echo: deck.iter().any(|card| card.card_id == CardName::AstralEcho),
            bonus_tap_proc_chance_mult_basis_points: if deck
                .iter()
                .any(|card| card.card_id == CardName::AstralEcho)
            {
                mult_to_basis_points(support_modifiers.bonus_tap_proc_chance_mult as f32)
            } else {
                mult_to_basis_points(1.0)
            },
            tap_count: Self::deck_tick_count(deck, boss),
        };

        proc_cache.get_proc_count(scenario).unwrap_or(0.0)
    }

    fn fast_card_final_damage_per_proc(
        card: &Card,
        boss: &Boss,
        target_part: BossPartName,
        card_base_damage: f64,
    ) -> u64 {
        let mut scratch_boss = boss.clone();
        let mut scratch_card = card.clone();

        if scratch_card.card_id == CardName::CosmicHaymaker {
            scratch_card.tap_count = COSMIC_HAYMAKER_TAPS_PER_PROC.saturating_sub(1);
        }

        let raw_damage =
            scratch_card.on_proc(&mut scratch_boss, target_part, card_base_damage, 0, 0);
        let source = DamageSource::Card(card.card_id);
        let mut final_damage = boss.preview_damage_with_source(target_part, raw_damage, &source);

        if card.card_id == CardName::FlakShot
            && matches!(
                boss.get_state_from_part(target_part),
                PartState::Armor | PartState::Cursed
            )
        {
            if let Some(body_part) = boss
                .parts()
                .iter()
                .find(|part| part.part_state == PartState::Body)
                .map(|part| part.part_name)
            {
                final_damage = final_damage.saturating_add(
                    boss.preview_damage_with_source(body_part, raw_damage, &source),
                );
            }
        }

        final_damage
    }
}

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
        if is_deck_synergistic(sim_stats, c1, c2, c3)
            && is_deck_boss_suitable(sim_stats, c1, c2, c3)
        {
            // Dereference the pointers to store clean Card values
            let deck = vec![c1.clone(), c2.clone(), c3.clone()];
            deck_combinations.push(deck);
        }
    }

    deck_combinations
}

const IS_CHECK_CARD_SYNERGY: bool = false;
const PURIFY_PRIORITY_AFFLICTIONS: [CardName; 6] = [
    CardName::AcidDrench,
    CardName::RavenousSwarm,
    CardName::RuinousRain,
    CardName::Amplify,
    CardName::ElectroZap,
    CardName::BlazingInferno,
];

fn is_purify_priority_affliction(card_name: CardName) -> bool {
    PURIFY_PRIORITY_AFFLICTIONS.contains(&card_name)
}

fn is_deck_synergistic(sim_stats: &SimStats, c1: &Card, c2: &Card, c3: &Card) -> bool {
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

    let has_ancestral_favor = deck.iter().any(|c| c.card_id == CardName::AncestralFavor);

    let has_rancid_gas = deck.iter().any(|c| c.card_id == CardName::RancidGas);

    let has_sands_of_time = deck.iter().any(|c| c.card_id == CardName::SandsOfTime);

    let has_whip = deck.iter().any(|c| c.card_id == CardName::WhipOfLightning);

    let has_celestial_static = deck.iter().any(|c| c.card_id == CardName::CelestialStatic);
    let has_grasping_vines = deck.iter().any(|c| c.card_id == CardName::GraspingVines);
    let has_totem_of_power = deck.iter().any(|c| c.card_id == CardName::TotemOfPower);
    let has_corrosive_bubble = deck.iter().any(|c| c.card_id == CardName::CorrosiveBubbles);
    let has_ravenous_swarm = deck.iter().any(|c| c.card_id == CardName::RavenousSwarm);
    let has_ruinous_rain = deck.iter().any(|c| c.card_id == CardName::RuinousRain);

    let has_fusion_bomb = deck.iter().any(|c| c.card_id == CardName::FusionBomb);
    let has_soul_fire = deck.iter().any(|c| c.card_id == CardName::SoulFire);
    let has_crushing_instinct = deck.iter().any(|c| c.card_id == CardName::CrushingInstinct);

    let has_blazing_inferno = deck.iter().any(|c| c.card_id == CardName::BlazingInferno);
    let has_amplify = deck.iter().any(|c| c.card_id == CardName::Amplify);
    let has_grim_shadow = deck.iter().any(|c| c.card_id == CardName::GrimShadow);
    let has_decaying_strike = deck.iter().any(|c| c.card_id == CardName::DecayingStrike);
    let has_radioactivity = deck.iter().any(|c| c.card_id == CardName::Radioactivity);
    let has_thriving_plague = deck.iter().any(|c| c.card_id == CardName::ThrivingPlague);
    let has_electro_zap = deck.iter().any(|c| c.card_id == CardName::ElectroZap);
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

    //Rule 4 Burst support must use with burst card or other support card
    if has_ancestral_favor {
        if burst_count < 1 {
            return false;
        }
        if affliction_count == 1 && !has_maelstrom {
            return false;
        }
    }
    if IS_CHECK_CARD_SYNERGY {
        println!("Rule 4 PASS")
    }

    //Rule 5 Affliction support must use with burst card or other support card
    if has_rancid_gas {
        if affliction_count < 1 {
            return false;
        }
        if burst_count == 1 && !has_guard_break {
            return false;
        }
    }
    if IS_CHECK_CARD_SYNERGY {
        println!("Rule 5 PASS")
    }

    //Rule 6 never 3 support card
    if support_count == 3 {
        return false;
    }
    if IS_CHECK_CARD_SYNERGY {
        println!("Rule 6 PASS")
    }

    // //Rule 7 : Sand of Time card must use with another debuff inflict card
    if has_sands_of_time {
        if affliction_count <= 1 {
            return false;
        }
        if has_maelstrom && affliction_count == 2 {
            return false;
        }
    }
    if IS_CHECK_CARD_SYNERGY {
        println!("Rule 7 PASS")
    }

    //rule 8 : celestial card not suit with limb support card
    if has_celestial_static {
        if has_grasping_vines || has_totem_of_power {
            return false;
        }
    }
    if IS_CHECK_CARD_SYNERGY {
        println!("Rule 8 PASS")
    }

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
        if has_electro_zap {
            return false;
        }
    }

    if IS_CHECK_CARD_SYNERGY {
        println!("Rule 10 PASS")
    }

    //rule 11
    //some affliction should not use with sot
    if has_sands_of_time {
        if has_corrosive_bubble || has_ravenous_swarm || has_ruinous_rain || has_totem_of_power {
            return false;
        }
    }

    //rule 12
    if has_fusion_bomb {
        if has_totem_of_power || has_soul_fire || has_crushing_instinct {
            return false;
        }
    }

    //rule 14
    //2 support cards must intersect some boss part
    if has_soul_fire || has_crushing_instinct {
        if has_grasping_vines {
            return false;
        }
    }
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

fn format_float_count(count: f32) -> String {
    if (count.fract()).abs() < f32::EPSILON {
        format!("{:.0}", count)
    } else {
        format!("{:.2}", count)
    }
}

fn fast_calc_pattern_is_single_target(pattern: &AttackPattern) -> bool {
    matches!(
        pattern,
        AttackPattern::SingleAny
            | AttackPattern::SingleHead
            | AttackPattern::SingleTorso
            | AttackPattern::SingleBody
            | AttackPattern::SingleArmor
            | AttackPattern::SingleLimb
            | AttackPattern::SingleCursed
    )
}

fn should_print_sim_pattern_progress(current_pattern: usize, total_patterns: usize) -> bool {
    if !PRINT_SIM_PATTERN_PROGRESS || total_patterns == 0 {
        return false;
    }

    if PRINT_EVERY_SIM_PATTERN {
        return true;
    }

    if SIM_PATTERN_PROGRESS_STEP_PERCENT == 0 {
        return false;
    }

    let previous_bucket = (current_pattern.saturating_sub(1) * 100)
        / total_patterns
        / SIM_PATTERN_PROGRESS_STEP_PERCENT;
    let current_bucket =
        (current_pattern * 100) / total_patterns / SIM_PATTERN_PROGRESS_STEP_PERCENT;

    current_pattern == total_patterns || current_bucket > previous_bucket
}

fn advance_sim_progress(
    progress: Option<&SimProgress>,
    pattern_index: usize,
    total_attack_patterns: usize,
) -> (usize, usize) {
    if let Some(progress) = progress {
        let current_pattern = progress
            .current_pattern
            .fetch_add(1, AtomicOrdering::Relaxed)
            + 1;
        return (current_pattern, progress.total_patterns);
    }

    (pattern_index + 1, total_attack_patterns)
}

fn sim_progress_summary(current_pattern: usize, total_patterns: usize) -> String {
    let percent = if total_patterns == 0 {
        100.0
    } else {
        (current_pattern as f64 / total_patterns as f64) * 100.0
    };

    format!("{}/{} ({:.2}%)", current_pattern, total_patterns, percent)
}

fn card_display_with_level(card: &Card) -> String {
    format!("{}({})", card.card_id.display_name(), card.level)
}

fn card_roll_counts_as_proc(card: &Card, boss: &Boss, attack_part: BossPartName) -> bool {
    match card.cardtype {
        CardType::Affliction => true,
        CardType::Burst => burst_roll_counts_as_proc(card, boss, attack_part),
        CardType::Support => false,
    }
}

fn burst_roll_counts_as_proc(card: &Card, boss: &Boss, attack_part: BossPartName) -> bool {
    match card.card_id {
        CardName::CosmicHaymaker => {
            card.tap_count.saturating_add(1) >= COSMIC_HAYMAKER_TAPS_PER_PROC
        }
        CardName::CelestialStatic => {
            !attack_part.is_limb()
                && boss.get_state_from_part(attack_part) != PartState::Skeleton
                && card.celestial_stacks >= CELESTIAL_STATIC_STACKS_PER_PROC
        }
        _ => true,
    }
}

fn prepare_deck_for_sim(deck: &mut [Card]) {
    ensure_deck_card_skills(deck);
    if apply_amplify_level_sharing(deck) {
        ensure_deck_card_skills(deck);
    }
    apply_global_raid_card_modifiers(deck);
}

fn apply_global_raid_card_modifiers(deck: &mut [Card]) {
    let global = global_raid_modifiers();

    if (global.affliction_duration_mult - 1.0).abs() <= f64::EPSILON {
        return;
    }

    for card in deck
        .iter_mut()
        .filter(|card| card.cardtype == CardType::Affliction)
    {
        card.skill.duration *= global.affliction_duration_mult;
    }
}

fn apply_amplify_level_sharing(deck: &mut [Card]) -> bool {
    let Some((amplify_level, share_rate)) = deck
        .iter()
        .find(|card| card.card_id == CardName::Amplify)
        .map(|card| (card.level, card.skill.bonus_c.unwrap_or(0.1)))
    else {
        return false;
    };

    let shared_levels = (amplify_level as f64 * share_rate).ceil().max(1.0) as u16;
    let mut changed = false;

    for card in deck
        .iter_mut()
        .filter(|card| card.card_id != CardName::Amplify)
    {
        let max_level = card.skill.max_level;
        let boosted_level = card.level.saturating_add(shared_levels).min(max_level);
        if boosted_level != card.level {
            card.level = boosted_level;
            changed = true;
        }
    }

    changed
}

fn ensure_deck_card_skills(deck: &mut [Card]) {
    for card in deck {
        card.ensure_skill_cache();
    }
}

fn cache_deck_proc_chances(deck: &mut [Card], boss: &Boss) {
    for card in deck
        .iter_mut()
        .filter(|card| matches!(card.cardtype, CardType::Burst | CardType::Affliction))
    {
        card.proc_chance_cache = card.get_proc_chance(boss);
    }
}

fn card_has_dynamic_proc_chance(card_name: CardName) -> bool {
    matches!(
        card_name,
        CardName::BlazingInferno | CardName::WhipOfLightning
    )
}

fn trigger_astral_echo_extra_tap(deck: &mut [Card]) -> bool {
    let Some(astral_echo) = deck
        .iter_mut()
        .find(|card| card.card_id == CardName::AstralEcho)
    else {
        return false;
    };

    let max_charges = astral_echo.skill.bonus_c.unwrap_or(5.0).max(1.0) as u16;
    astral_echo.tap_count = astral_echo.tap_count.saturating_add(1);

    if astral_echo.tap_count < max_charges {
        return false;
    }

    astral_echo.tap_count = 0;
    true
}

fn support_modifiers_for_deck(deck: &[Card], boss: &Boss) -> SupportModifiers {
    let mut deck = deck.to_vec();
    combined_support_modifiers(&mut deck, boss)
}

fn combined_support_modifiers(deck: &mut [Card], boss: &Boss) -> SupportModifiers {
    let deck_snapshot = deck.to_vec();
    let global = global_raid_modifiers();
    let mut support = SupportModifiers::default();

    for card in deck
        .iter_mut()
        .filter(|card| card.cardtype == CardType::Support)
    {
        let modifier = card
            .support_modifiers(boss, &deck_snapshot)
            .scale_effects(global.support_effect_mult);
        support.merge(&modifier);
    }

    support.burst_damage_mult *= global.burst_damage_mult;
    support.burst_chance_mult *= global.burst_chance_mult;
    support.affliction_chance_mult *= global.affliction_chance_mult;
    support.affliction_damage_mult *= global.affliction_damage_mult;
    support.all_damage_mult *= global.all_damage_mult;
    support.attack_duration_add_seconds += global.attack_duration_add_seconds;

    support
}

fn deck_has_dynamic_support_modifier(deck: &[Card]) -> bool {
    deck.iter().any(|card| {
        matches!(
            card.card_id,
            CardName::InsanityVoid | CardName::SkeletalSmash | CardName::VictoryMarch
        )
    })
}

fn deck_creates_timed_boss_effect(deck: &[Card]) -> bool {
    deck.iter().any(|card| {
        matches!(card.cardtype, CardType::Affliction)
            || matches!(card.card_id, CardName::GuardBreak | CardName::TotemOfPower)
    })
}
