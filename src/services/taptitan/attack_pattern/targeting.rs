use super::*;

pub(super) fn single_part_candidates(
    source_parts: &[BossPartName],
    target: BossPartName,
) -> Vec<BossPartName> {
    if source_parts.contains(&target) {
        vec![target]
    } else {
        Vec::new()
    }
}

pub(super) fn lowest_stack_duration(affliction: &Affliction) -> f64 {
    affliction
        .stacks
        .iter()
        .map(|stack| stack.remaining_duration)
        .min_by(|left, right| left.total_cmp(right))
        .unwrap_or(0.0)
}

pub(super) fn is_better_refresh_target(
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

pub(super) fn cycle_filtered_candidates(
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

pub(super) fn part_is_active(boss: &Boss, part: BossPartName) -> bool {
    boss.part(part).part_state != PartState::Skeleton
}

pub(super) fn first_active_part(boss: &Boss, parts: &[BossPartName]) -> Option<BossPartName> {
    parts
        .iter()
        .copied()
        .find(|part| part_is_active(boss, *part))
}

pub(super) fn cycle_active_parts(
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

pub(super) fn cycle_first_active_parts(
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

pub(super) fn part_passes_support_target_rules(
    boss: &Boss,
    deck: &[Card],
    part: BossPartName,
) -> bool {
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
