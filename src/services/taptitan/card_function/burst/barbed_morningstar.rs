use crate::models::{
    boss::{Boss, BossPartName, PartState},
    cards::Card,
    damage_source::DamageSource,
};

const DEFAULT_PROC_CHANCE: f64 = 0.12;
const DEFAULT_MAX_BONUS_PARTS: usize = 5;

pub fn get_proc_chance(card: &Card, _boss: &Boss) -> f64 {
    if card.skill.chance > 0.0 {
        card.skill.chance
    } else {
        DEFAULT_PROC_CHANCE
    }
}

pub fn on_proc(card: &Card, boss: &mut Boss, target_part: BossPartName, damage: f64) -> f64 {
    let burst_mult = card.skill.value_a.unwrap_or(1.0);
    let armor_damage_boost = card.skill.bonus_c.unwrap_or(0.0);
    let body_damage_boost = card.skill.bonus_d.unwrap_or(0.0);
    let max_bonus_parts = card
        .skill
        .bonus_e
        .map(|value| value.max(0.0) as usize)
        .unwrap_or(DEFAULT_MAX_BONUS_PARTS);
    let target_state = boss.get_state_from_part(target_part);
    let bonus_mult = match target_state {
        PartState::Armor | PartState::Cursed => {
            1.0 + armor_damage_boost * body_part_count(boss, max_bonus_parts) as f64
        }
        PartState::Body => 1.0 + body_damage_boost * armor_part_count(boss, max_bonus_parts) as f64,
        PartState::Skeleton => 1.0,
    };
    let result_damage = (damage * burst_mult * bonus_mult).max(0.0);

    boss.on_hit_with_source(target_part, result_damage, DamageSource::Card(card.card_id));
    result_damage
}

fn armor_part_count(boss: &Boss, max_bonus_parts: usize) -> usize {
    boss.parts()
        .iter()
        .filter(|part| matches!(part.part_state, PartState::Armor | PartState::Cursed))
        .count()
        .min(max_bonus_parts)
}

fn body_part_count(boss: &Boss, max_bonus_parts: usize) -> usize {
    boss.parts()
        .iter()
        .filter(|part| part.part_state == PartState::Body)
        .count()
        .min(max_bonus_parts)
}
