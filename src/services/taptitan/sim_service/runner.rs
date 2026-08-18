use super::*;

impl SimService {
    pub fn run_simulation_with_optional_body_phase(
        payload: SimPayLoad,
    ) -> (SimRunResult, Option<SimRunResult>) {
        let should_run_body_phase = should_run_targeted_body_phase(
            payload.include_body_phase,
            &payload.boss_data,
            &payload.attackable_part,
        );
        let current_result = Self::run_simulation(payload.clone());
        if !should_run_body_phase {
            return (current_result, None);
        }

        let mut body_payload = payload;
        convert_targeted_armor_to_body(&mut body_payload.boss_data, &body_payload.attackable_part);
        body_payload.include_body_phase = false;
        let mut body_result =
            Self::run_simulation_requiring_card(body_payload, CardName::InsanityVoid);
        for deck in &mut body_result.decks {
            deck.simulation_phase = SimulationPhase::TargetedBody;
        }

        let mut void_result = current_result.clone();
        void_result.total_attack_patterns = void_result
            .total_attack_patterns
            .saturating_add(body_result.total_attack_patterns);
        replace_required_card_deck_results(
            &mut void_result.decks,
            body_result.decks,
            CardName::InsanityVoid,
        );
        void_result.total_decks = void_result.decks.len();

        (current_result, Some(void_result))
    }

    pub fn is_fast_calc_deck(deck: &[Card]) -> bool {
        !deck.is_empty()
            && deck
                .iter()
                .all(|card| FAST_CALC_CARDS.contains(&card.card_id))
    }

    pub(super) fn deck_tick_count(deck: &[Card], boss: &Boss) -> u32 {
        let support_modifiers = support_modifiers_for_deck(deck, boss);
        let base_duration_seconds = TICKS_PER_ROUND as f64 / TICKS_PER_SECOND;
        let duration_seconds =
            (base_duration_seconds + support_modifiers.attack_duration_add_seconds).max(0.0);

        (duration_seconds * TICKS_PER_SECOND).round() as u32
    }

    pub fn run_simulation(payload: SimPayLoad) -> SimRunResult {
        Self::run_simulation_internal(payload, None)
    }

    fn run_simulation_requiring_card(payload: SimPayLoad, required_card: CardName) -> SimRunResult {
        Self::run_simulation_internal(payload, Some(required_card))
    }

    fn run_simulation_internal(
        mut payload: SimPayLoad,
        required_card: Option<CardName>,
    ) -> SimRunResult {
        payload.boss_data.snapshot_initial_curse_parts();
        let sim_stats = SimStats {
            player_stat: Arc::new(payload.player_raid_data),
            boss_stat: payload.boss_data,
            attackable_part: payload.attackable_part,
            usable_card: payload.usable_card,
            mirror_force_boost: payload.mirror_force_boost,
        };

        let valid_decks = generate_deck(&sim_stats)
            .into_iter()
            .filter(|deck| deck_matches_required_card(deck, required_card));
        let deck_patterns = valid_decks
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

    pub(super) fn run_deck_pattern_work(
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

    pub(super) fn run_single_deck_pattern_work(
        sim_stats: &SimStats,
        deck: Vec<Card>,
        attack_patterns: Vec<AttackPattern>,
        proc_cache: &PreDeterminedProc,
        progress: Option<&SimProgress>,
    ) -> SimDeckResult {
        if Self::is_fast_calc_deck(&deck) {
            Self::run_fast_calc_deck_sim(sim_stats, deck, attack_patterns, proc_cache, progress)
        } else {
            Self::run_deck_sim(
                sim_stats,
                deck,
                attack_patterns,
                SIMS_ROUNDS,
                progress,
                1,
                None,
                None,
                false,
            )
        }
    }

    pub fn run_deck_simulation(payload: SimPayLoad) -> Option<SimDeckResult> {
        Self::run_exact_deck_for_phase(payload, false, 10)
    }

    pub fn run_exact_deck_for_phase(
        mut payload: SimPayLoad,
        body_phase: bool,
        rounds: u64,
    ) -> Option<SimDeckResult> {
        if body_phase {
            if !should_run_targeted_body_phase(true, &payload.boss_data, &payload.attackable_part) {
                return None;
            }
            convert_targeted_armor_to_body(&mut payload.boss_data, &payload.attackable_part);
        }
        payload.boss_data.snapshot_initial_curse_parts();
        let sim_stats = SimStats {
            player_stat: Arc::new(payload.player_raid_data),
            boss_stat: payload.boss_data,
            attackable_part: payload.attackable_part,
            usable_card: payload.usable_card,
            mirror_force_boost: payload.mirror_force_boost,
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
                    .map(|mut card| {
                        crate::models::seasonal_card_boosts::apply_seasonal_level_boost(&mut card);
                        card.ensure_skill_cache();
                        card
                    })
            })
            .collect();

        if select_deck.len() != 3 {
            return None;
        }

        let attack_patterns = generate_attack_patterns(&sim_stats, &select_deck);
        if attack_patterns.is_empty() {
            return None;
        }

        let tap_count = Self::deck_tick_count(&select_deck, &sim_stats.boss_stat);
        let mut proc_cache = PreDeterminedProc::new();
        proc_cache.generate_proc_count(&select_deck, &sim_stats.boss_stat, tap_count);
        let mut result = if Self::is_fast_calc_deck(&select_deck) {
            Self::run_fast_calc_deck_sim(
                &sim_stats,
                select_deck,
                attack_patterns,
                &proc_cache,
                None,
            )
        } else {
            Self::run_deck_sim(
                &sim_stats,
                select_deck,
                attack_patterns,
                rounds,
                None,
                1,
                None,
                None,
                false,
            )
        };
        if body_phase {
            result.simulation_phase = SimulationPhase::TargetedBody;
        }
        Some(result)
    }

    pub fn run_deck_debug_simulation(
        mut payload: SimPayLoad,
        total_taps: u32,
        rounds_per_pattern: u64,
    ) -> Option<SimDeckResult> {
        payload.boss_data.snapshot_initial_curse_parts();
        let sim_stats = SimStats {
            player_stat: Arc::new(payload.player_raid_data),
            boss_stat: payload.boss_data,
            attackable_part: payload.attackable_part,
            usable_card: payload.usable_card,
            mirror_force_boost: payload.mirror_force_boost,
        };
        let select_deck = sim_stats
            .usable_card
            .iter()
            .filter_map(|card_name| {
                sim_stats
                    .player_stat
                    .card_list
                    .iter()
                    .find(|card| card.card_id == *card_name)
                    .cloned()
                    .map(|mut card| {
                        crate::models::seasonal_card_boosts::apply_seasonal_level_boost(&mut card);
                        card.ensure_skill_cache();
                        card
                    })
            })
            .collect::<Vec<_>>();
        if select_deck.len() != 3 {
            return None;
        }
        let attack_patterns = generate_all_attack_patterns(&sim_stats, &select_deck);
        if attack_patterns.is_empty() {
            return None;
        }

        Some(Self::run_deck_sim(
            &sim_stats,
            select_deck,
            attack_patterns,
            rounds_per_pattern,
            None,
            1,
            Some(TICKS_PER_ROUND),
            Some(total_taps),
            true,
        ))
    }

    pub fn run_deck_sim(
        sim_stats: &SimStats,
        select_deck: Vec<Card>,
        attack_patterns: Vec<AttackPattern>,
        round: u64,
        progress: Option<&SimProgress>,
        taps_per_tick: u32,
        fixed_tick_count: Option<u32>,
        total_tap_limit: Option<u32>,
        include_all_patterns: bool,
    ) -> SimDeckResult {
        let mut select_deck = select_deck;
        prepare_deck_for_sim(&mut select_deck, &sim_stats.boss_stat);
        cache_deck_proc_chances(&mut select_deck, &sim_stats.boss_stat);
        let dependency_part_mask =
            deck_dependency_part_mask(sim_stats, &select_deck, &attack_patterns);

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
                simulation_phase: SimulationPhase::Current,
                patterns: Vec::new(),
                dependency_part_mask,
            };
        }

        let sim_rounds = round;
        let tap_count = fixed_tick_count
            .unwrap_or_else(|| Self::deck_tick_count(&select_deck, &sim_stats.boss_stat));
        let should_update_boss = deck_creates_timed_boss_effect(&select_deck);
        let deck_card_names = select_deck
            .iter()
            .map(|card| card.card_id)
            .collect::<Vec<_>>();
        let mut best_pattern: Option<SimPatternResult> = None;
        let mut pattern_results = Vec::new();

        for (pattern_index, pattern) in attack_patterns.into_iter().enumerate() {
            let pattern_name = pattern.describe();
            let mut total_sim_damage: u64 = 0;
            let mut lowest_round_damage = u64::MAX;
            let mut highest_round_damage = 0;
            let mut card_damage_totals = vec![0u64; select_deck.len()];
            let mut card_proc_totals = vec![0u64; select_deck.len()];

            for _ in 1..=sim_rounds {
                // Keep one RNG for the whole round so the hot loop does not repeatedly
                // acquire the thread-local generator. Tests can pass a seeded RNG through
                // the same call chain.
                let mut rng = rand::rng();
                let mut boss = sim_stats.boss_stat.clone();
                boss.set_result_target_parts(&sim_stats.attackable_part);
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
                    let current_target = total_tap_limit
                        .is_none_or(|tap_limit| i < tap_limit)
                        .then(|| {
                            prepared_pattern.next_target(
                                &boss,
                                last_target,
                                &deck,
                                &sim_stats.attackable_part,
                            )
                        })
                        .flatten();

                    if let Some(current_target) = current_target {
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
                        for _ in 0..taps_per_tick {
                            Self::tap_boss(
                                &mut boss,
                                current_target,
                                &mut deck,
                                &damage_context,
                                &mut total_burst_proc,
                                1.0,
                                &mut card_proc_totals,
                                &mut support_cache,
                                sim_stats.mirror_force_boost,
                                &mut rng,
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
                                    sim_stats.mirror_force_boost,
                                    &mut rng,
                                );
                            }
                        }

                        if let Some(totem_card) = &totem_card {
                            totem_of_power::try_spawn(
                                &mut pending_totems,
                                totem_card,
                                &boss,
                                current_target,
                                i,
                                &mut next_totem_spawn_tick,
                                &mut rng,
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

            let pattern_result = SimPatternResult {
                pattern: pattern_name.clone(),
                average_damage,
                average_damage_display: format_compact(average_damage),
                lowest_round_damage,
                lowest_round_damage_display: format_compact(lowest_round_damage),
                highest_round_damage,
                highest_round_damage_display: format_compact(highest_round_damage),
                card_damage,
            };

            if best_pattern.as_ref().map_or(true, |best| {
                pattern_result.average_damage > best.average_damage
            }) {
                best_pattern = Some(pattern_result.clone());
            }
            if include_all_patterns {
                pattern_results.push(pattern_result);
            }

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

        SimDeckResult {
            deck,
            deck_names,
            total_attack_patterns,
            best_pattern,
            simulation_phase: SimulationPhase::Current,
            patterns: pattern_results,
            dependency_part_mask,
        }
    }
    pub(super) fn tap_boss(
        boss: &mut Boss,
        attack_part: BossPartName,
        deck: &mut [Card],
        damage_context: &SimDamageContext,
        total_burst_proc: &mut u32,
        proc_chance_scale: f64,
        card_proc_totals: &mut [u64],
        support_cache: &mut RoundSupportCache,
        mirror_force_boost: f64,
        rng: &mut impl Rng,
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
            let roll: f64 = rng.random();
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

                card.on_proc(
                    boss,
                    attack_part,
                    card_base_damage,
                    mirror_force_boost,
                    *total_burst_proc,
                    rng,
                );
            }
        }

        // tap damage on boss
        let tap_damage = true_base_tap as f64;
        boss.on_hit_with_source(attack_part, tap_damage, DamageSource::Tap);
    }
}

fn deck_matches_required_card(deck: &[Card], required_card: Option<CardName>) -> bool {
    required_card.is_none_or(|required| deck.iter().any(|card| card.card_id == required))
}

fn should_run_targeted_body_phase(
    include_body_phase: bool,
    boss: &Boss,
    attackable_parts: &[BossPartName],
) -> bool {
    include_body_phase
        && attackable_parts.iter().any(|part_name| {
            matches!(
                boss.get_state_from_part(*part_name),
                PartState::Armor | PartState::Cursed
            )
        })
}

fn convert_targeted_armor_to_body(boss: &mut Boss, attackable_parts: &[BossPartName]) {
    for part_name in attackable_parts {
        let part = boss.part_mut(*part_name);
        if matches!(part.part_state, PartState::Armor | PartState::Cursed) {
            part.part_state = PartState::Body;
            part.current_armor = 0;
        }
    }
}

fn replace_required_card_deck_results(
    current: &mut Vec<SimDeckResult>,
    body: Vec<SimDeckResult>,
    required_card: CardName,
) {
    current.retain(|result| !result.deck.contains(&required_card));
    current.extend(body);
}

#[cfg(test)]
mod body_phase_tests {
    use serde_json::json;

    use super::*;

    fn part(name: &str, state: &str, armor: u64, health: u64) -> serde_json::Value {
        json!({
            "part_name": name,
            "part_state": state,
            "max_armor": 100,
            "max_health": 200,
            "current_armor": armor,
            "current_health": health
        })
    }

    fn boss() -> Boss {
        serde_json::from_value(json!({
            "boss_name": "Jukk",
            "head": part("Head", "Cursed", 75, 180),
            "torso": part("Torso", "Armor", 50, 170),
            "left_shoulder": part("LeftShoulder", "Body", 0, 160),
            "right_shoulder": part("RightShoulder", "Body", 0, 150),
            "left_hand": part("LeftHand", "Body", 0, 140),
            "right_hand": part("RightHand", "Skeleton", 0, 0),
            "left_leg": part("LeftLeg", "Cursed", 25, 130),
            "right_leg": part("RightLeg", "Armor", 20, 120)
        }))
        .expect("test boss should deserialize")
    }

    fn pattern(damage: u64) -> SimPatternResult {
        SimPatternResult {
            pattern: "test".to_string(),
            average_damage: damage,
            average_damage_display: damage.to_string(),
            lowest_round_damage: damage,
            lowest_round_damage_display: damage.to_string(),
            highest_round_damage: damage,
            highest_round_damage_display: damage.to_string(),
            card_damage: Vec::new(),
        }
    }

    fn deck(cards: Vec<CardName>, damage: u64, phase: SimulationPhase) -> SimDeckResult {
        SimDeckResult {
            deck_names: cards
                .iter()
                .map(|card| card.display_name().to_string())
                .collect(),
            deck: cards,
            total_attack_patterns: 1,
            best_pattern: Some(pattern(damage)),
            simulation_phase: phase,
            patterns: Vec::new(),
            dependency_part_mask: 0,
        }
    }

    #[test]
    fn body_phase_deck_filter_only_accepts_insanity_void_decks() {
        let insanity_void: Card = serde_json::from_value(json!({
            "card_id": "CrushingVoid",
            "cardtype": "Support",
            "level": 1
        }))
        .expect("Insanity Void card should deserialize");
        let razor_wind: Card = serde_json::from_value(json!({
            "card_id": "RazorWind",
            "cardtype": "Burst",
            "level": 1
        }))
        .expect("Razor Wind card should deserialize");

        assert!(deck_matches_required_card(
            &[razor_wind.clone(), insanity_void],
            Some(CardName::InsanityVoid)
        ));
        assert!(!deck_matches_required_card(
            &[razor_wind],
            Some(CardName::InsanityVoid)
        ));
        assert!(deck_matches_required_card(&[], None));
    }

    #[test]
    fn body_phase_requires_enabled_and_convertible_target() {
        let boss = boss();
        let single_armor_target = [BossPartName::Head];
        assert!(!should_run_targeted_body_phase(
            false,
            &boss,
            &single_armor_target
        ));
        assert!(should_run_targeted_body_phase(
            true,
            &boss,
            &single_armor_target
        ));
        assert!(!should_run_targeted_body_phase(true, &boss, &[]));

        let body_target = [BossPartName::LeftShoulder];
        assert!(!should_run_targeted_body_phase(true, &boss, &body_target));
    }

    #[test]
    fn conversion_changes_only_targeted_armor_and_curse_parts() {
        let mut boss = boss();
        convert_targeted_armor_to_body(
            &mut boss,
            &[
                BossPartName::Head,
                BossPartName::Torso,
                BossPartName::LeftShoulder,
                BossPartName::RightHand,
                BossPartName::LeftLeg,
            ],
        );

        assert_eq!(boss.head.part_state, PartState::Body);
        assert_eq!(boss.head.current_armor, 0);
        assert_eq!(boss.head.current_health, 180);
        assert_eq!(boss.torso.part_state, PartState::Body);
        assert_eq!(boss.left_shoulder.current_health, 160);
        assert_eq!(boss.right_hand.part_state, PartState::Skeleton);
        assert_eq!(boss.right_leg.part_state, PartState::Armor);
        assert_eq!(boss.right_leg.current_armor, 20);
    }

    #[test]
    fn void_phase_replaces_matching_current_decks_even_when_damage_is_lower() {
        let cards = vec![
            CardName::RazorWind,
            CardName::AncestralFavor,
            CardName::InsanityVoid,
        ];
        let tied_cards = vec![
            CardName::MoonBeam,
            CardName::TeamTactics,
            CardName::AcidDrench,
        ];
        let mut current = vec![
            deck(cards.clone(), 100, SimulationPhase::Current),
            deck(tied_cards.clone(), 200, SimulationPhase::Current),
        ];
        let body = vec![deck(
            cards.into_iter().rev().collect(),
            90,
            SimulationPhase::TargetedBody,
        )];

        replace_required_card_deck_results(&mut current, body, CardName::InsanityVoid);

        assert_eq!(current.len(), 2);
        let void_deck = current
            .iter()
            .find(|result| result.deck.contains(&CardName::InsanityVoid))
            .unwrap();
        let non_void_deck = current
            .iter()
            .find(|result| !result.deck.contains(&CardName::InsanityVoid))
            .unwrap();
        assert_eq!(void_deck.simulation_phase, SimulationPhase::TargetedBody);
        assert_eq!(void_deck.best_pattern.as_ref().unwrap().average_damage, 90);
        assert_eq!(non_void_deck.simulation_phase, SimulationPhase::Current);
    }
}
