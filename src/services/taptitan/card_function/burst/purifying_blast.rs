use crate::models::{
    boss::{Boss, BossPartName},
    cards::{Card, CardType},
    damage_source::DamageSource,
};

pub fn get_proc_chance(_card: &Card, _boss: &Boss) -> f64 {
    0.1
}
pub fn on_proc(card: &Card, boss: &mut Boss, target_part: BossPartName, damage: f64) {
    let consumed_stacks = {
        let target_afflictions = boss.afflictions_mut(target_part);
        let consumed_stacks = target_afflictions
            .iter()
            .filter(|affliction| affliction.source_card.card_type() == CardType::Affliction)
            .map(|affliction| affliction.stack_count())
            .sum::<usize>();
        target_afflictions
            .retain(|affliction| affliction.source_card.card_type() != CardType::Affliction);
        consumed_stacks
    };

    let purifying_mult = card.skill.value_a.unwrap_or(1.0);
    let purify_bonus = card.skill.bonus_c.unwrap_or(1.0);
    let affliction_mult = 1.0 + (purify_bonus * consumed_stacks as f64);

    boss.on_hit_with_source(
        target_part,
        (damage * purifying_mult * affliction_mult).max(0.0) as u64,
        DamageSource::Card(card.card_id),
    );
}
