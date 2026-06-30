use rand::Rng;

use crate::models::{
    affliction::{Affliction, AfflictionKind},
    boss::{Boss, BossPartName, PartState},
    card_skill_data::{card_skill_bonusamountC, card_skill_bonusamountD},
    cards::{Card, CardName},
};

const TICKS_PER_SECOND: u32 = 20;
const HEAD_TORSO_MIN_TRAVEL_TICKS: u32 = 40;
const HEAD_TORSO_MAX_TRAVEL_TICKS: u32 = 70;
const LIMB_MIN_TRAVEL_TICKS: u32 = 30;
const LIMB_MAX_TRAVEL_TICKS: u32 = 80;
const TAP_WINDOW_TICKS: u32 = 10;
const HAYMAKER_MAX_CHARGES: u16 = 70;
const HAYMAKER_SAVE_TICKS: u16 = 40;
const HAYMAKER_SAVE_TICKS_WITH_ECHO: u16 = 48;

#[derive(Debug, Clone)]
pub struct PendingTotem {
    pub target_part: BossPartName,
    pub earliest_tap_tick: u32,
    pub latest_tap_tick: u32,
}

pub fn try_spawn(
    pending_totems: &mut Vec<PendingTotem>,
    totem_card: &Card,
    boss: &Boss,
    target_part: BossPartName,
    current_tick: u32,
    next_spawn_tick: &mut f64,
) {
    if totem_card.card_id != CardName::TotemOfPower {
        return;
    }

    if (current_tick as f64) + f64::EPSILON < *next_spawn_tick {
        return;
    }

    *next_spawn_tick += spawn_interval_ticks();

    if boss.part(target_part).part_state == PartState::Skeleton {
        return;
    }

    let mut rng = rand::rng();
    let travel_ticks = travel_ticks_for_part(target_part, &mut rng);
    let land_tick = current_tick.saturating_add(travel_ticks);

    pending_totems.push(PendingTotem {
        target_part,
        earliest_tap_tick: land_tick.saturating_sub(TAP_WINDOW_TICKS),
        latest_tap_tick: land_tick.saturating_add(TAP_WINDOW_TICKS),
    });
}

pub fn first_spawn_tick() -> f64 {
    spawn_interval_ticks()
}

pub fn update(
    pending_totems: &mut Vec<PendingTotem>,
    totem_card: &Card,
    deck: &[Card],
    boss: &mut Boss,
    current_tick: u32,
) {
    if totem_card.card_id != CardName::TotemOfPower {
        return;
    }

    let should_save_for_haymaker = deck
        .iter()
        .any(|card| card.card_id == CardName::CosmicHaymaker);
    let haymaker_ready_window = haymaker_ready_window(deck);

    let mut index = 0;
    while index < pending_totems.len() {
        let totem = &pending_totems[index];
        let is_tappable = current_tick >= totem.earliest_tap_tick;
        let must_tap_now = current_tick >= totem.latest_tap_tick;
        let should_tap_for_haymaker = should_save_for_haymaker
            && haymaker_stacks_remaining(deck)
                .map(|remaining| remaining <= haymaker_ready_window)
                .unwrap_or(false);
        let should_tap = is_tappable && (!should_save_for_haymaker || should_tap_for_haymaker);

        if should_tap || must_tap_now {
            let target_part = pending_totems.swap_remove(index).target_part;
            apply_debuff(boss, target_part, totem_card);
            continue;
        }

        index += 1;
    }
}

fn apply_debuff(boss: &mut Boss, target_part: BossPartName, card: &Card) {
    if boss.part(target_part).part_state == PartState::Skeleton {
        return;
    }

    let duration = card_skill_bonusamountC(CardName::TotemOfPower).unwrap_or(2.0);
    boss.apply_affliction(
        target_part,
        Affliction::new(
            AfflictionKind::TotemOfPowerDebuff,
            card.card_id,
            card.level,
            1,
            duration,
            0.0,
            0.0,
            1.0,
            u32::MAX,
        ),
    );
}

fn haymaker_ready_window(deck: &[Card]) -> u16 {
    if deck.iter().any(|card| card.card_id == CardName::AstralEcho) {
        HAYMAKER_SAVE_TICKS_WITH_ECHO
    } else {
        HAYMAKER_SAVE_TICKS
    }
}

fn haymaker_stacks_remaining(deck: &[Card]) -> Option<u16> {
    let haymaker = deck
        .iter()
        .find(|card| card.card_id == CardName::CosmicHaymaker)?;
    Some(HAYMAKER_MAX_CHARGES.saturating_sub(haymaker.tap_count))
}

fn spawn_interval_ticks() -> f64 {
    let spawn_interval_seconds = card_skill_bonusamountD(CardName::TotemOfPower)
        .unwrap_or(0.66)
        .max(f64::EPSILON);
    spawn_interval_seconds * TICKS_PER_SECOND as f64
}

fn travel_ticks_for_part(target_part: BossPartName, rng: &mut impl Rng) -> u32 {
    if target_part.is_limb() {
        rng.random_range(LIMB_MIN_TRAVEL_TICKS..=LIMB_MAX_TRAVEL_TICKS)
    } else {
        rng.random_range(HEAD_TORSO_MIN_TRAVEL_TICKS..=HEAD_TORSO_MAX_TRAVEL_TICKS)
    }
}
