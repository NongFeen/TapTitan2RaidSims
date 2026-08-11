use super::*;

impl SimService {
    pub fn run_fast_calc_deck_sim(
        sim_stats: &SimStats,
        select_deck: Vec<Card>,
        attack_patterns: Vec<AttackPattern>,
        proc_cache: &PreDeterminedProc,
        progress: Option<&SimProgress>,
    ) -> SimDeckResult {
        let mut select_deck = select_deck;
        prepare_deck_for_sim(&mut select_deck, &sim_stats.boss_stat);

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
                patterns: Vec::new(),
            };
        }

        let mut best_pattern: Option<SimPatternResult> = None;

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

            let mut total_damage = 0.0f64;
            let mut card_damage_totals: HashMap<CardName, f64> = HashMap::new();
            let mut card_proc_totals: HashMap<CardName, f32> = HashMap::new();

            for (target_part, tap_count) in &target_tap_counts {
                let current_state = boss.get_state_from_part(*target_part);
                let tap_damage = damage_context.true_base_tap(*target_part, current_state) as f64;
                let final_tap_damage =
                    boss.preview_damage_with_source(*target_part, tap_damage, &DamageSource::Tap);

                total_damage += final_tap_damage * *tap_count as f64;
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
                            (final_damage_per_proc * target_proc_count as f64).max(0.0);

                        total_damage += card_damage;
                        *card_damage_totals.entry(card.card_id).or_insert(0.0) += card_damage;
                    }
                }
            }

            let card_damage = select_deck
                .iter()
                .map(|card| {
                    let average_damage = card_damage_totals
                        .get(&card.card_id)
                        .copied()
                        .unwrap_or(0.0)
                        .max(0.0) as u64;
                    SimCardDamageResult {
                        card: card.card_id,
                        card_name: card.card_id.display_name().to_string(),
                        average_damage,
                        average_damage_display: format_compact(average_damage),
                    }
                })
                .collect();

            let total_damage = total_damage.max(0.0) as u64;
            let pattern_result = SimPatternResult {
                pattern: pattern_name.clone(),
                average_damage: total_damage,
                average_damage_display: format_compact(total_damage),
                lowest_round_damage: total_damage,
                lowest_round_damage_display: format_compact(total_damage),
                highest_round_damage: total_damage,
                highest_round_damage_display: format_compact(total_damage),
                card_damage,
            };

            if best_pattern.as_ref().map_or(true, |best| {
                pattern_result.average_damage > best.average_damage
            }) {
                best_pattern = Some(pattern_result);
            }

            let (current_progress, total_progress) =
                advance_sim_progress(progress, pattern_index, total_attack_patterns);

            if should_print_sim_pattern_progress(current_progress, total_progress) {
                let card_summary = select_deck
                    .iter()
                    .map(|card| {
                        let average_damage = card_damage_totals
                            .get(&card.card_id)
                            .copied()
                            .unwrap_or(0.0)
                            .max(0.0) as u64;
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

        SimDeckResult {
            deck,
            deck_names,
            total_attack_patterns,
            best_pattern,
            patterns: Vec::new(),
        }
    }

    pub(super) fn fast_math_target_tap_counts(
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

    pub(super) fn fast_total_taps(deck: &[Card], boss: &Boss) -> u32 {
        let base_taps = Self::deck_tick_count(deck, boss);
        let echo_taps = if deck.iter().any(|card| card.card_id == CardName::AstralEcho) {
            base_taps / 5
        } else {
            0
        };

        base_taps + echo_taps
    }

    pub(super) fn fast_proc_count_for_card(
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

    pub(super) fn fast_card_final_damage_per_proc(
        card: &Card,
        boss: &Boss,
        target_part: BossPartName,
        card_base_damage: f64,
    ) -> f64 {
        let mut scratch_boss = boss.clone();
        let mut scratch_card = card.clone();

        if scratch_card.card_id == CardName::CosmicHaymaker {
            scratch_card.tap_count = COSMIC_HAYMAKER_TAPS_PER_PROC.saturating_sub(1);
        }

        // Fast-calculation cards do not consume randomness. A fixed seed keeps
        // that contract explicit while sharing the normal proc dispatch API.
        let mut rng = rand::rngs::StdRng::seed_from_u64(0);
        let raw_damage = scratch_card.on_proc(
            &mut scratch_boss,
            target_part,
            card_base_damage,
            0,
            0,
            &mut rng,
        );
        boss.preview_damage_with_source(target_part, raw_damage, &DamageSource::Card(card.card_id))
    }
}
