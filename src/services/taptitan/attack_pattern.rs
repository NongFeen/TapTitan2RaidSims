use crate::models::affliction::{Affliction, AfflictionKind};
use crate::models::boss::{Boss, BossPartName, PartState};
use crate::models::cards::{Card, CardName, CardType};
use std::cmp::Ordering;

use super::sim_service::SimStats;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AttackPattern {
    SingleAny,
    SingleHead,
    SingleTorso,
    SingleBody,
    SingleArmor,
    SingleLimb,
    SingleCursed,
    CycleCursed,
    CycleHeadTorso,
    CycleLimb,
    CycleBody,
    CycleArmor,
    CycleAllActive,
    CycleParts(usize),
    FusionBombSpread,
    ThrivingPlagueSpread,
    RadioactivitySpread,
    DecayingStrikeFocus,
    BlazingInfernoStack,
    CelestialStatic,
    WhipRuinousFocus,
}

pub struct PreparedAttackPattern {
    pattern: AttackPattern,
    target_plan: PreparedTargetPlan,
}

enum PreparedTargetPlan {
    Dynamic,
    Ordered(Vec<BossPartName>),
    FirstActiveParts {
        source_parts: Vec<BossPartName>,
        count: usize,
    },
}

const ALL_BOSS_PARTS: [BossPartName; 8] = [
    BossPartName::Head,
    BossPartName::Torso,
    BossPartName::LeftShoulder,
    BossPartName::RightShoulder,
    BossPartName::LeftHand,
    BossPartName::RightHand,
    BossPartName::LeftLeg,
    BossPartName::RightLeg,
];

#[allow(dead_code)]
const DEBUG_HEAD_TORSO_SUPPORT_PATTERNS: bool = true;
const MAX_ATTACK_PATTERNS_PER_DECK: usize = 3; // 0 = no cap

struct CandidateParts {
    parts: [BossPartName; 8],
    len: usize,
}

impl CandidateParts {
    fn new() -> Self {
        Self {
            parts: [BossPartName::Head; 8],
            len: 0,
        }
    }

    fn push(&mut self, part: BossPartName) {
        if self.len >= self.parts.len() {
            return;
        }

        self.parts[self.len] = part;
        self.len += 1;
    }

    fn as_slice(&self) -> &[BossPartName] {
        &self.parts[..self.len]
    }

    fn is_empty(&self) -> bool {
        self.len == 0
    }
}

impl PreparedAttackPattern {
    pub fn next_target(
        &self,
        boss: &Boss,
        last_target: Option<BossPartName>,
        deck: &[Card],
        attackable_parts: &[BossPartName],
    ) -> Option<BossPartName> {
        match &self.target_plan {
            PreparedTargetPlan::Dynamic => {
                self.pattern
                    .next_target(boss, last_target, deck, attackable_parts)
            }
            PreparedTargetPlan::Ordered(targets) => {
                self.pattern
                    .next_prepared_target(boss, targets, last_target)
            }
            PreparedTargetPlan::FirstActiveParts {
                source_parts,
                count,
            } => cycle_first_active_parts(boss, source_parts, *count, last_target),
        }
    }
}

impl AttackPattern {
    pub fn describe(&self) -> String {
        match self {
            AttackPattern::SingleAny => "SingleAny".to_string(),
            AttackPattern::SingleHead => "SingleHead".to_string(),
            AttackPattern::SingleTorso => "SingleTorso".to_string(),
            AttackPattern::SingleBody => "SingleBody".to_string(),
            AttackPattern::SingleArmor => "SingleArmor".to_string(),
            AttackPattern::SingleLimb => "SingleLimb".to_string(),
            AttackPattern::SingleCursed => "SingleCursed".to_string(),
            AttackPattern::CycleHeadTorso => "CycleHeadTorso".to_string(),
            AttackPattern::CycleCursed => "CycleCursed".to_string(),
            AttackPattern::CycleLimb => "CycleLimb".to_string(),
            AttackPattern::CycleArmor => "CycleArmor".to_string(),
            AttackPattern::CycleBody => "CycleBody".to_string(),
            AttackPattern::CycleAllActive => "CycleAllActive".to_string(),
            AttackPattern::CycleParts(count) => format!("CycleParts({})", count),
            AttackPattern::FusionBombSpread => "FusionBombSpread".to_string(),
            AttackPattern::ThrivingPlagueSpread => "ThrivingPlagueSpread".to_string(),
            AttackPattern::RadioactivitySpread => "RadioactivitySpread".to_string(),
            AttackPattern::DecayingStrikeFocus => "DecayingStrikeFocus".to_string(),
            AttackPattern::BlazingInfernoStack => "BlazingInfernoStack".to_string(),
            AttackPattern::CelestialStatic => "CelestialStatic".to_string(),
            AttackPattern::WhipRuinousFocus => "WhipRuinousFocus".to_string(),
        }
    }

    pub fn next_target(
        &self,
        boss: &Boss,
        last_target: Option<BossPartName>,
        deck: &[Card],
        attackable_parts: &[BossPartName],
    ) -> Option<BossPartName> {
        let candidates = self.candidate_parts_buffer(boss, deck, attackable_parts);
        if candidates.is_empty() {
            return None;
        }
        let candidates = candidates.as_slice();

        if let AttackPattern::WhipRuinousFocus = self {
            if let Some(last) = last_target {
                if let Some(index) = candidates.iter().position(|part| *part == last) {
                    return candidates
                        .get((index + 1) % candidates.len())
                        .copied()
                        .or_else(|| candidates.first().copied());
                }
            }

            return candidates.first().copied();
        }

        if let AttackPattern::FusionBombSpread = self {
            if let Some(open_part) = candidates.iter().copied().find(|part| {
                !boss
                    .afflictions(*part)
                    .iter()
                    .any(|aff| aff.kind == AfflictionKind::FusionBombDebuff)
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

        if let AttackPattern::ThrivingPlagueSpread = self {
            let mut best_plague_part: Option<(BossPartName, usize, f64)> = None;

            for part in candidates.iter().copied() {
                let Some(affliction) = boss
                    .afflictions(part)
                    .iter()
                    .find(|aff| aff.kind == AfflictionKind::ThrivingPlagueDebuff)
                else {
                    return Some(part);
                };

                let stack_count = affliction.stack_count();
                let lowest_remaining = lowest_stack_duration(affliction);

                if is_better_refresh_target(
                    lowest_remaining,
                    stack_count,
                    best_plague_part.map(|(_, best_stack_count, best_remaining)| {
                        (best_remaining, best_stack_count)
                    }),
                ) {
                    best_plague_part = Some((part, stack_count, lowest_remaining));
                }
            }

            return best_plague_part.map(|(target, _, _)| target);
        }

        if let AttackPattern::RadioactivitySpread = self {
            let disease_limit = 8usize;
            let mut diseased_count = 0usize;
            let mut first_missing_disease = None;
            let mut best_disease_part: Option<(BossPartName, usize, f64)> = None;

            for part in candidates.iter().copied() {
                let Some(affliction) = boss
                    .afflictions(part)
                    .iter()
                    .find(|aff| aff.kind == AfflictionKind::RadioactivityDebuff)
                else {
                    if first_missing_disease.is_none() {
                        first_missing_disease = Some(part);
                    }
                    continue;
                };

                diseased_count += 1;
                let stack_count = affliction.stack_count();
                let lowest_remaining = lowest_stack_duration(affliction);

                if is_better_refresh_target(
                    lowest_remaining,
                    stack_count,
                    best_disease_part.map(|(_, best_stack_count, best_remaining)| {
                        (best_remaining, best_stack_count)
                    }),
                ) {
                    best_disease_part = Some((part, stack_count, lowest_remaining));
                }
            }

            if diseased_count < disease_limit {
                if let Some(target) = first_missing_disease {
                    return Some(target);
                }
            }

            return best_disease_part.map(|(target, _, _)| target);
        }

        if let AttackPattern::DecayingStrikeFocus = self {
            let decay_limit = 5usize;
            let mut best_target: Option<(BossPartName, u64)> = None;

            for part in candidates.iter().copied() {
                let decay_stack_count = boss
                    .afflictions(part)
                    .iter()
                    .find(|aff| aff.kind == AfflictionKind::DecayingStrikeDebuff)
                    .map(|affliction| affliction.stack_count())
                    .unwrap_or(0);

                if decay_stack_count >= decay_limit {
                    continue;
                }

                let boss_part = boss.part(part);
                let remaining_durability = match boss_part.part_state {
                    PartState::Armor | PartState::Cursed => boss_part.current_armor,
                    PartState::Body => boss_part.current_health,
                    PartState::Skeleton => u64::MAX,
                };

                if best_target
                    .map(|(_, best_remaining)| remaining_durability < best_remaining)
                    .unwrap_or(true)
                {
                    best_target = Some((part, remaining_durability));
                }
            }

            return best_target.map(|(target, _)| target);
        }

        if let AttackPattern::BlazingInfernoStack = self {
            let best_burning_stack_count = candidates
                .iter()
                .map(|part| {
                    boss.afflictions(*part)
                        .iter()
                        .find(|aff| aff.kind == AfflictionKind::BlazingInfernoDebuff)
                        .map(|affliction| affliction.stack_count())
                        .unwrap_or(0)
                })
                .filter(|stack_count| *stack_count < 3)
                .min();

            if let Some(best_burning_stack_count) = best_burning_stack_count {
                if let Some(target) = candidates.iter().copied().find(|part| {
                    boss.afflictions(*part)
                        .iter()
                        .find(|aff| aff.kind == AfflictionKind::BlazingInfernoDebuff)
                        .map(|affliction| affliction.stack_count())
                        .unwrap_or(0)
                        == best_burning_stack_count
                }) {
                    return Some(target);
                }
            }
        }

        if let AttackPattern::CelestialStatic = self {
            let celestial_stacks = deck
                .iter()
                .find(|card| card.card_id == CardName::CelestialStatic)
                .map(|card| card.celestial_stacks)
                .unwrap_or(0);

            if celestial_stacks >= 8 {
                if let Some(target) = cycle_filtered_candidates(candidates, last_target, |part| {
                    matches!(part, BossPartName::Head | BossPartName::Torso)
                }) {
                    return Some(target);
                }

                return cycle_filtered_candidates(candidates, last_target, |part| part.is_limb());
            }

            if let Some(target) =
                cycle_filtered_candidates(candidates, last_target, |part| part.is_limb())
            {
                return Some(target);
            }

            return cycle_filtered_candidates(candidates, last_target, |part| {
                matches!(part, BossPartName::Head | BossPartName::Torso)
            });
        }

        //basic attack pattern
        match self {
            AttackPattern::SingleAny
            | AttackPattern::SingleHead
            | AttackPattern::SingleTorso
            | AttackPattern::SingleBody
            | AttackPattern::SingleArmor
            | AttackPattern::SingleCursed
            | AttackPattern::SingleLimb => {
                if let Some(last) = last_target {
                    if candidates.contains(&last) {
                        return Some(last);
                    }
                }

                candidates.first().copied()
            }
            AttackPattern::CycleHeadTorso
            | AttackPattern::CycleLimb
            | AttackPattern::CycleCursed
            | AttackPattern::CycleBody
            | AttackPattern::CycleArmor
            | AttackPattern::CycleAllActive
            | AttackPattern::CycleParts(_)
            | AttackPattern::FusionBombSpread
            | AttackPattern::ThrivingPlagueSpread
            | AttackPattern::RadioactivitySpread
            | AttackPattern::DecayingStrikeFocus
            | AttackPattern::BlazingInfernoStack
            | AttackPattern::CelestialStatic
            | AttackPattern::WhipRuinousFocus => match last_target {
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
            },
        }
    }

    pub fn prepare(
        &self,
        boss: &Boss,
        deck: &[Card],
        attackable_parts: &[BossPartName],
    ) -> PreparedAttackPattern {
        let target_plan = match self {
            AttackPattern::SingleAny
            | AttackPattern::SingleHead
            | AttackPattern::SingleTorso
            | AttackPattern::SingleLimb
            | AttackPattern::CycleHeadTorso
            | AttackPattern::CycleLimb
            | AttackPattern::CycleAllActive => {
                PreparedTargetPlan::Ordered(self.candidate_parts(boss, deck, attackable_parts))
            }
            AttackPattern::CycleParts(count) => PreparedTargetPlan::FirstActiveParts {
                source_parts: self.source_parts(boss, deck, attackable_parts),
                count: *count,
            },
            _ => PreparedTargetPlan::Dynamic,
        };

        PreparedAttackPattern {
            pattern: self.clone(),
            target_plan,
        }
    }

    fn next_prepared_target(
        &self,
        boss: &Boss,
        targets: &[BossPartName],
        last_target: Option<BossPartName>,
    ) -> Option<BossPartName> {
        if pattern_is_single_target(self) {
            if let Some(last) = last_target {
                if targets.contains(&last) && part_is_active(boss, last) {
                    return Some(last);
                }
            }

            return first_active_part(boss, targets);
        }

        cycle_active_parts(boss, targets, last_target)
    }

    fn candidate_parts(
        &self,
        boss: &Boss,
        deck: &[Card],
        attackable_parts: &[BossPartName],
    ) -> Vec<BossPartName> {
        let source_parts = self.source_parts(boss, deck, attackable_parts);

        match self {
            AttackPattern::SingleAny => source_parts,
            AttackPattern::SingleHead => single_part_candidates(&source_parts, BossPartName::Head),
            AttackPattern::SingleTorso => {
                single_part_candidates(&source_parts, BossPartName::Torso)
            }
            AttackPattern::SingleBody => source_parts
                .into_iter()
                .filter(|part| boss.part(*part).part_state == PartState::Body)
                .collect(),
            AttackPattern::SingleArmor => source_parts
                .into_iter()
                .filter(|part| {
                    matches!(
                        boss.part(*part).part_state,
                        PartState::Armor | PartState::Cursed
                    )
                })
                .collect(),
            AttackPattern::SingleLimb => source_parts
                .into_iter()
                .filter(BossPartName::is_limb)
                .collect(),
            AttackPattern::SingleCursed => source_parts
                .into_iter()
                .filter(|part| boss.part(*part).part_state == PartState::Cursed)
                .collect(),
            AttackPattern::CycleHeadTorso => source_parts
                .into_iter()
                .filter(|part| matches!(part, BossPartName::Head | BossPartName::Torso))
                .collect(),
            AttackPattern::CycleLimb => source_parts
                .into_iter()
                .filter(BossPartName::is_limb)
                .collect(),
            AttackPattern::CycleBody => source_parts
                .into_iter()
                .filter(|part| boss.part(*part).part_state == PartState::Body)
                .collect(),
            AttackPattern::CycleArmor => source_parts
                .into_iter()
                .filter(|part| {
                    matches!(
                        boss.part(*part).part_state,
                        PartState::Armor | PartState::Cursed
                    )
                })
                .collect(),
            AttackPattern::CycleAllActive
            | AttackPattern::FusionBombSpread
            | AttackPattern::ThrivingPlagueSpread
            | AttackPattern::RadioactivitySpread
            | AttackPattern::DecayingStrikeFocus
            | AttackPattern::BlazingInfernoStack
            | AttackPattern::CelestialStatic => source_parts,
            AttackPattern::CycleParts(count) => source_parts.into_iter().take(*count).collect(),
            AttackPattern::WhipRuinousFocus => source_parts.into_iter().take(5).collect(),
            AttackPattern::CycleCursed => source_parts
                .into_iter()
                .filter(|part| boss.part(*part).part_state == PartState::Cursed)
                .collect(),
        }
    }

    pub fn fast_calc_target_parts(
        &self,
        boss: &Boss,
        deck: &[Card],
        attackable_parts: &[BossPartName],
    ) -> Vec<BossPartName> {
        self.candidate_parts(boss, deck, attackable_parts)
    }

    fn candidate_parts_buffer(
        &self,
        boss: &Boss,
        deck: &[Card],
        attackable_parts: &[BossPartName],
    ) -> CandidateParts {
        let source_parts = self.source_parts_buffer(boss, deck, attackable_parts);
        let mut candidates = CandidateParts::new();
        let mut take_limit = match self {
            AttackPattern::CycleParts(count) => Some(*count),
            AttackPattern::WhipRuinousFocus => Some(5),
            _ => None,
        };

        for part in source_parts.as_slice().iter().copied() {
            let should_include = match self {
                AttackPattern::SingleAny
                | AttackPattern::CycleAllActive
                | AttackPattern::FusionBombSpread
                | AttackPattern::ThrivingPlagueSpread
                | AttackPattern::RadioactivitySpread
                | AttackPattern::DecayingStrikeFocus
                | AttackPattern::BlazingInfernoStack
                | AttackPattern::CelestialStatic
                | AttackPattern::CycleParts(_)
                | AttackPattern::WhipRuinousFocus => true,
                AttackPattern::SingleHead => part == BossPartName::Head,
                AttackPattern::SingleTorso => part == BossPartName::Torso,
                AttackPattern::SingleBody => boss.part(part).part_state == PartState::Body,
                AttackPattern::SingleArmor => matches!(
                    boss.part(part).part_state,
                    PartState::Armor | PartState::Cursed
                ),
                AttackPattern::SingleLimb => part.is_limb(),
                AttackPattern::SingleCursed => boss.part(part).part_state == PartState::Cursed,
                AttackPattern::CycleHeadTorso => {
                    matches!(part, BossPartName::Head | BossPartName::Torso)
                }
                AttackPattern::CycleLimb => part.is_limb(),
                AttackPattern::CycleBody => boss.part(part).part_state == PartState::Body,
                AttackPattern::CycleArmor => matches!(
                    boss.part(part).part_state,
                    PartState::Armor | PartState::Cursed
                ),
                AttackPattern::CycleCursed => boss.part(part).part_state == PartState::Cursed,
            };

            if !should_include {
                continue;
            }

            if let Some(remaining) = take_limit.as_mut() {
                if *remaining == 0 {
                    break;
                }
                *remaining -= 1;
            }

            candidates.push(part);
        }

        candidates
    }

    fn source_parts(
        &self,
        boss: &Boss,
        deck: &[Card],
        attackable_parts: &[BossPartName],
    ) -> Vec<BossPartName> {
        if self.can_target_untargetable_parts(deck) {
            let source_parts: Vec<BossPartName> = boss
                .parts()
                .iter()
                .copied()
                .map(|part| part.part_name)
                .filter(|part| boss.part(*part).part_state != PartState::Skeleton)
                .collect();

            return source_parts;
        }

        let source_parts: Vec<BossPartName> = attackable_parts
            .iter()
            .copied()
            .filter(|part| boss.part(*part).part_state != PartState::Skeleton)
            .filter(|part| part_passes_support_target_rules(boss, deck, *part))
            .collect();
        source_parts
    }

    fn can_target_untargetable_parts(&self, _deck: &[Card]) -> bool {
        match self {
            AttackPattern::CelestialStatic | AttackPattern::WhipRuinousFocus => true,
            _ => false,
        }
    }

    fn source_parts_buffer(
        &self,
        boss: &Boss,
        deck: &[Card],
        attackable_parts: &[BossPartName],
    ) -> CandidateParts {
        let mut source_parts = CandidateParts::new();

        if self.can_target_untargetable_parts(deck) {
            for part in ALL_BOSS_PARTS {
                if part_is_active(boss, part) {
                    source_parts.push(part);
                }
            }

            return source_parts;
        }

        for part in attackable_parts.iter().copied() {
            if part_is_active(boss, part) && part_passes_support_target_rules(boss, deck, part) {
                source_parts.push(part);
            }
        }

        source_parts
    }
}

fn single_part_candidates(
    source_parts: &[BossPartName],
    target: BossPartName,
) -> Vec<BossPartName> {
    if source_parts.contains(&target) {
        vec![target]
    } else {
        Vec::new()
    }
}

fn lowest_stack_duration(affliction: &Affliction) -> f64 {
    affliction
        .stacks
        .iter()
        .map(|stack| stack.remaining_duration)
        .min_by(|left, right| left.total_cmp(right))
        .unwrap_or(0.0)
}

fn is_better_refresh_target(
    remaining_duration: f64,
    stack_count: usize,
    best: Option<(f64, usize)>,
) -> bool {
    best.map(|(best_remaining_duration, best_stack_count)| {
        remaining_duration
            .total_cmp(&best_remaining_duration)
            .then_with(|| stack_count.cmp(&best_stack_count))
            == Ordering::Less
    })
    .unwrap_or(true)
}

fn cycle_filtered_candidates(
    candidates: &[BossPartName],
    last_target: Option<BossPartName>,
    predicate: impl Fn(BossPartName) -> bool,
) -> Option<BossPartName> {
    let mut first = None;
    let mut return_next = false;

    for part in candidates.iter().copied() {
        if !predicate(part) {
            continue;
        }

        if first.is_none() {
            first = Some(part);
        }

        if return_next {
            return Some(part);
        }

        if Some(part) == last_target {
            return_next = true;
        }
    }

    first
}

fn part_is_active(boss: &Boss, part: BossPartName) -> bool {
    boss.part(part).part_state != PartState::Skeleton
}

fn first_active_part(boss: &Boss, parts: &[BossPartName]) -> Option<BossPartName> {
    parts
        .iter()
        .copied()
        .find(|part| part_is_active(boss, *part))
}

fn cycle_active_parts(
    boss: &Boss,
    parts: &[BossPartName],
    last_target: Option<BossPartName>,
) -> Option<BossPartName> {
    let first = first_active_part(boss, parts)?;
    let Some(last) = last_target else {
        return Some(first);
    };

    let mut return_next = false;

    for part in parts.iter().copied() {
        if !part_is_active(boss, part) {
            continue;
        }

        if return_next {
            return Some(part);
        }

        if part == last {
            return_next = true;
        }
    }

    Some(first)
}

fn cycle_first_active_parts(
    boss: &Boss,
    source_parts: &[BossPartName],
    count: usize,
    last_target: Option<BossPartName>,
) -> Option<BossPartName> {
    let mut first = None;
    let mut return_next = false;
    let mut active_count = 0usize;

    for part in source_parts.iter().copied() {
        if !part_is_active(boss, part) {
            continue;
        }

        if active_count >= count {
            break;
        }

        if first.is_none() {
            first = Some(part);
        }

        if return_next {
            return Some(part);
        }

        if Some(part) == last_target {
            return_next = true;
        }

        active_count += 1;
    }

    first
}

fn part_passes_support_target_rules(boss: &Boss, deck: &[Card], part: BossPartName) -> bool {
    let state = boss.part(part).part_state;

    for card in deck {
        match card.card_id {
            CardName::GraspingVines => {
                if !part.is_limb() {
                    return false;
                }
            }
            CardName::InspiringForce => {
                if state != PartState::Body {
                    return false;
                }
            }
            CardName::PrismaticRift | CardName::SkeletalSmash => {
                if !matches!(state, PartState::Armor | PartState::Cursed) {
                    return false;
                }
            }
            CardName::SoulFire | CardName::CrushingInstinct => {
                if !matches!(part, BossPartName::Head | BossPartName::Torso) {
                    return false;
                }
            }
            _ => {}
        }
    }

    true
}

pub fn generate_attack_patterns(sim_stats: &SimStats, deck: &[Card]) -> Vec<AttackPattern> {
    if deck_is_target_insensitive(deck) {
        let pattern = AttackPattern::SingleAny;
        return if pattern_has_candidates(&pattern, sim_stats, deck) {
            vec![pattern]
        } else {
            Vec::new()
        };
    }

    let mut patterns = Vec::new();

    for pattern in base_attack_patterns() {
        if pattern_is_available_for_deck(&pattern, deck)
            && pattern_passes_deck_rules(&pattern, sim_stats, deck)
            && pattern_has_candidates(&pattern, sim_stats, deck)
        {
            patterns.push(pattern);
        }
    }

    dedupe_generic_attack_patterns(sim_stats, deck, &mut patterns);
    keep_max_coverage_spread_affliction_patterns(sim_stats, deck, &mut patterns);
    keep_top_attack_patterns(sim_stats, deck, &mut patterns);
    // debug_print_head_torso_support_patterns(sim_stats, deck, &patterns);

    patterns
}

fn base_attack_patterns() -> Vec<AttackPattern> {
    vec![
        AttackPattern::SingleAny,
        AttackPattern::SingleHead,
        AttackPattern::SingleTorso,
        AttackPattern::SingleBody,
        AttackPattern::SingleArmor,
        AttackPattern::SingleLimb,
        AttackPattern::CycleHeadTorso,
        AttackPattern::CycleLimb,
        AttackPattern::CycleBody,
        AttackPattern::CycleArmor,
        AttackPattern::CycleParts(2),
        AttackPattern::CycleParts(3),
        AttackPattern::CycleParts(4),
        AttackPattern::CycleParts(5),
        AttackPattern::CycleParts(6),
        AttackPattern::CycleParts(7),
        AttackPattern::CycleAllActive,
        //new
        AttackPattern::SingleCursed,
        AttackPattern::CycleCursed,
        // card specific patterns
        AttackPattern::FusionBombSpread,
        AttackPattern::ThrivingPlagueSpread,
        AttackPattern::RadioactivitySpread,
        AttackPattern::DecayingStrikeFocus,
        AttackPattern::BlazingInfernoStack,
        AttackPattern::CelestialStatic,
        AttackPattern::WhipRuinousFocus,
    ]
}

fn pattern_has_candidates(pattern: &AttackPattern, sim_stats: &SimStats, deck: &[Card]) -> bool {
    if let AttackPattern::CycleParts(count) = pattern {
        let active_part_count = pattern
            .source_parts(&sim_stats.boss_stat, deck, &sim_stats.attackable_part)
            .len();

        return *count > 1 && *count < active_part_count;
    }

    !pattern
        .candidate_parts(&sim_stats.boss_stat, deck, &sim_stats.attackable_part)
        .is_empty()
}

fn pattern_is_available_for_deck(pattern: &AttackPattern, deck: &[Card]) -> bool {
    match pattern {
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

fn deck_is_target_insensitive(deck: &[Card]) -> bool {
    deck.iter()
        .all(|card| card_is_target_insensitive(card.card_id))
}

fn card_is_target_insensitive(card_name: CardName) -> bool {
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

fn deck_has_target_insensitive_card(deck: &[Card]) -> bool {
    deck.iter()
        .any(|card| card_is_target_insensitive(card.card_id))
}

fn deck_has_single_pattern_rejected_affliction(deck: &[Card]) -> bool {
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

fn dedupe_generic_attack_patterns(
    sim_stats: &SimStats,
    deck: &[Card],
    patterns: &mut Vec<AttackPattern>,
) {
    let mut seen_signatures: Vec<(u8, Vec<BossPartName>)> = Vec::new();

    patterns.retain(|pattern| {
        if pattern_is_card_specific(pattern) {
            return true;
        }

        let Some(signature) = generic_pattern_signature(pattern, sim_stats, deck) else {
            return true;
        };

        if seen_signatures.contains(&signature) {
            return false;
        }

        seen_signatures.push(signature);
        true
    });
}

fn keep_max_coverage_spread_affliction_patterns(
    sim_stats: &SimStats,
    deck: &[Card],
    patterns: &mut Vec<AttackPattern>,
) {
    if !deck_has_spread_affliction(deck) || patterns.len() <= 1 {
        return;
    }

    let max_candidate_count = patterns
        .iter()
        .map(|pattern| {
            pattern
                .candidate_parts(&sim_stats.boss_stat, deck, &sim_stats.attackable_part)
                .len()
        })
        .max()
        .unwrap_or(0);

    if max_candidate_count == 0 {
        return;
    }

    patterns.retain(|pattern| {
        pattern
            .candidate_parts(&sim_stats.boss_stat, deck, &sim_stats.attackable_part)
            .len()
            == max_candidate_count
    });
}

fn deck_has_spread_affliction(deck: &[Card]) -> bool {
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

fn keep_top_attack_patterns(
    sim_stats: &SimStats,
    deck: &[Card],
    patterns: &mut Vec<AttackPattern>,
) {
    if MAX_ATTACK_PATTERNS_PER_DECK == 0 || patterns.len() <= MAX_ATTACK_PATTERNS_PER_DECK {
        return;
    }

    let mut ranked_patterns = patterns.drain(..).enumerate().collect::<Vec<_>>();

    ranked_patterns.sort_by(|(left_index, left_pattern), (right_index, right_pattern)| {
        attack_pattern_priority(right_pattern, sim_stats, deck)
            .cmp(&attack_pattern_priority(left_pattern, sim_stats, deck))
            .then_with(|| left_index.cmp(right_index))
    });

    patterns.extend(
        ranked_patterns
            .into_iter()
            .take(MAX_ATTACK_PATTERNS_PER_DECK)
            .map(|(_, pattern)| pattern),
    );
}

fn attack_pattern_priority(
    pattern: &AttackPattern,
    sim_stats: &SimStats,
    deck: &[Card],
) -> (i32, usize, usize) {
    let candidates =
        pattern.candidate_parts(&sim_stats.boss_stat, deck, &sim_stats.attackable_part);
    let candidate_count = candidates.len();
    let source_count = pattern
        .source_parts(&sim_stats.boss_stat, deck, &sim_stats.attackable_part)
        .len();

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

fn deck_wants_wide_attack(deck: &[Card]) -> bool {
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

fn support_pattern_fit_score(pattern: &AttackPattern, deck: &[Card]) -> i32 {
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

fn burst_target_fit_score(
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

fn matching_target_score(
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

fn matching_state_score(
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

fn ratio_score(matching_count: usize, total_count: usize, max_score: i32) -> i32 {
    if matching_count == 0 || total_count == 0 {
        return 0;
    }

    ((matching_count as f64 / total_count as f64) * max_score as f64).round() as i32
        + matching_count as i32 * 50
}

fn barbed_morningstar_target_score(card: &Card, boss: &Boss, candidates: &[BossPartName]) -> i32 {
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

fn pattern_shape_score(
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

fn generic_pattern_signature(
    pattern: &AttackPattern,
    sim_stats: &SimStats,
    deck: &[Card],
) -> Option<(u8, Vec<BossPartName>)> {
    let candidates =
        pattern.candidate_parts(&sim_stats.boss_stat, deck, &sim_stats.attackable_part);

    if candidates.is_empty() {
        return None;
    }

    let mode = if pattern_is_single_target(pattern) || candidates.len() <= 1 {
        0
    } else if pattern_is_cycle_target(pattern) {
        1
    } else {
        return None;
    };

    Some((mode, candidates))
}

fn pattern_passes_deck_rules(pattern: &AttackPattern, sim_stats: &SimStats, deck: &[Card]) -> bool {
    if deck_has_card(deck, CardName::TotemOfPower) && pattern_is_cycle_target(pattern) {
        return false;
    }
    if deck_has_card(deck, CardName::FusionBomb)
        && pattern
            .candidate_parts(&sim_stats.boss_stat, deck, &sim_stats.attackable_part)
            .len()
            < 3
    {
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

fn pattern_is_cycle_target(pattern: &AttackPattern) -> bool {
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

fn pattern_is_card_specific(pattern: &AttackPattern) -> bool {
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

fn pattern_is_single_target(pattern: &AttackPattern) -> bool {
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

fn deck_has_card(deck: &[Card], card_name: CardName) -> bool {
    deck.iter().any(|card| card.card_id == card_name)
}

#[allow(dead_code)]
fn debug_print_head_torso_support_patterns(
    sim_stats: &SimStats,
    deck: &[Card],
    patterns: &[AttackPattern],
) {
    if !DEBUG_HEAD_TORSO_SUPPORT_PATTERNS {
        return;
    }

    if !deck_has_card(deck, CardName::SoulFire) && !deck_has_card(deck, CardName::CrushingInstinct)
    {
        return;
    }

    let deck_names = deck
        .iter()
        .map(|card| card.card_id.display_name())
        .collect::<Vec<_>>()
        .join(" | ");

    println!(
        "[PATTERN DEBUG] deck {} | total patterns {}",
        deck_names,
        patterns.len()
    );

    for pattern in patterns {
        let candidates =
            pattern.candidate_parts(&sim_stats.boss_stat, deck, &sim_stats.attackable_part);
        let candidate_names = candidates
            .iter()
            .map(|part| {
                format!(
                    "{:?}({:?})",
                    part,
                    sim_stats.boss_stat.part(*part).part_state
                )
            })
            .collect::<Vec<_>>()
            .join(", ");

        println!(
            "[PATTERN DEBUG]   {} -> [{}]",
            pattern.describe(),
            candidate_names
        );
    }
}
