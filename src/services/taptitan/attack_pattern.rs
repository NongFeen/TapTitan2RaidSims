use crate::models::affliction::AfflictionKind;
use crate::models::boss::{Boss, BossPartName, PartState};
use crate::models::cards::{Card, CardName, CardType};

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
    AcidDrenchStack,
    ThrivingPlagueSpread,
    RadioactivitySpread,
    DecayingStrikeFocus,
    GrimShadowStack,
    BlazingInfernoStack,
    CelestialStatic,
    RuinousRainFocus,
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
            AttackPattern::SingleCursed => "SingleCursed".to_string(),
            AttackPattern::CycleHeadTorso => "CycleHeadTorso".to_string(),
            AttackPattern::CycleCursed => "CycleCursed".to_string(),
            AttackPattern::CycleLimb => "CycleLimb".to_string(),
            AttackPattern::CycleArmor => "CycleArmor".to_string(),
            AttackPattern::CycleBody => "CycleBody".to_string(),
            AttackPattern::CycleAllActive => "CycleAllActive".to_string(),
            AttackPattern::CycleParts(count) => format!("CycleParts({})", count),
            AttackPattern::FusionBombSpread => "FusionBombSpread".to_string(),
            AttackPattern::AcidDrenchStack => "AcidDrenchStack".to_string(),
            AttackPattern::ThrivingPlagueSpread => "ThrivingPlagueSpread".to_string(),
            AttackPattern::RadioactivitySpread => "RadioactivitySpread".to_string(),
            AttackPattern::DecayingStrikeFocus => "DecayingStrikeFocus".to_string(),
            AttackPattern::GrimShadowStack => "GrimShadowStack".to_string(),
            AttackPattern::BlazingInfernoStack => "BlazingInfernoStack".to_string(),
            AttackPattern::CelestialStatic => "CelestialStatic".to_string(),
            AttackPattern::RuinousRainFocus => "RuinousRainFocus".to_string(),
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

        if let AttackPattern::AcidDrenchStack = self {
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

        if let AttackPattern::ThrivingPlagueSpread = self {
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

        if let AttackPattern::RadioactivitySpread = self {
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

        if let AttackPattern::DecayingStrikeFocus = self {
            let decay_limit = 5usize;
            let eligible_parts: Vec<(BossPartName, u64)> = candidates
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

        if let AttackPattern::GrimShadowStack = self {
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

        if let AttackPattern::BlazingInfernoStack = self {
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

        if let AttackPattern::CelestialStatic = self {
            let celestial_stacks = deck
                .iter()
                .find(|card| card.card_id == CardName::CelestialStatic)
                .map(|card| card.celestial_stacks)
                .unwrap_or(0);

            let limb_candidates: Vec<BossPartName> = candidates
                .iter()
                .copied()
                .filter(BossPartName::is_limb)
                .collect();

            let head_torso_candidates: Vec<BossPartName> = candidates
                .iter()
                .copied()
                .filter(|part| matches!(part, BossPartName::Head | BossPartName::Torso))
                .collect();

            if celestial_stacks >= 8 {
                if let Some(target) = cycle_candidates(&head_torso_candidates, last_target) {
                    return Some(target);
                }

                return cycle_candidates(&limb_candidates, last_target);
            }

            if let Some(target) = cycle_candidates(&limb_candidates, last_target) {
                return Some(target);
            }

            return cycle_candidates(&head_torso_candidates, last_target);
        }

        if let AttackPattern::RuinousRainFocus = self {
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
            | AttackPattern::AcidDrenchStack
            | AttackPattern::ThrivingPlagueSpread
            | AttackPattern::RadioactivitySpread
            | AttackPattern::DecayingStrikeFocus
            | AttackPattern::GrimShadowStack
            | AttackPattern::BlazingInfernoStack
            | AttackPattern::CelestialStatic
            | AttackPattern::RuinousRainFocus
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
                .filter(|part| boss.part(*part).part_state == PartState::Armor)
                .collect(),
            AttackPattern::CycleAllActive
            | AttackPattern::FusionBombSpread
            | AttackPattern::AcidDrenchStack
            | AttackPattern::ThrivingPlagueSpread
            | AttackPattern::RadioactivitySpread
            | AttackPattern::DecayingStrikeFocus
            | AttackPattern::GrimShadowStack
            | AttackPattern::BlazingInfernoStack
            | AttackPattern::CelestialStatic => source_parts,
            AttackPattern::CycleParts(count) => source_parts.into_iter().take(*count).collect(),
            AttackPattern::RuinousRainFocus => source_parts,
            AttackPattern::WhipRuinousFocus => source_parts.into_iter().take(5).collect(),
            AttackPattern::CycleCursed => source_parts
                .into_iter()
                .filter(|part| boss.part(*part).part_state == PartState::Cursed)
                .collect(),
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

    fn can_target_untargetable_parts(&self, _deck: &[Card]) -> bool {
        match self {
            AttackPattern::CelestialStatic | AttackPattern::WhipRuinousFocus => true,
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

fn cycle_candidates(
    candidates: &[BossPartName],
    last_target: Option<BossPartName>,
) -> Option<BossPartName> {
    if candidates.is_empty() {
        return None;
    }

    match last_target {
        Some(last) => candidates
            .iter()
            .position(|part| *part == last)
            .and_then(|index| candidates.get((index + 1) % candidates.len()).copied())
            .or_else(|| candidates.first().copied()),
        None => candidates.first().copied(),
    }
}

pub fn generate_attack_patterns(sim_stats: &SimStats, deck: &[Card]) -> Vec<AttackPattern> {
    let mut patterns = Vec::new();

    for pattern in base_attack_patterns() {
        if pattern_is_available_for_deck(&pattern, deck)
            && pattern_is_allowed_for_deck(&pattern, sim_stats, deck)
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
        AttackPattern::CycleBody,
        AttackPattern::CycleArmor,
        AttackPattern::CycleAllActive,
        AttackPattern::CycleParts(2),
        AttackPattern::CycleParts(3),
        AttackPattern::CycleParts(4),
        AttackPattern::CycleParts(5),
        AttackPattern::CycleParts(6),

        //new
        AttackPattern::SingleCursed,
        AttackPattern::CycleCursed,

        // card specific patterns
        AttackPattern::FusionBombSpread,
        AttackPattern::AcidDrenchStack,
        AttackPattern::ThrivingPlagueSpread,
        AttackPattern::RadioactivitySpread,
        AttackPattern::DecayingStrikeFocus,
        AttackPattern::GrimShadowStack,
        AttackPattern::BlazingInfernoStack,
        AttackPattern::CelestialStatic,
        AttackPattern::RuinousRainFocus,
        AttackPattern::WhipRuinousFocus,
    ]
}

fn pattern_has_candidates(pattern: &AttackPattern, sim_stats: &SimStats, deck: &[Card]) -> bool {
    !pattern
        .candidate_parts(&sim_stats.boss_stat, deck, &sim_stats.attackable_part)
        .is_empty()
}

fn pattern_is_available_for_deck(pattern: &AttackPattern, deck: &[Card]) -> bool {
    match pattern {
        AttackPattern::FusionBombSpread => deck_has_card(deck, CardName::FusionBomb),
        AttackPattern::AcidDrenchStack => deck_has_card(deck, CardName::AcidDrench),
        AttackPattern::ThrivingPlagueSpread => deck_has_card(deck, CardName::ThrivingPlague),
        AttackPattern::RadioactivitySpread => deck_has_card(deck, CardName::Radioactivity),
        AttackPattern::DecayingStrikeFocus => deck_has_card(deck, CardName::DecayingStrike),
        AttackPattern::GrimShadowStack => deck_has_card(deck, CardName::GrimShadow),
        AttackPattern::BlazingInfernoStack => deck_has_card(deck, CardName::BlazingInferno),
        AttackPattern::CelestialStatic => deck_has_card(deck, CardName::CelestialStatic),
        AttackPattern::RuinousRainFocus => deck_has_card(deck, CardName::RuinousRain),
        AttackPattern::WhipRuinousFocus => {
            deck_has_card(deck, CardName::WhipOfLightning)
                && deck_has_card(deck, CardName::RuinousRain)
        }
        _ => true,
    }
}

fn pattern_is_allowed_for_deck(
    pattern: &AttackPattern,
    _sim_stats: &SimStats,
    deck: &[Card],
) -> bool {
    let burst_count = deck.iter().filter(|card| card.cardtype == CardType::Burst).count();
    let affliction_count = deck.iter().filter(|card| card.cardtype == CardType::Affliction).count();
    let true_support_count = deck.iter().filter(|card| card.cardtype == CardType::Support).count();
    let pseudo_support_count = deck.iter()
    .filter(|card| card.card_id == CardName::GuardBreak || card.card_id == CardName::Maelstrom)
    .count();
    let support_count = true_support_count + pseudo_support_count;

    //card
    let has_celestial_static = deck_has_card(deck, CardName::CelestialStatic);
    let has_electro_zap = deck_has_card(deck, CardName::ElectroZap) ;
    
    let mut is_allow = true;
    //support
    if deck_has_card(deck, CardName::GraspingVines){
        //not allow cycle torso
        if matches!(pattern, AttackPattern::CycleHeadTorso) 
        || matches!(pattern, AttackPattern::SingleTorso) 
        || matches!(pattern, AttackPattern::SingleHead) {
            is_allow = false;
        }
    }
    if deck_has_card(deck, CardName::SoulFire) || deck_has_card(deck, CardName::CrushingInstinct) {
        //not allow to attack limb
        if matches!(pattern, AttackPattern::SingleLimb) 
        || matches!(pattern, AttackPattern::CycleLimb) 
        {
            is_allow = false;
        }
    }
    if deck_has_card(deck, CardName::PrismaticRift) || deck_has_card(deck, CardName::SkeletalSmash) {
        //not allow cycle body
        if matches!(pattern, AttackPattern::CycleBody) || matches!(pattern, AttackPattern::SingleBody) {
            is_allow = false;
        }
    }
    if deck_has_card(deck, CardName::InspiringForce){
        //not allow cycle armor
        if matches!(pattern, AttackPattern::CycleArmor) || matches!(pattern, AttackPattern::SingleArmor) {
            is_allow = false;
        }
    }
    //totem of power disallow cycle pattern
    if deck_has_card(deck, CardName::TotemOfPower){
        if pattern_is_cycle_target(pattern)  {
            is_allow = false;
        }
    }
    //celestial
    if has_celestial_static && deck_has_only_celestial_static_and_supports(deck) {
        is_allow = matches!(pattern, AttackPattern::CelestialStatic);
        return  is_allow;
    }
    if has_celestial_static && pattern_is_single_target(pattern) {
        is_allow = false;
        return  is_allow;
    }
    if has_celestial_static && has_electro_zap{
        is_allow = false;
        return  is_allow;
    }
    
    //burst
        //have burst but no afflcition will ignore cycle pattern.
        //include electro zap as burst card
    if (burst_count > 0  && affliction_count == 0) || (affliction_count == 1 && has_electro_zap) { 
        //ignore purify, whip, chain, celestial, burst guard
        if deck_has_card(deck, CardName::PurifyingBlast) 
        || deck_has_card(deck, CardName::WhipOfLightning) 
        || deck_has_card(deck, CardName::ChainOfVengeance) 
        || deck_has_card(deck, CardName::CelestialStatic) 
        || deck_has_card(deck, CardName::GuardBreak){
            // is_allow = false;
        }else {
            if pattern_is_cycle_target(pattern) {
                is_allow = false;
            }
        }
    }
    if affliction_count== 0 && deck_has_card(deck, CardName::GuardBreak) || affliction_count == 1 && deck_has_card(deck, CardName::Maelstrom) {
        if pattern_is_cycle_target(pattern) {
            is_allow = false;
        }
    }
    
    if deck_has_card(deck, CardName::PurifyingBlast) && deck_has_card(deck, CardName::ElectroZap) {
        if pattern_is_cycle_target(pattern) {
            is_allow = false;
        }
    }
    return  is_allow;
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
fn pattern_is_cycle_target(pattern: &AttackPattern) -> bool {
    matches!(
        pattern,
        AttackPattern::CycleHeadTorso
            | AttackPattern::CycleLimb
            | AttackPattern::CycleBody
            | AttackPattern::CycleArmor
            | AttackPattern::CycleAllActive
            | AttackPattern::CycleCursed
            | AttackPattern::CycleParts(_)
    )
}

fn deck_has_only_celestial_static_and_supports(deck: &[Card]) -> bool {
    deck.len() == 3
        && deck_has_card(deck, CardName::CelestialStatic)
        && deck
            .iter()
            .filter(|card| card.cardtype == CardType::Support || card.card_id == CardName::GuardBreak || card.card_id == CardName::Maelstrom)
            .count() == 2 
}

fn deck_has_card(deck: &[Card], card_name: CardName) -> bool {
    deck.iter().any(|card| card.card_id == card_name)
}
