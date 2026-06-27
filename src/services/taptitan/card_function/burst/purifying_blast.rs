use crate::models::{
    boss::{Boss, BossPartName},
    card_skill_data::card_skill_value_a,
    cards::Card,
    damage_source::DamageSource,
};

pub fn get_proc_chance(_card: &Card, _boss: &Boss) -> f64 {
    0.1
}
pub fn on_proc(card: &Card, boss: &mut Boss, target_part: BossPartName, damage: f64) {
    let target = boss.part_mut(target_part);
    let had_affliction = !target.afflictions.is_empty();
    let affliction_count = 1.0;
    if had_affliction {
        //TODO
        //let remove_count = target.remove affliction
        //affliction_count+= remove_count
    }
    let purifying_mult = card_skill_value_a(card.card_id, card.level).unwrap_or(1.0);

    boss.on_hit_with_source(
        target_part,
        (damage * purifying_mult * affliction_count).max(0.0) as u64,
        DamageSource::Card(card.card_id),
    );
}
