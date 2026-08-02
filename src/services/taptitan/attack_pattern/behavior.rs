use super::*;

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

    pub(super) fn next_prepared_target(
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

    pub(super) fn candidate_parts(
        &self,
        boss: &Boss,
        deck: &[Card],
        attackable_parts: &[BossPartName],
    ) -> Vec<BossPartName> {
        let source_parts = self.source_parts(boss, deck, attackable_parts);
        self.candidate_parts_from_source(boss, &source_parts)
    }

    pub(super) fn candidate_parts_from_source(
        &self,
        boss: &Boss,
        source_parts: &[BossPartName],
    ) -> Vec<BossPartName> {
        match self {
            AttackPattern::SingleAny => source_parts.to_vec(),
            AttackPattern::SingleHead => single_part_candidates(source_parts, BossPartName::Head),
            AttackPattern::SingleTorso => single_part_candidates(source_parts, BossPartName::Torso),
            AttackPattern::SingleBody => source_parts
                .iter()
                .copied()
                .filter(|part| boss.part(*part).part_state == PartState::Body)
                .collect(),
            AttackPattern::SingleArmor => source_parts
                .iter()
                .copied()
                .filter(|part| {
                    matches!(
                        boss.part(*part).part_state,
                        PartState::Armor | PartState::Cursed
                    )
                })
                .collect(),
            AttackPattern::SingleLimb => source_parts
                .iter()
                .copied()
                .filter(BossPartName::is_limb)
                .collect(),
            AttackPattern::SingleCursed => source_parts
                .iter()
                .copied()
                .filter(|part| boss.part(*part).part_state == PartState::Cursed)
                .collect(),
            AttackPattern::CycleHeadTorso => source_parts
                .iter()
                .copied()
                .filter(|part| matches!(part, BossPartName::Head | BossPartName::Torso))
                .collect(),
            AttackPattern::CycleLimb => source_parts
                .iter()
                .copied()
                .filter(BossPartName::is_limb)
                .collect(),
            AttackPattern::CycleBody => source_parts
                .iter()
                .copied()
                .filter(|part| boss.part(*part).part_state == PartState::Body)
                .collect(),
            AttackPattern::CycleArmor => source_parts
                .iter()
                .copied()
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
            | AttackPattern::CelestialStatic => source_parts.to_vec(),
            AttackPattern::CycleParts(count) => source_parts.iter().copied().take(*count).collect(),
            AttackPattern::WhipRuinousFocus => source_parts.iter().copied().take(5).collect(),
            AttackPattern::CycleCursed => source_parts
                .iter()
                .copied()
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

    pub(super) fn candidate_parts_buffer(
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

    pub(super) fn source_parts(
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

    pub(super) fn can_target_untargetable_parts(&self, _deck: &[Card]) -> bool {
        match self {
            AttackPattern::CelestialStatic | AttackPattern::WhipRuinousFocus => true,
            _ => false,
        }
    }

    pub(super) fn source_parts_buffer(
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
