use crate::models::{
    boss::{Boss, BossPartName, PartState}, card_skill_data::card_skill_value_a, cards::Card, damage_source::DamageSource,
};
pub fn get_proc_chance(_card: &Card, _boss: &Boss) -> f64 {
    0.12
}

pub fn on_proc(
    card: &Card,
    boss: &mut Boss,
    target_part: BossPartName,
    damage: f64,
) {
    let flak_mult = card_skill_value_a(card.card_id, card.level).unwrap_or(1.0);
    let total_flak_damage = damage * flak_mult;
    let current_state = boss.get_state_from_part(target_part);

    boss.on_hit_with_source(
        target_part,
        (total_flak_damage).max(0.0) as u64,
        DamageSource::Card(card.card_id),
    );

    if current_state == PartState::Armor || current_state == PartState::Cursed{
        if let Some(random_body_part) = boss.get_random_body_part() {
            // Apply the ricochet damage directly to the random Body part
            boss.on_hit_with_source(
                random_body_part.part_name,
                total_flak_damage as u64,
                DamageSource::Card(card.card_id),
            );
        }
    }
}
