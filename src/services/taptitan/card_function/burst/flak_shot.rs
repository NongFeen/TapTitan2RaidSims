use crate::models::{
    boss::{Boss, BossPartName, PartState},
    cards::Card,
    damage_source::DamageSource,
};
pub fn get_proc_chance(_card: &Card, _boss: &Boss) -> f64 {
    0.12
}

pub fn on_proc(card: &Card, boss: &mut Boss, target_part: BossPartName, damage: f64) -> f64 {
    let flak_mult = card.skill.value_a.unwrap_or(1.0);
    let total_flak_damage = (damage * flak_mult).max(0.0);
    let current_state = boss.get_state_from_part(target_part);

    boss.on_hit_with_source(
        target_part,
        total_flak_damage,
        DamageSource::Card(card.card_id),
    );

    if current_state == PartState::Armor || current_state == PartState::Cursed {
        if let Some(random_body_part) = boss.get_random_body_part() {
            let ricochet_raw_damage = boss.player_raid_data.as_ref().map_or(damage, |data| {
                let target_part_add = data.get_total_part_state_add(target_part, current_state);
                let ricochet_part_add =
                    data.get_total_part_state_add(random_body_part.part_name, PartState::Body);
                // println!(
                //     "[DEBUG]flakshot ricochet damage {} target_part_add {} ricochet_part_add {}",
                //     damage - f64::from(target_part_add) + f64::from(ricochet_part_add),
                //     target_part_add,
                //     ricochet_part_add
                // );
                damage - f64::from(target_part_add) + f64::from(ricochet_part_add)
            });
            let ricochet_flak_damage = (ricochet_raw_damage * flak_mult).max(0.0);

            let final_damage = boss.preview_damage_with_source(
                random_body_part.part_name,
                ricochet_flak_damage,
                &DamageSource::Card(card.card_id),
            );
            boss.record_damage(DamageSource::Card(card.card_id), final_damage);
            boss.on_hit(random_body_part.part_name, final_damage);
        }
    }
    total_flak_damage
}
