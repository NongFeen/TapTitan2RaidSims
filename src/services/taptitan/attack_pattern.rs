use crate::models::affliction::{Affliction, AfflictionKind};
use crate::models::boss::{Boss, BossPartName, PartState};
use crate::models::cards::{Card, CardName, CardType};
use std::cmp::Ordering;

use super::sim_service::SimStats;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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

const MAX_ATTACK_PATTERNS_PER_DECK: usize = 3; // 0 = no cap

const BASE_ATTACK_PATTERNS: &[AttackPattern] = &[
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
    AttackPattern::SingleCursed,
    AttackPattern::CycleCursed,
    AttackPattern::FusionBombSpread,
    AttackPattern::ThrivingPlagueSpread,
    AttackPattern::RadioactivitySpread,
    AttackPattern::DecayingStrikeFocus,
    AttackPattern::BlazingInfernoStack,
    AttackPattern::CelestialStatic,
    AttackPattern::WhipRuinousFocus,
];

struct CandidateParts {
    parts: [BossPartName; 8],
    len: usize,
}

struct AttackPatternInfo {
    pattern: AttackPattern,
    candidates: Vec<BossPartName>,
    source_count: usize,
    priority: (i32, usize, usize),
}

impl AttackPatternInfo {
    fn new(pattern: AttackPattern, sim_stats: &SimStats, deck: &[Card]) -> Self {
        let source_parts =
            pattern.source_parts(&sim_stats.boss_stat, deck, &sim_stats.attackable_part);
        let source_count = source_parts.len();
        let candidates = pattern.candidate_parts_from_source(&sim_stats.boss_stat, &source_parts);
        let priority = attack_pattern_priority_from_parts(
            &pattern,
            sim_stats,
            deck,
            &candidates,
            source_count,
        );

        Self {
            pattern,
            candidates,
            source_count,
            priority,
        }
    }
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

mod behavior;
mod generation;
mod targeting;

use generation::*;
pub use generation::{generate_all_attack_patterns, generate_attack_patterns};
use targeting::*;
