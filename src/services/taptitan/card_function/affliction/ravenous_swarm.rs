use rand::Rng;
use rand::seq::IndexedRandom;

use crate::models::{
    affliction::{Affliction, AfflictionKind},
    boss::{Boss, BossPartName, BossTickView, PartState},
    cards::Card,
};

use super::{AfflictionRemoveView, shared};

const TICK_INTERVAL_SECONDS: f64 = 0.2;

pub fn get_proc_chance(card: &Card, boss: &Boss) -> f64 {
    shared::get_proc_chance(card, boss)
    // 1.0
}
pub fn on_proc(
    card: &Card,
    boss: &mut Boss,
    target_part: BossPartName,
    damage: f64,
    rng: &mut impl Rng,
) {
    let Some(affliction) = shared::build_affliction(card, boss, target_part, damage, 0.0) else {
        return;
    };

    let apply_part = if is_ravenous_swarm_at_max_stacks(boss, target_part, affliction.max_stacks) {
        match random_spread_part(boss, target_part, affliction.max_stacks, rng) {
            Some(part) => part,
            None => return,
        }
    } else {
        target_part
    };

    let Some(mut affliction) = shared::build_affliction(card, boss, apply_part, damage, 0.0) else {
        return;
    };
    affliction.tick_interval_seconds = TICK_INTERVAL_SECONDS;

    boss.apply_affliction(apply_part, affliction);
}
pub fn on_tick(
    affliction: &Affliction,
    boss: &BossTickView,
    part_name: BossPartName,
    stack_multiplier: f64,
    elapsed_seconds: f64,
) -> u64 {
    shared::on_tick(
        affliction,
        boss,
        part_name,
        stack_multiplier,
        elapsed_seconds,
    )
}
pub fn on_remove(affliction: &AfflictionRemoveView, attached_duration: f64) -> u64 {
    shared::on_remove(affliction, attached_duration)
}

fn is_ravenous_swarm_at_max_stacks(boss: &Boss, part_name: BossPartName, max_stacks: u32) -> bool {
    boss.afflictions(part_name)
        .iter()
        .find(|affliction| affliction.kind == AfflictionKind::RavenousSwarmDebuff)
        .map(|affliction| affliction.stack_count() >= max_stacks as usize)
        .unwrap_or(false)
}

fn random_spread_part(
    boss: &Boss,
    target_part: BossPartName,
    max_stacks: u32,
    rng: &mut impl Rng,
) -> Option<BossPartName> {
    let mut body_parts = Vec::new();
    let mut armor_or_cursed_parts = Vec::new();

    for part_name in all_part_names() {
        if part_name == target_part || is_ravenous_swarm_at_max_stacks(boss, part_name, max_stacks)
        {
            continue;
        }

        match boss.part(part_name).part_state {
            PartState::Body => body_parts.push(part_name),
            PartState::Armor | PartState::Cursed => armor_or_cursed_parts.push(part_name),
            PartState::Skeleton => {}
        }
    }

    body_parts
        .choose(rng)
        .copied()
        .or_else(|| armor_or_cursed_parts.choose(rng).copied())
}

fn all_part_names() -> [BossPartName; 8] {
    [
        BossPartName::Head,
        BossPartName::Torso,
        BossPartName::LeftShoulder,
        BossPartName::RightShoulder,
        BossPartName::LeftHand,
        BossPartName::RightHand,
        BossPartName::LeftLeg,
        BossPartName::RightLeg,
    ]
}
