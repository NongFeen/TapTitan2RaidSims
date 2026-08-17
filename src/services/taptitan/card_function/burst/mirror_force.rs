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

pub fn on_proc(
    card: &Card,
    boss: &mut Boss,
    target_part: BossPartName,
    damage: f64,
    mirror_force_boost: f64,
) -> f64 {
    let mirror_force_mult = card.skill.value_a.unwrap_or(1.0);
    let boost = clan_boost_multiplier(mirror_force_boost);
    // println!("Boost{}",boost);
    let result_damage = (damage * mirror_force_mult * boost).max(0.0);
    boss.on_hit_with_source(target_part, result_damage, DamageSource::Card(card.card_id));
    result_damage
}

fn clan_boost_multiplier(mirror_force_boost: f64) -> f64 {
    1.0 + mirror_force_boost.max(0.0)
}

#[cfg(test)]
mod tests {
    use super::clan_boost_multiplier;

    #[test]
    fn fractional_clan_boost_maps_to_the_expected_multiplier() {
        assert!((clan_boost_multiplier(0.0) - 1.0).abs() < f64::EPSILON);
        assert!((clan_boost_multiplier(0.35) - 1.35).abs() < f64::EPSILON);
    }
}
