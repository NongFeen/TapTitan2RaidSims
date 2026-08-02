use super::*;

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

    pub(super) fn base_proc_chance(&self) -> f32 {
        self.proc_chance_basis_points as f32 / 10_000.0
    }

    pub(super) fn modified_proc_chance(&self, proc_chance_scale: f32) -> f32 {
        self.base_proc_chance() * self.proc_chance_mult() * proc_chance_scale
    }

    pub(super) fn proc_chance_mult(&self) -> f32 {
        self.proc_chance_mult_basis_points as f32 / 10_000.0
    }

    pub(super) fn bonus_tap_proc_chance_mult(&self) -> f32 {
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

pub(super) fn proc_chance_to_basis_points(proc_chance: f32) -> u16 {
    (proc_chance.clamp(0.0, 1.0) * 10_000.0).round() as u16
}

pub(super) fn mult_to_basis_points(mult: f32) -> u16 {
    (mult.max(0.0) * 10_000.0).round().min(u16::MAX as f32) as u16
}

pub(super) fn sim_worker_count(work_items: usize) -> usize {
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

pub(super) fn split_deck_pattern_work(
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
