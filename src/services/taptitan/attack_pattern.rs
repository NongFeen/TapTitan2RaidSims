use crate::models::affliction::AfflictionKind;
use crate::models::boss::{Boss, BossPartName, PartState};
use crate::models::cards::{Card, CardName};

use super::sim_service::SimStats;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AttackPattern {
    SingleAny,
    SingleHead,
    SingleTorso,
    SingleBody,
    SingleArmor,
    SingleLimb,
    CycleHeadTorso,
    CycleLimb,
    CycleAllActive,
    FocusParts(usize),
    CelestialStatic,
    WhipRuinousFocus,
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
            AttackPattern::CycleHeadTorso => "CycleHeadTorso".to_string(),
            AttackPattern::CycleLimb => "CycleLimb".to_string(),
            AttackPattern::CycleAllActive => "CycleAllActive".to_string(),
            AttackPattern::FocusParts(count) => format!("FocusParts({})", count),
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
        let candidates = self.candidate_parts(boss, deck, attackable_parts);
        if candidates.is_empty() {
            return None;
        }

        if let AttackPattern::FocusParts(focus_count) = self {
            let disease_parts: Vec<(BossPartName, usize, f64)> = candidates
                .iter()
                .copied()
                .filter_map(|part| {
                    let affliction = boss
                        .part(part)
                        .afflictions
                        .iter()
                        .find(|aff| aff.kind == AfflictionKind::RadioactivityDebuff)?;

                    Some((
                        part,
                        affliction.stack_count(),
                        affliction
                            .stacks
                            .iter()
                            .map(|stack| stack.remaining_duration)
                            .min_by(|left, right| left.total_cmp(right))
                            .unwrap_or(0.0),
                    ))
                })
                .collect();

            let has_clean_part = candidates.iter().copied().any(|part| {
                boss.part(part)
                    .afflictions
                    .iter()
                    .all(|aff| aff.kind != AfflictionKind::RadioactivityDebuff)
            });

            if disease_parts.len() < *focus_count && has_clean_part {
                if let Some(target) = candidates.iter().copied().find(|part| {
                    boss.part(*part)
                        .afflictions
                        .iter()
                        .all(|aff| aff.kind != AfflictionKind::RadioactivityDebuff)
                }) {
                    return Some(target);
                }
            }

            if let Some((target, _, _)) = disease_parts.into_iter().min_by(|left, right| {
                left.2
                    .total_cmp(&right.2)
                    .then_with(|| left.1.cmp(&right.1))
            }) {
                return Some(target);
            }
        }

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

        //fuse deck
        if deck.iter().any(|card| card.card_id == CardName::FusionBomb) {
            if let Some(open_part) = candidates.iter().copied().find(|part| {
                !boss
                    .part(*part)
                    .afflictions
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

        //acid (sword posion)
        if deck.iter().any(|card| card.card_id == CardName::AcidDrench) {
            let poisoned_parts: Vec<(BossPartName, usize, f64)> = candidates
                .iter()
                .copied()
                .filter_map(|part| {
                    let affliction = boss
                        .part(part)
                        .afflictions
                        .iter()
                        .find(|aff| aff.kind == AfflictionKind::AcidDrenchDebuff)?;

                    Some((
                        part,
                        affliction.stack_count(),
                        affliction
                            .stacks
                            .iter()
                            .map(|stack| stack.remaining_duration)
                            .min_by(|left, right| left.total_cmp(right))
                            .unwrap_or(0.0),
                    ))
                })
                .collect();

            let mut focus_parts: Vec<(BossPartName, usize, f64)> = poisoned_parts
                .iter()
                .copied()
                .filter(|(_, stack_count, remaining_duration)| {
                    *stack_count < 15 && *remaining_duration <= 2.0
                })
                .collect();

            if focus_parts.is_empty() {
                focus_parts = poisoned_parts
                    .iter()
                    .copied()
                    .filter(|(_, stack_count, _)| *stack_count < 15)
                    .collect();
            }

            if let Some((target, _, _)) = focus_parts.into_iter().min_by(|left, right| {
                left.2
                    .total_cmp(&right.2)
                    .then_with(|| left.1.cmp(&right.1))
            }) {
                return Some(target);
            }
        }

        // plague
        if deck
            .iter()
            .any(|card| card.card_id == CardName::ThrivingPlague)
        {
            let plague_parts: Vec<(BossPartName, usize, f64)> = candidates
                .iter()
                .copied()
                .filter_map(|part| {
                    let affliction = boss
                        .part(part)
                        .afflictions
                        .iter()
                        .find(|aff| aff.kind == AfflictionKind::ThrivingPlagueDebuff)?;

                    Some((
                        part,
                        affliction.stack_count(),
                        affliction
                            .stacks
                            .iter()
                            .map(|stack| stack.remaining_duration)
                            .min_by(|left, right| left.total_cmp(right))
                            .unwrap_or(0.0),
                    ))
                })
                .collect();

            let missing_plague_parts: Vec<BossPartName> = candidates
                .iter()
                .copied()
                .filter(|part| {
                    boss.part(*part)
                        .afflictions
                        .iter()
                        .all(|aff| aff.kind != AfflictionKind::ThrivingPlagueDebuff)
                })
                .collect();

            if let Some(target) = missing_plague_parts.first().copied() {
                return Some(target);
            }

            if let Some((target, _, _)) = plague_parts.into_iter().min_by(|left, right| {
                left.2
                    .total_cmp(&right.2)
                    .then_with(|| left.1.cmp(&right.1))
            }) {
                return Some(target);
            }
        }

        //radio
        if deck
            .iter()
            .any(|card| card.card_id == CardName::Radioactivity)
        {
            let disease_parts: Vec<(BossPartName, usize, f64)> = candidates
                .iter()
                .copied()
                .filter_map(|part| {
                    let affliction = boss
                        .part(part)
                        .afflictions
                        .iter()
                        .find(|aff| aff.kind == AfflictionKind::RadioactivityDebuff)?;

                    Some((
                        part,
                        affliction.stack_count(),
                        affliction
                            .stacks
                            .iter()
                            .map(|stack| stack.remaining_duration)
                            .min_by(|left, right| left.total_cmp(right))
                            .unwrap_or(0.0),
                    ))
                })
                .collect();

            let disease_limit = 6usize;
            let diseased_count = disease_parts.len();

            if diseased_count < disease_limit {
                if let Some(target) = candidates.iter().copied().find(|part| {
                    boss.part(*part)
                        .afflictions
                        .iter()
                        .all(|aff| aff.kind != AfflictionKind::RadioactivityDebuff)
                }) {
                    return Some(target);
                }
            }

            if let Some((target, _, _)) = disease_parts.into_iter().min_by(|left, right| {
                left.2
                    .total_cmp(&right.2)
                    .then_with(|| left.1.cmp(&right.1))
            }) {
                return Some(target);
            }
        }

        //decay
        if deck
            .iter()
            .any(|card| card.card_id == CardName::DecayingStrike)
        {
            let decay_limit = 5usize;
            let mut eligible_parts: Vec<(BossPartName, u64)> = candidates
                .iter()
                .copied()
                .filter_map(|part| {
                    let decay_stack_count = boss
                        .part(part)
                        .afflictions
                        .iter()
                        .find(|aff| aff.kind == AfflictionKind::DecayingStrikeDebuff)
                        .map(|affliction| affliction.stack_count())
                        .unwrap_or(0);

                    if decay_stack_count >= decay_limit {
                        return None;
                    }

                    let boss_part = boss.part(part);
                    let remaining_durability = match boss_part.part_state {
                        PartState::Armor | PartState::Cursed => boss_part.current_armor,
                        PartState::Body => boss_part.current_health,
                        PartState::Skeleton => u64::MAX,
                    };

                    Some((part, remaining_durability))
                })
                .collect();

            if let Some((target, _)) = eligible_parts
                .into_iter()
                .min_by_key(|(_, remaining_durability)| *remaining_durability)
            {
                return Some(target);
            }
        }

        //shadow (keep 7 stack)
        if deck.iter().any(|card| card.card_id == CardName::GrimShadow) {
            let shadow_limit = 7usize;
            let mut shadow_parts: Vec<(BossPartName, usize)> = candidates
                .iter()
                .copied()
                .filter_map(|part| {
                    let shadow_stack_count = boss
                        .part(part)
                        .afflictions
                        .iter()
                        .find(|aff| aff.kind == AfflictionKind::GrimShadowDebuff)
                        .map(|affliction| affliction.stack_count())
                        .unwrap_or(0);

                    if shadow_stack_count >= shadow_limit {
                        return None;
                    }

                    Some((part, shadow_stack_count))
                })
                .collect();

            shadow_parts.sort_by_key(|(_, stack_count)| *stack_count);
            shadow_parts.truncate(3);

            if let Some((target, _)) = shadow_parts
                .into_iter()
                .min_by_key(|(_, stack_count)| *stack_count)
            {
                return Some(target);
            }
        }

        //inferno (keep all part 3 stack)
        if deck
            .iter()
            .any(|card| card.card_id == CardName::BlazingInferno)
        {
            let best_burning_stack_count = candidates
                .iter()
                .map(|part| {
                    boss.part(*part)
                        .afflictions
                        .iter()
                        .find(|aff| aff.kind == AfflictionKind::BlazingInfernoDebuff)
                        .map(|affliction| affliction.stack_count())
                        .unwrap_or(0)
                })
                .filter(|stack_count| *stack_count < 3)
                .min();

            if let Some(best_burning_stack_count) = best_burning_stack_count {
                if let Some(target) = candidates.iter().copied().find(|part| {
                    boss.part(*part)
                        .afflictions
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

        //celestial
        if deck
            .iter()
            .any(|card| card.card_id == CardName::CelestialStatic)
        {
            let limb_candidates: Vec<BossPartName> = candidates
                .iter()
                .copied()
                .filter(BossPartName::is_limb)
                .collect();

            if !limb_candidates.is_empty() {
                return match last_target {
                    Some(last) => {
                        if let Some(index) = limb_candidates.iter().position(|part| *part == last) {
                            limb_candidates
                                .get((index + 1) % limb_candidates.len())
                                .copied()
                                .or_else(|| limb_candidates.first().copied())
                        } else {
                            limb_candidates.first().copied()
                        }
                    }
                    None => limb_candidates.first().copied(),
                };
            }

            let head_torso_candidates: Vec<BossPartName> = candidates
                .iter()
                .copied()
                .filter(|part| matches!(part, BossPartName::Head | BossPartName::Torso))
                .collect();

            if !head_torso_candidates.is_empty() {
                return match last_target {
                    Some(last) => {
                        if let Some(index) =
                            head_torso_candidates.iter().position(|part| *part == last)
                        {
                            head_torso_candidates
                                .get((index + 1) % head_torso_candidates.len())
                                .copied()
                                .or_else(|| head_torso_candidates.first().copied())
                        } else {
                            head_torso_candidates.first().copied()
                        }
                    }
                    None => head_torso_candidates.first().copied(),
                };
            }
        }

        //rain try to focus cursed -> if multiple just cycle
        if deck
            .iter()
            .any(|card| card.card_id == CardName::RuinousRain)
        {
            let cursed_parts: Vec<BossPartName> = candidates
                .iter()
                .copied()
                .filter(|part| boss.part(*part).part_state == PartState::Cursed)
                .collect();

            if let Some(target) = cursed_parts.first().copied() {
                return Some(target);
            }

            if candidates.len() > 1 {
                return match last_target {
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
                };
            }
        }

        //basic attack pattern
        match self {
            AttackPattern::SingleAny
            | AttackPattern::SingleHead
            | AttackPattern::SingleTorso
            | AttackPattern::SingleBody
            | AttackPattern::SingleArmor
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
            | AttackPattern::CycleAllActive
            | AttackPattern::FocusParts(_)
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
                .filter(|part| boss.part(*part).part_state == PartState::Armor)
                .collect(),
            AttackPattern::SingleLimb => source_parts
                .into_iter()
                .filter(BossPartName::is_limb)
                .collect(),
            AttackPattern::CycleHeadTorso => source_parts
                .into_iter()
                .filter(|part| matches!(part, BossPartName::Head | BossPartName::Torso))
                .collect(),
            AttackPattern::CycleLimb => source_parts
                .into_iter()
                .filter(BossPartName::is_limb)
                .collect(),
            AttackPattern::CycleAllActive
            | AttackPattern::FocusParts(_)
            | AttackPattern::CelestialStatic => source_parts,
            AttackPattern::WhipRuinousFocus => source_parts.into_iter().take(5).collect(),
        }
    }

    fn source_parts(
        &self,
        boss: &Boss,
        deck: &[Card],
        attackable_parts: &[BossPartName],
    ) -> Vec<BossPartName> {
        if self.can_target_untargetable_parts(deck) {
            return boss
                .parts()
                .iter()
                .copied()
                .map(|part| part.part_name)
                .filter(|part| boss.part(*part).part_state != PartState::Skeleton)
                .collect();
        }

        attackable_parts
            .iter()
            .copied()
            .filter(|part| boss.part(*part).part_state != PartState::Skeleton)
            .collect()
    }

    fn can_target_untargetable_parts(&self, deck: &[Card]) -> bool {
        match self {
            AttackPattern::CelestialStatic => deck
                .iter()
                .any(|card| card.card_id == CardName::CelestialStatic),
            AttackPattern::WhipRuinousFocus => deck.iter().any(|card| {
                matches!(
                    card.card_id,
                    CardName::WhipOfLightning | CardName::RuinousRain
                )
            }),
            _ => false,
        }
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

pub fn generate_attack_patterns(sim_stats: &SimStats, deck: &[Card]) -> Vec<AttackPattern> {
    let mut patterns = Vec::new();

    for pattern in base_attack_patterns() {
        if pattern_is_allowed_for_deck(&pattern, deck)
            && pattern_has_candidates(&pattern, sim_stats, deck)
        {
            patterns.push(pattern);
        }
    }

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
        AttackPattern::CycleAllActive,
        AttackPattern::FocusParts(2),
        AttackPattern::FocusParts(3),
        AttackPattern::FocusParts(4),
        AttackPattern::FocusParts(5),
        AttackPattern::FocusParts(6),
        AttackPattern::CelestialStatic,
        AttackPattern::WhipRuinousFocus,
    ]
}

fn pattern_has_candidates(pattern: &AttackPattern, sim_stats: &SimStats, deck: &[Card]) -> bool {
    !pattern
        .candidate_parts(&sim_stats.boss_stat, deck, &sim_stats.attackable_part)
        .is_empty()
}

fn pattern_is_allowed_for_deck(pattern: &AttackPattern, deck: &[Card]) -> bool {
    let has_head_torso_focus = deck.iter().any(|card| {
        matches!(
            card.card_id,
            CardName::CrushingInstinct | CardName::SoulFire
        )
    });
    let has_limb_focus = deck
        .iter()
        .any(|card| card.card_id == CardName::GraspingVines);
    let has_single_target_focus = deck
        .iter()
        .any(|card| card.card_id == CardName::TotemOfPower);
    let has_body_focus = deck
        .iter()
        .any(|card| card.card_id == CardName::InspiringForce);
    let has_armor_focus = deck
        .iter()
        .any(|card| card.card_id == CardName::PrismaticRift);
    let has_radioactivity = deck
        .iter()
        .any(|card| card.card_id == CardName::Radioactivity);
    let has_ruinous_rain = deck
        .iter()
        .any(|card| card.card_id == CardName::RuinousRain);
    let has_corrosive_bubbles = deck
        .iter()
        .any(|card| card.card_id == CardName::CorrosiveBubbles);
    let has_celestial_static = deck
        .iter()
        .any(|card| card.card_id == CardName::CelestialStatic);
    let has_whip_of_lightning = deck
        .iter()
        .any(|card| card.card_id == CardName::WhipOfLightning);

    if has_head_torso_focus {
        return matches!(
            pattern,
            AttackPattern::SingleHead | AttackPattern::SingleTorso | AttackPattern::CycleHeadTorso
        );
    }

    if has_limb_focus {
        return matches!(
            pattern,
            AttackPattern::SingleLimb | AttackPattern::CycleLimb
        );
    }

    if has_body_focus {
        return matches!(
            pattern,
            AttackPattern::SingleBody | AttackPattern::CycleAllActive
        );
    }

    if has_armor_focus {
        return matches!(
            pattern,
            AttackPattern::SingleArmor | AttackPattern::CycleAllActive
        );
    }

    if has_single_target_focus {
        return matches!(
            pattern,
            AttackPattern::SingleAny
                | AttackPattern::SingleHead
                | AttackPattern::SingleTorso
                | AttackPattern::SingleBody
                | AttackPattern::SingleArmor
                | AttackPattern::SingleLimb
        );
    }

    if has_celestial_static {
        return matches!(
            pattern,
            AttackPattern::CelestialStatic
                | AttackPattern::SingleLimb
                | AttackPattern::CycleLimb
                | AttackPattern::SingleHead
                | AttackPattern::SingleTorso
                | AttackPattern::CycleHeadTorso
        );
    }

    if matches!(pattern, AttackPattern::CelestialStatic) {
        return has_celestial_static;
    }

    if matches!(pattern, AttackPattern::WhipRuinousFocus) {
        return has_whip_of_lightning && has_ruinous_rain;
    }

    if has_whip_of_lightning && has_ruinous_rain {
        return matches!(
            pattern,
            AttackPattern::WhipRuinousFocus
                | AttackPattern::CycleAllActive
                | AttackPattern::CycleHeadTorso
                | AttackPattern::CycleLimb
        );
    }

    if has_radioactivity {
        return matches!(
            pattern,
            AttackPattern::FocusParts(2)
                | AttackPattern::FocusParts(3)
                | AttackPattern::FocusParts(4)
                | AttackPattern::FocusParts(5)
                | AttackPattern::FocusParts(6)
        );
    }

    if has_ruinous_rain {
        return matches!(
            pattern,
            AttackPattern::CycleAllActive
                | AttackPattern::SingleAny
                | AttackPattern::FocusParts(2)
                | AttackPattern::FocusParts(3)
                | AttackPattern::FocusParts(4)
                | AttackPattern::FocusParts(5)
                | AttackPattern::FocusParts(6)
        );
    }

    if has_corrosive_bubbles {
        return matches!(
            pattern,
            AttackPattern::SingleAny
                | AttackPattern::SingleHead
                | AttackPattern::SingleTorso
                | AttackPattern::SingleBody
                | AttackPattern::SingleArmor
                | AttackPattern::SingleLimb
        );
    }

    if has_whip_of_lightning {
        return matches!(
            pattern,
            AttackPattern::CycleAllActive
                | AttackPattern::CycleHeadTorso
                | AttackPattern::CycleLimb
                | AttackPattern::SingleAny
        );
    }

    true
}
