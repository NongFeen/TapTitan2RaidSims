use crate::models::{
    boss::{Boss, BossPartName},
    cards::Card,
    damage_source::DamageSource,
};

pub fn get_proc_chance(_card: &Card, _boss: &Boss) -> f64 {
    // 1.0
    0.12
}
const MAX_TARGET: usize = 6;

pub fn on_proc(
    card: &mut Card, // 👈 Must be mutable to track queue changes
    boss: &mut Boss,
    target_part: BossPartName,
    damage: f64,
) {
    let chain_mult = card.skill.value_a.unwrap_or(1.0);
    let part_boost = card.skill.bonus_c.unwrap_or(1.0);

    if !card.chained_parts.contains(&target_part) {
        card.chained_parts.push(target_part);
    }

    if card.chained_parts.len() > MAX_TARGET {
        card.chained_parts.remove(0); // Removes index 0 (the oldest addition)
    }

    // 2. Damage Calculations
    let part_count = card.chained_parts.len() as f64;

    let total_damage = damage * chain_mult * part_boost.powf(part_count - 1.0); // start add part boost at 2nd parts affected

    let split_damage = total_damage / part_count;
    // 1 parts = 148549

    // println!("Chain of Ven : part = {}
    // mult : {}
    // part_boost {}
    // pow {}
    // Split : {}
    // total : {}"
    // ,part_count, chain_mult,part_boost,part_boost.powf(part_count),split_damage,total_damage
    // );
    for part in &card.chained_parts {
        boss.on_hit_with_source(
            *part,
            split_damage.max(0.0) as u64,
            DamageSource::Card(card.card_id),
        );
    }
}
