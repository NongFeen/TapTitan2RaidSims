use crate::models::{
    affliction::Affliction,
    boss::{Boss, BossPartName, BossTickView},
    cards::{Card, CardName},
    damage_source::DamageSource,
};

use super::AfflictionDamageEvent;

mod acid_drench;
mod amplify;
mod blazing_inferno;
mod corrosive_bubbles;
mod decaying_strike;
mod electro_zap;
mod fusion_bomb;
mod grim_shadow;
mod maelstrom;
mod radioactivity;
mod ravenous_swarm;
mod ruinous_rain;
mod sands_of_time;
mod shared;
mod thriving_plague;

#[derive(Clone, Copy)]
pub struct AfflictionRemoveView {
    pub source_card: CardName,
    pub remove_damage: f64,
    pub bonus_c: Option<f64>,
}

pub fn get_proc_chance(card: &Card, boss: &Boss) -> f64 {
    match card.card_id {
        CardName::BlazingInferno => blazing_inferno::get_proc_chance(card, boss),
        CardName::AcidDrench => acid_drench::get_proc_chance(card, boss),
        CardName::DecayingStrike => decaying_strike::get_proc_chance(card, boss),
        CardName::FusionBomb => fusion_bomb::get_proc_chance(card, boss),
        CardName::GrimShadow => grim_shadow::get_proc_chance(card, boss),
        CardName::ThrivingPlague => thriving_plague::get_proc_chance(card, boss),
        CardName::Radioactivity => radioactivity::get_proc_chance(card, boss),
        CardName::RavenousSwarm => ravenous_swarm::get_proc_chance(card, boss),
        CardName::RuinousRain => ruinous_rain::get_proc_chance(card, boss),
        CardName::CorrosiveBubbles => corrosive_bubbles::get_proc_chance(card, boss),
        CardName::Maelstrom => maelstrom::get_proc_chance(card, boss),
        CardName::Amplify => amplify::get_proc_chance(card, boss),
        CardName::SandsOfTime => sands_of_time::get_proc_chance(card, boss),
        CardName::ElectroZap => electro_zap::get_proc_chance(card, boss),
        _ => 0.0,
    }
}

pub fn on_proc(card: &Card, boss: &mut Boss, target_part: BossPartName, damage: f64) -> u64 {
    match card.card_id {
        CardName::BlazingInferno => blazing_inferno::on_proc(card, boss, target_part, damage),
        CardName::AcidDrench => acid_drench::on_proc(card, boss, target_part, damage),
        CardName::DecayingStrike => decaying_strike::on_proc(card, boss, target_part, damage),
        CardName::FusionBomb => fusion_bomb::on_proc(card, boss, target_part, damage),
        CardName::GrimShadow => grim_shadow::on_proc(card, boss, target_part, damage),
        CardName::ThrivingPlague => thriving_plague::on_proc(card, boss, target_part, damage),
        CardName::Radioactivity => radioactivity::on_proc(card, boss, target_part, damage),
        CardName::RavenousSwarm => ravenous_swarm::on_proc(card, boss, target_part, damage),
        CardName::RuinousRain => ruinous_rain::on_proc(card, boss, target_part, damage),
        CardName::CorrosiveBubbles => corrosive_bubbles::on_proc(card, boss, target_part, damage),
        CardName::Maelstrom => maelstrom::on_proc(card, boss, target_part, damage),
        CardName::Amplify => amplify::on_proc(card, boss, target_part, damage),
        CardName::SandsOfTime => sands_of_time::on_proc(card, boss, target_part, damage),
        CardName::ElectroZap => electro_zap::on_proc(card, boss, target_part, damage),
        _ => {}
    }
    return 0;
}

pub fn on_tick(
    affliction: &mut Affliction,
    boss: &BossTickView,
    part_name: BossPartName,
    elapsed_seconds: f64,
) -> Vec<AfflictionDamageEvent> {
    let mut events = Vec::new();
    let tick_interval_seconds = affliction.tick_interval_seconds.max(f64::EPSILON);
    let source_card = affliction.source_card;
    let remove_view = AfflictionRemoveView {
        source_card,
        remove_damage: affliction.remove_damage,
        bonus_c: affliction.source_skill.bonus_c,
    };

    affliction.tick_elapsed += elapsed_seconds;

    while affliction.tick_elapsed + f64::EPSILON >= tick_interval_seconds
        && affliction
            .stacks
            .iter()
            .any(|stack| stack.remaining_duration > 0.0)
    {
        affliction.tick_elapsed -= tick_interval_seconds;

        let tick_damage = affliction
            .stacks
            .iter()
            .filter(|stack| stack.remaining_duration > 0.0)
            .map(|stack| {
                tick_damage_for(
                    affliction,
                    boss,
                    part_name,
                    stack.damage_multiplier,
                    tick_interval_seconds,
                )
            })
            .fold(0u64, u64::saturating_add);

        if tick_damage > 0 {
            let _lowest_remaining = affliction
                .stacks
                .iter()
                .filter(|stack| stack.remaining_duration > 0.0)
                .map(|stack| stack.remaining_duration)
                .min_by(|left, right| left.total_cmp(right))
                .unwrap_or(0.0);

            // println!(
            //     "[AFF TICK] card={:?} part={:?} damage={} lowest_remaining={:.2}s tick_interval={:.3}s elapsed={:.3}s active_stacks={}",
            //     affliction.source_card,
            //     part_name,
            //     tick_damage,
            //     lowest_remaining,
            //     tick_interval_seconds,
            //     elapsed_seconds,
            //     affliction
            //         .stacks
            //         .iter()
            //         .filter(|stack| stack.remaining_duration > 0.0)
            //         .count(),
            // );

            events.push(AfflictionDamageEvent {
                part_name,
                damage: tick_damage,
                source: DamageSource::Card(affliction.source_card),
            });
        }
    }

    for stack in &mut affliction.stacks {
        stack.tick(elapsed_seconds);

        if stack.is_expired() {
            let remove_duration = match source_card {
                CardName::FusionBomb => stack.elapsed_attached_duration,
                _ => stack.attached_duration,
            };
            let remove_damage = remove_damage_for(&remove_view, remove_duration);
            if remove_damage > 0 {
                // println!(
                //     "[AFF REMOVE] card={:?} part={:?} damage={} attached={:.2}s total_attached={:.2}s elapsed={:.3}s stacks_before_remove={}",
                //     affliction.source_card,
                //     part_name,
                //     remove_damage,
                //     stack.attached_duration,
                //     stack.elapsed_attached_duration,
                //     elapsed_seconds,
                //     active_stack_count,
                // );

                events.push(AfflictionDamageEvent {
                    part_name,
                    damage: remove_damage,
                    source: DamageSource::Card(source_card),
                });
            }
        }
    }

    events
}

fn tick_damage_for(
    affliction: &Affliction,
    boss: &BossTickView,
    part_name: BossPartName,
    stack_multiplier: f64,
    elapsed_seconds: f64,
) -> u64 {
    match affliction.source_card {
        CardName::BlazingInferno => blazing_inferno::on_tick(
            affliction,
            boss,
            part_name,
            stack_multiplier,
            elapsed_seconds,
        ),
        CardName::AcidDrench => acid_drench::on_tick(
            affliction,
            boss,
            part_name,
            stack_multiplier,
            elapsed_seconds,
        ),
        CardName::DecayingStrike => decaying_strike::on_tick(
            affliction,
            boss,
            part_name,
            stack_multiplier,
            elapsed_seconds,
        ),
        CardName::FusionBomb => fusion_bomb::on_tick(
            affliction,
            boss,
            part_name,
            stack_multiplier,
            elapsed_seconds,
        ),
        CardName::GrimShadow => grim_shadow::on_tick(
            affliction,
            boss,
            part_name,
            stack_multiplier,
            elapsed_seconds,
        ),
        CardName::ThrivingPlague => thriving_plague::on_tick(
            affliction,
            boss,
            part_name,
            stack_multiplier,
            elapsed_seconds,
        ),
        CardName::Radioactivity => radioactivity::on_tick(
            affliction,
            boss,
            part_name,
            stack_multiplier,
            elapsed_seconds,
        ),
        CardName::RavenousSwarm => ravenous_swarm::on_tick(
            affliction,
            boss,
            part_name,
            stack_multiplier,
            elapsed_seconds,
        ),
        CardName::RuinousRain => ruinous_rain::on_tick(
            affliction,
            boss,
            part_name,
            stack_multiplier,
            elapsed_seconds,
        ),
        CardName::CorrosiveBubbles => corrosive_bubbles::on_tick(
            affliction,
            boss,
            part_name,
            stack_multiplier,
            elapsed_seconds,
        ),
        CardName::Maelstrom => maelstrom::on_tick(
            affliction,
            boss,
            part_name,
            stack_multiplier,
            elapsed_seconds,
        ),
        CardName::Amplify => amplify::on_tick(
            affliction,
            boss,
            part_name,
            stack_multiplier,
            elapsed_seconds,
        ),
        CardName::SandsOfTime => sands_of_time::on_tick(
            affliction,
            boss,
            part_name,
            stack_multiplier,
            elapsed_seconds,
        ),
        CardName::ElectroZap => electro_zap::on_tick(
            affliction,
            boss,
            part_name,
            stack_multiplier,
            elapsed_seconds,
        ),
        _ => 0,
    }
}

fn remove_damage_for(affliction: &AfflictionRemoveView, attached_duration: f64) -> u64 {
    match affliction.source_card {
        CardName::BlazingInferno => blazing_inferno::on_remove(affliction, attached_duration),
        CardName::AcidDrench => acid_drench::on_remove(affliction, attached_duration),
        CardName::DecayingStrike => decaying_strike::on_remove(affliction, attached_duration),
        CardName::FusionBomb => fusion_bomb::on_remove(affliction, attached_duration),
        CardName::GrimShadow => grim_shadow::on_remove(affliction, attached_duration),
        CardName::ThrivingPlague => thriving_plague::on_remove(affliction, attached_duration),
        CardName::Radioactivity => radioactivity::on_remove(affliction, attached_duration),
        CardName::RavenousSwarm => ravenous_swarm::on_remove(affliction, attached_duration),
        CardName::RuinousRain => ruinous_rain::on_remove(affliction, attached_duration),
        CardName::CorrosiveBubbles => corrosive_bubbles::on_remove(affliction, attached_duration),
        CardName::Maelstrom => maelstrom::on_remove(affliction, attached_duration),
        CardName::Amplify => amplify::on_remove(affliction, attached_duration),
        CardName::SandsOfTime => sands_of_time::on_remove(affliction, attached_duration),
        CardName::ElectroZap => electro_zap::on_remove(affliction, attached_duration),
        _ => 0,
    }
}
