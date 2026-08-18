use super::*;

pub(super) fn format_compact(damage: u64) -> String {
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

pub(super) fn format_average_count(total_count: u64, rounds: u64) -> String {
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

pub(super) fn format_float_count(count: f32) -> String {
    if (count.fract()).abs() < f32::EPSILON {
        format!("{:.0}", count)
    } else {
        format!("{:.2}", count)
    }
}

pub(super) fn fast_calc_pattern_is_single_target(pattern: &AttackPattern) -> bool {
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

pub(super) fn should_print_sim_pattern_progress(
    current_pattern: usize,
    total_patterns: usize,
) -> bool {
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

pub(super) fn advance_sim_progress(
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

pub(super) fn sim_progress_summary(current_pattern: usize, total_patterns: usize) -> String {
    let percent = if total_patterns == 0 {
        100.0
    } else {
        (current_pattern as f64 / total_patterns as f64) * 100.0
    };

    format!("{}/{} ({:.2}%)", current_pattern, total_patterns, percent)
}

pub(super) fn card_display_with_level(card: &Card) -> String {
    format!("{}({})", card.card_id.display_name(), card.level)
}

pub(super) fn card_roll_counts_as_proc(
    card: &Card,
    boss: &Boss,
    attack_part: BossPartName,
) -> bool {
    match card.cardtype {
        CardType::Affliction => true,
        CardType::Burst => burst_roll_counts_as_proc(card, boss, attack_part),
        CardType::Support => false,
    }
}

pub(super) fn burst_roll_counts_as_proc(
    card: &Card,
    boss: &Boss,
    attack_part: BossPartName,
) -> bool {
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

pub(super) fn prepare_deck_for_sim(deck: &mut [Card], boss: &Boss) {
    ensure_deck_card_skills(deck);
    if apply_amplify_level_sharing(deck) {
        ensure_deck_card_skills(deck);
    }
    apply_global_raid_card_modifiers(
        deck,
        boss.global_raid_modifier,
        boss.global_raid_modifier_amount,
    );
}

pub(super) fn apply_global_raid_card_modifiers(
    deck: &mut [Card],
    selected: GlobalRaidModifier,
    amount: Option<f64>,
) {
    let global = global_raid_modifiers(selected, amount);

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

pub(super) fn apply_amplify_level_sharing(deck: &mut [Card]) -> bool {
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

pub(super) fn ensure_deck_card_skills(deck: &mut [Card]) {
    for card in deck {
        card.ensure_skill_cache();
    }
}

pub(super) fn cache_deck_proc_chances(deck: &mut [Card], boss: &Boss) {
    for card in deck
        .iter_mut()
        .filter(|card| matches!(card.cardtype, CardType::Burst | CardType::Affliction))
    {
        card.proc_chance_cache = card.get_proc_chance(boss);
    }
}

pub(super) fn card_has_dynamic_proc_chance(card_name: CardName) -> bool {
    matches!(
        card_name,
        CardName::BlazingInferno | CardName::WhipOfLightning
    )
}

pub(super) fn trigger_astral_echo_extra_tap(deck: &mut [Card]) -> bool {
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

pub(super) fn support_modifiers_for_deck(deck: &[Card], boss: &Boss) -> SupportModifiers {
    let mut deck = deck.to_vec();
    combined_support_modifiers(&mut deck, boss)
}

pub(super) fn combined_support_modifiers(deck: &mut [Card], boss: &Boss) -> SupportModifiers {
    let deck_snapshot = deck.to_vec();
    let global = global_raid_modifiers(boss.global_raid_modifier, boss.global_raid_modifier_amount);
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

pub(super) fn deck_has_dynamic_support_modifier(deck: &[Card]) -> bool {
    deck.iter().any(|card| {
        matches!(
            card.card_id,
            CardName::InsanityVoid | CardName::SkeletalSmash | CardName::VictoryMarch
        )
    })
}

pub(super) fn deck_dependency_part_mask(
    sim_stats: &SimStats,
    deck: &[Card],
    attack_patterns: &[AttackPattern],
) -> u8 {
    dependency_part_mask_for(
        &sim_stats.boss_stat,
        &sim_stats.attackable_part,
        deck,
        attack_patterns,
    )
}

fn dependency_part_mask_for(
    boss: &Boss,
    attackable_parts: &[BossPartName],
    deck: &[Card],
    attack_patterns: &[AttackPattern],
) -> u8 {
    if deck.iter().any(|card| {
        matches!(
            card.card_id,
            CardName::BarbedMorningstar
                | CardName::FlakShot
                | CardName::InsanityVoid
                | CardName::RavenousSwarm
                | CardName::SkeletalSmash
                | CardName::VictoryMarch
        )
    }) {
        return u8::MAX;
    }

    attack_patterns.iter().fold(0u8, |mask, pattern| {
        pattern
            .fast_calc_target_parts(boss, deck, attackable_parts)
            .into_iter()
            .fold(mask, |mask, part| mask | part.dependency_mask())
    })
}

#[cfg(test)]
mod dependency_mask_tests {
    use serde_json::json;

    use super::*;

    fn part(name: &str) -> serde_json::Value {
        json!({
            "part_name": name,
            "part_state": "Body",
            "max_armor": 100,
            "max_health": 100,
            "current_armor": 0,
            "current_health": 100
        })
    }

    fn boss() -> Boss {
        serde_json::from_value(json!({
            "boss_name": "Jukk",
            "head": part("Head"),
            "torso": part("Torso"),
            "left_shoulder": part("LeftShoulder"),
            "right_shoulder": part("RightShoulder"),
            "left_hand": part("LeftHand"),
            "right_hand": part("RightHand"),
            "left_leg": part("LeftLeg"),
            "right_leg": part("RightLeg")
        }))
        .unwrap()
    }

    fn card(id: &str, cardtype: &str) -> Card {
        serde_json::from_value(json!({
            "card_id": id,
            "cardtype": cardtype,
            "level": 1
        }))
        .unwrap()
    }

    #[test]
    fn direct_patterns_mark_only_their_candidate_parts() {
        let mask = dependency_part_mask_for(
            &boss(),
            &[BossPartName::Head, BossPartName::Torso],
            &[card("MoonBeam", "Burst")],
            &[AttackPattern::SingleHead],
        );
        assert_eq!(mask, BossPartName::Head.dependency_mask());
    }

    #[test]
    fn global_state_cards_depend_on_every_part() {
        let mask = dependency_part_mask_for(
            &boss(),
            &[BossPartName::Head],
            &[card("FinisherAttack", "Support")],
            &[AttackPattern::SingleHead],
        );
        assert_eq!(mask, u8::MAX);
    }
}

pub(super) fn deck_creates_timed_boss_effect(deck: &[Card]) -> bool {
    deck.iter().any(|card| {
        matches!(card.cardtype, CardType::Affliction)
            || matches!(card.card_id, CardName::GuardBreak | CardName::TotemOfPower)
    })
}
