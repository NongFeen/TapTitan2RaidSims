use crate::models::{
    boss::{Boss, BossPartName},
    card_skill_data::{card_skill_bonusamountC, card_skill_value_a},
    cards::Card,
    damage_source::DamageSource,
};

pub fn get_proc_chance(_card: &Card, _boss: &Boss) -> f64 {
    0.5
}
pub fn on_proc(card: &Card, boss: &mut Boss, target_part: BossPartName, damage: f64) {
    let consumed_stacks = {
        let target = boss.part_mut(target_part);
        let consumed_stacks = target
            .afflictions
            .iter()
            .map(|affliction| affliction.stack_count())
            .sum::<usize>();
        target.afflictions.clear();
        consumed_stacks
    };

    let purifying_mult = card_skill_value_a(card.card_id, card.level).unwrap_or(1.0);
    let purify_bonus = card_skill_bonusamountC(card.card_id).unwrap_or(1.0);
    let affliction_mult = 1.0 + (purify_bonus * consumed_stacks as f64);

    boss.on_hit_with_source(
        target_part,
        (damage * purifying_mult * affliction_mult).max(0.0) as u64,
        DamageSource::Card(card.card_id),
    );
}
