use super::*;

pub fn generate_attack_patterns(sim_stats: &SimStats, deck: &[Card]) -> Vec<AttackPattern> {
    generate_attack_patterns_with_mode(sim_stats, deck, false)
}

pub fn generate_all_attack_patterns(sim_stats: &SimStats, deck: &[Card]) -> Vec<AttackPattern> {
    generate_attack_patterns_with_mode(sim_stats, deck, true)
}

fn generate_attack_patterns_with_mode(
    sim_stats: &SimStats,
    deck: &[Card],
    include_all_valid_patterns: bool,
) -> Vec<AttackPattern> {
    if deck_is_target_insensitive(deck) {
        let pattern = AttackPattern::SingleAny;
        let info = AttackPatternInfo::new(pattern, sim_stats, deck);
        return if pattern_has_candidates(&info) {
            vec![pattern]
        } else {
            Vec::new()
        };
    }

    let mut pattern_infos = Vec::new();

    for pattern in base_attack_patterns().iter().copied() {
        if !pattern_is_available_for_deck(&pattern, deck) {
            continue;
        }
        if !pattern_passes_static_deck_rules(&pattern, deck) {
            continue;
        }

        let info = AttackPatternInfo::new(pattern, sim_stats, deck);
        if !pattern_has_candidates(&info) || !pattern_passes_candidate_deck_rules(&info, deck) {
            continue;
        }

        pattern_infos.push(info);
    }

    dedupe_generic_attack_patterns(&mut pattern_infos);
    if !include_all_valid_patterns {
        if sim_stats.boss_stat.recommend_1_to_2_part_patterns_only {
            pattern_infos.retain(|info| (1..=2).contains(&attacked_part_count(info)));
        }
        keep_max_coverage_spread_affliction_patterns(deck, &mut pattern_infos);
        keep_top_attack_patterns(&mut pattern_infos);
    }

    let patterns = pattern_infos
        .into_iter()
        .map(|info| info.pattern)
        .collect::<Vec<_>>();

    patterns
}

pub(super) fn base_attack_patterns() -> &'static [AttackPattern] {
    BASE_ATTACK_PATTERNS
}

pub(super) fn pattern_has_candidates(info: &AttackPatternInfo) -> bool {
    if let AttackPattern::CycleParts(count) = info.pattern {
        return count > 1 && count < info.source_count;
    }

    !info.candidates.is_empty()
}

pub(super) fn pattern_is_available_for_deck(pattern: &AttackPattern, deck: &[Card]) -> bool {
    match pattern {
        AttackPattern::SingleCursed | AttackPattern::CycleCursed => {
            deck_has_card(deck, CardName::Fragmentize) || deck_has_card(deck, CardName::RuinousRain)
        }
        AttackPattern::FusionBombSpread => deck_has_card(deck, CardName::FusionBomb),
        AttackPattern::ThrivingPlagueSpread => deck_has_card(deck, CardName::ThrivingPlague),
        AttackPattern::RadioactivitySpread => deck_has_card(deck, CardName::Radioactivity),
        AttackPattern::DecayingStrikeFocus => deck_has_card(deck, CardName::DecayingStrike),
        AttackPattern::BlazingInfernoStack => deck_has_card(deck, CardName::BlazingInferno),
        AttackPattern::CelestialStatic => deck_has_card(deck, CardName::CelestialStatic),
        AttackPattern::WhipRuinousFocus => {
            deck_has_card(deck, CardName::WhipOfLightning)
                && deck_has_card(deck, CardName::RuinousRain)
        }
        _ => true,
    }
}

/// Candidate count is the number cycled across for multi-target patterns. A
/// single-target pattern chooses one candidate and stays there, even when it
/// has several possible candidates.
pub(super) fn attacked_part_count(info: &AttackPatternInfo) -> usize {
    if pattern_is_single_target(&info.pattern) {
        usize::from(!info.candidates.is_empty())
    } else {
        info.candidates.len()
    }
}

pub(super) fn deck_is_target_insensitive(deck: &[Card]) -> bool {
    deck.iter()
        .all(|card| card_is_target_insensitive(card.card_id))
}

pub(super) fn card_is_target_insensitive(card_name: CardName) -> bool {
    matches!(
        card_name,
        CardName::ClanshipBarrage
            | CardName::PurifyingBlast
            | CardName::CosmicHaymaker
            | CardName::MirrorForce
            | CardName::GuardBreak
            | CardName::ElectroZap
            | CardName::InsanityVoid
            | CardName::RancidGas
            | CardName::VictoryMarch
            | CardName::AncestralFavor
            | CardName::TeamTactics
            | CardName::AstralEcho
            | CardName::RadiantKaleidoscope
            | CardName::BattleDrums
    )
}

pub(super) fn deck_has_target_insensitive_card(deck: &[Card]) -> bool {
    deck.iter()
        .any(|card| card_is_target_insensitive(card.card_id))
}

pub(super) fn deck_has_single_pattern_rejected_affliction(deck: &[Card]) -> bool {
    deck.iter().any(|card| {
        card.card_id.card_type() == crate::models::cards::CardType::Affliction
            && !matches!(
                card.card_id,
                CardName::CorrosiveBubbles
                    | CardName::RuinousRain
                    | CardName::ElectroZap
                    | CardName::Maelstrom
            )
    })
}

pub(super) fn dedupe_generic_attack_patterns(patterns: &mut Vec<AttackPatternInfo>) {
    let mut seen_signatures: Vec<(u8, Vec<BossPartName>)> = Vec::new();

    patterns.retain(|info| {
        if pattern_is_card_specific(&info.pattern) {
            return true;
        }

        let Some(signature) = generic_pattern_signature(info) else {
            return true;
        };

        if seen_signatures.contains(&signature) {
            return false;
        }

        seen_signatures.push(signature);
        true
    });
}

pub(super) fn keep_max_coverage_spread_affliction_patterns(
    deck: &[Card],
    patterns: &mut Vec<AttackPatternInfo>,
) {
    if !deck_has_spread_affliction(deck) || patterns.len() <= 1 {
        return;
    }

    let max_candidate_count = patterns
        .iter()
        .map(|info| info.candidates.len())
        .max()
        .unwrap_or(0);

    if max_candidate_count == 0 {
        return;
    }

    patterns.retain(|info| info.candidates.len() == max_candidate_count);
}

pub(super) fn deck_has_spread_affliction(deck: &[Card]) -> bool {
    deck.iter().any(|card| {
        matches!(
            card.card_id,
            CardName::FusionBomb
                | CardName::ThrivingPlague
                | CardName::BlazingInferno
                | CardName::Amplify
        )
    })
}

pub(super) fn keep_top_attack_patterns(patterns: &mut Vec<AttackPatternInfo>) {
    if MAX_ATTACK_PATTERNS_PER_DECK == 0 || patterns.len() <= MAX_ATTACK_PATTERNS_PER_DECK {
        return;
    }

    let mut ranked_patterns = patterns
        .drain(..)
        .enumerate()
        .map(|(index, info)| (index, info))
        .collect::<Vec<_>>();

    ranked_patterns.sort_by(|(left_index, left_info), (right_index, right_info)| {
        right_info
            .priority
            .cmp(&left_info.priority)
            .then_with(|| left_index.cmp(right_index))
    });

    patterns.extend(
        ranked_patterns
            .into_iter()
            .take(MAX_ATTACK_PATTERNS_PER_DECK)
            .map(|(_, info)| info),
    );
}

pub(super) fn attack_pattern_priority_from_parts(
    pattern: &AttackPattern,
    sim_stats: &SimStats,
    deck: &[Card],
    candidates: &[BossPartName],
    source_count: usize,
) -> (i32, usize, usize) {
    let candidate_count = candidates.len();

    let mut score = 0;

    if pattern_is_card_specific(pattern) {
        score += 20_000;
    }

    if deck_wants_wide_attack(deck) {
        score += candidate_count as i32 * 1_000;
        if pattern_is_cycle_target(pattern) {
            score += 750;
        }
        if pattern_is_single_target(pattern) {
            score -= 1_000;
        }
    } else {
        score += candidate_count as i32 * 250;
    }

    score += support_pattern_fit_score(pattern, deck);
    score += burst_target_fit_score(pattern, deck, &sim_stats.boss_stat, &candidates);
    score += pattern_shape_score(pattern, candidate_count, source_count);

    (score, candidate_count, source_count)
}

pub(super) fn deck_wants_wide_attack(deck: &[Card]) -> bool {
    deck.iter().any(|card| {
        card.cardtype == CardType::Affliction
            && !matches!(
                card.card_id,
                CardName::CorrosiveBubbles
                    | CardName::RuinousRain
                    | CardName::ElectroZap
                    | CardName::Maelstrom
            )
    })
}

pub(super) fn support_pattern_fit_score(pattern: &AttackPattern, deck: &[Card]) -> i32 {
    let mut score = 0;

    for card in deck {
        match card.card_id {
            CardName::GraspingVines => {
                if matches!(
                    pattern,
                    AttackPattern::SingleLimb | AttackPattern::CycleLimb
                ) {
                    score += 1_500;
                }
            }
            CardName::InspiringForce => {
                if matches!(
                    pattern,
                    AttackPattern::SingleBody | AttackPattern::CycleBody
                ) {
                    score += 1_500;
                }
            }
            CardName::PrismaticRift | CardName::SkeletalSmash => {
                if matches!(
                    pattern,
                    AttackPattern::SingleArmor | AttackPattern::CycleArmor
                ) {
                    score += 1_500;
                }
            }
            CardName::SoulFire | CardName::CrushingInstinct => {
                if matches!(
                    pattern,
                    AttackPattern::SingleHead
                        | AttackPattern::SingleTorso
                        | AttackPattern::CycleHeadTorso
                ) {
                    score += 1_500;
                }
            }
            _ => {}
        }
    }

    score
}

pub(super) fn burst_target_fit_score(
    pattern: &AttackPattern,
    deck: &[Card],
    boss: &Boss,
    candidates: &[BossPartName],
) -> i32 {
    if candidates.is_empty() {
        return 0;
    }

    let mut score = 0;

    for card in deck {
        match card.card_id {
            CardName::MoonBeam => {
                score += matching_target_score(candidates, |part| {
                    matches!(
                        part,
                        BossPartName::Torso
                            | BossPartName::LeftShoulder
                            | BossPartName::RightShoulder
                            | BossPartName::LeftHand
                            | BossPartName::RightHand
                    )
                });
            }
            CardName::SkullBash => {
                score += matching_target_score(candidates, |part| {
                    matches!(
                        part,
                        BossPartName::Head | BossPartName::LeftLeg | BossPartName::RightLeg
                    )
                });
            }
            CardName::Fragmentize => {
                score += matching_state_score(candidates, boss, |state| {
                    matches!(state, PartState::Armor | PartState::Cursed)
                });
                score +=
                    matching_state_score(candidates, boss, |state| state == PartState::Cursed) / 2;
            }
            CardName::RazorWind => {
                score += matching_state_score(candidates, boss, |state| state == PartState::Body);
            }
            CardName::PsychicShackles => {
                score += matching_target_score(candidates, |part| part.is_limb());
            }
            CardName::FlakShot => {
                if boss
                    .parts()
                    .iter()
                    .any(|part| part.part_state == PartState::Body)
                {
                    score += matching_state_score(candidates, boss, |state| {
                        matches!(state, PartState::Armor | PartState::Cursed)
                    });
                }
            }
            CardName::BarbedMorningstar => {
                score += barbed_morningstar_target_score(card, boss, candidates);
            }
            CardName::ChainOfVengeance => {
                if pattern_is_cycle_target(pattern) {
                    score += (candidates.len().min(6) as i32) * 300;
                }
            }
            _ => {}
        }
    }

    score
}

pub(super) fn matching_target_score(
    candidates: &[BossPartName],
    predicate: impl Fn(BossPartName) -> bool,
) -> i32 {
    let matching_count = candidates
        .iter()
        .copied()
        .filter(|part| predicate(*part))
        .count();

    ratio_score(matching_count, candidates.len(), 2_500)
}

pub(super) fn matching_state_score(
    candidates: &[BossPartName],
    boss: &Boss,
    predicate: impl Fn(PartState) -> bool,
) -> i32 {
    let matching_count = candidates
        .iter()
        .copied()
        .filter(|part| predicate(boss.part(*part).part_state))
        .count();

    ratio_score(matching_count, candidates.len(), 2_500)
}

pub(super) fn ratio_score(matching_count: usize, total_count: usize, max_score: i32) -> i32 {
    if matching_count == 0 || total_count == 0 {
        return 0;
    }

    ((matching_count as f64 / total_count as f64) * max_score as f64).round() as i32
        + matching_count as i32 * 50
}

pub(super) fn barbed_morningstar_target_score(
    card: &Card,
    boss: &Boss,
    candidates: &[BossPartName],
) -> i32 {
    let armor_damage_boost = card.skill.bonus_c.unwrap_or(0.0);
    let body_damage_boost = card.skill.bonus_d.unwrap_or(0.0);
    let max_bonus_parts = card
        .skill
        .bonus_e
        .map(|value| value.max(0.0) as usize)
        .unwrap_or(5);
    let armor_part_count = boss
        .parts()
        .iter()
        .filter(|part| matches!(part.part_state, PartState::Armor | PartState::Cursed))
        .count()
        .min(max_bonus_parts);
    let body_part_count = boss
        .parts()
        .iter()
        .filter(|part| part.part_state == PartState::Body)
        .count()
        .min(max_bonus_parts);

    let total_bonus = candidates
        .iter()
        .copied()
        .map(|part| match boss.part(part).part_state {
            PartState::Armor | PartState::Cursed => armor_damage_boost * body_part_count as f64,
            PartState::Body => body_damage_boost * armor_part_count as f64,
            PartState::Skeleton => 0.0,
        })
        .sum::<f64>();

    ((total_bonus / candidates.len() as f64) * 2_000.0).round() as i32
}

pub(super) fn pattern_shape_score(
    pattern: &AttackPattern,
    candidate_count: usize,
    source_count: usize,
) -> i32 {
    match pattern {
        AttackPattern::CelestialStatic | AttackPattern::WhipRuinousFocus => 4_000,
        AttackPattern::FusionBombSpread
        | AttackPattern::ThrivingPlagueSpread
        | AttackPattern::RadioactivitySpread
        | AttackPattern::BlazingInfernoStack => 3_000,
        AttackPattern::DecayingStrikeFocus => 2_000,
        AttackPattern::CycleAllActive => 1_000,
        AttackPattern::CycleParts(count) if *count == source_count.saturating_sub(1) => 900,
        AttackPattern::CycleParts(count) => 500 + (*count as i32 * 20),
        AttackPattern::CycleHeadTorso
        | AttackPattern::CycleLimb
        | AttackPattern::CycleBody
        | AttackPattern::CycleArmor
        | AttackPattern::CycleCursed => 650 + candidate_count as i32 * 20,
        AttackPattern::SingleAny => 100,
        AttackPattern::SingleHead
        | AttackPattern::SingleTorso
        | AttackPattern::SingleBody
        | AttackPattern::SingleArmor
        | AttackPattern::SingleLimb
        | AttackPattern::SingleCursed => 250,
    }
}

pub(super) fn generic_pattern_signature(
    info: &AttackPatternInfo,
) -> Option<(u8, Vec<BossPartName>)> {
    if info.candidates.is_empty() {
        return None;
    }

    let mode = if pattern_is_single_target(&info.pattern) || info.candidates.len() <= 1 {
        0
    } else if pattern_is_cycle_target(&info.pattern) {
        1
    } else {
        return None;
    };

    Some((mode, info.candidates.clone()))
}

pub(super) fn pattern_passes_static_deck_rules(pattern: &AttackPattern, deck: &[Card]) -> bool {
    if deck_has_card(deck, CardName::TotemOfPower) && pattern_is_cycle_target(pattern) {
        return false;
    }
    // remove single pattern spread deck contain insensitive target card
    if pattern_is_single_target(pattern)
        && deck_has_target_insensitive_card(deck)
        && deck_has_single_pattern_rejected_affliction(deck)
    {
        return false;
    }

    true
}

pub(super) fn pattern_passes_candidate_deck_rules(info: &AttackPatternInfo, deck: &[Card]) -> bool {
    if deck_has_card(deck, CardName::FusionBomb) && info.candidates.len() < 3 {
        return false;
    }

    true
}

pub(super) fn pattern_is_cycle_target(pattern: &AttackPattern) -> bool {
    matches!(
        pattern,
        AttackPattern::CycleCursed
            | AttackPattern::CycleHeadTorso
            | AttackPattern::CycleLimb
            | AttackPattern::CycleBody
            | AttackPattern::CycleArmor
            | AttackPattern::CycleAllActive
            | AttackPattern::CycleParts(_)
            | AttackPattern::BlazingInfernoStack
            | AttackPattern::FusionBombSpread
            | AttackPattern::RadioactivitySpread
            | AttackPattern::ThrivingPlagueSpread
    )
}

pub(super) fn pattern_is_card_specific(pattern: &AttackPattern) -> bool {
    matches!(
        pattern,
        AttackPattern::FusionBombSpread
            | AttackPattern::ThrivingPlagueSpread
            | AttackPattern::RadioactivitySpread
            | AttackPattern::DecayingStrikeFocus
            | AttackPattern::BlazingInfernoStack
            | AttackPattern::CelestialStatic
            | AttackPattern::WhipRuinousFocus
    )
}

pub(super) fn pattern_is_single_target(pattern: &AttackPattern) -> bool {
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

pub(super) fn deck_has_card(deck: &[Card], card_name: CardName) -> bool {
    deck.iter().any(|card| card.card_id == card_name)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn card(card_id: CardName) -> Card {
        Card {
            card_id,
            cardtype: card_id.card_type(),
            level: 1,
            enabled: true,
            tap_count: 0,
            chained_parts: Vec::new(),
            celestial_stacks: 0,
            skill: Default::default(),
            proc_chance_cache: 0.0,
        }
    }

    #[test]
    fn cursed_patterns_require_a_curse_target_card() {
        let ordinary_deck = [card(CardName::MoonBeam)];
        let fragmentize_deck = [card(CardName::Fragmentize)];
        let ruinous_rain_deck = [card(CardName::RuinousRain)];

        for pattern in [AttackPattern::SingleCursed, AttackPattern::CycleCursed] {
            assert!(!pattern_is_available_for_deck(&pattern, &ordinary_deck));
            assert!(pattern_is_available_for_deck(&pattern, &fragmentize_deck));
            assert!(pattern_is_available_for_deck(&pattern, &ruinous_rain_deck));
        }
    }

    #[test]
    fn narrow_filter_counts_single_patterns_as_one_attacked_part() {
        let candidates = vec![BossPartName::Head, BossPartName::Torso];
        let single = AttackPatternInfo {
            pattern: AttackPattern::SingleAny,
            candidates: candidates.clone(),
            source_count: candidates.len(),
            priority: (0, 0, 0),
        };
        let cycle = AttackPatternInfo {
            pattern: AttackPattern::CycleAllActive,
            candidates,
            source_count: 2,
            priority: (0, 0, 0),
        };

        assert_eq!(attacked_part_count(&single), 1);
        assert_eq!(attacked_part_count(&cycle), 2);
    }
}
