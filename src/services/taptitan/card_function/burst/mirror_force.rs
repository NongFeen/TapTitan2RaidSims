use crate::models::{
    boss::{Boss, BossPartName},
    cards::Card,
    damage_source::DamageSource,
};

pub fn get_proc_chance(card: &Card, _boss: &Boss) -> f64 {
    if !card.skill.has_row {
        return 0.0;
    }

    card.skill
        .chance
        .min(card.skill.max_chance.max(card.skill.chance))
}

pub fn on_proc(card: &Card, boss: &mut Boss, target_part: BossPartName, damage: f64, mirror_force_boost:u32) {
    let mirror_force_mult = card.skill.value_a.unwrap_or(1.0);
    let boost = 1.00 + ((mirror_force_boost as f64) / 100.00);
    // println!("Boost{}",boost);
    boss.on_hit_with_source(
        target_part,
        (damage * mirror_force_mult * boost).max(0.0) as u64,
        DamageSource::Card(card.card_id),
    );
}
