use crate::{models::{
    boss::{Boss, BossPartName}, card_skill_data::card_skill_value_a, cards::Card,
}, services::taptitan::card_function::burst::purifying_blast};

pub fn on_proc(
    card: &Card,
    boss: &mut Boss,
    target_part: BossPartName,
    damage: f64,
) -> f64 {
    let target = boss.part_mut(target_part);
    let had_affliction = !target.afflictions.is_empty();
    let affliction_count = 1.0; 
    if had_affliction{
        //TODO
        //let remove_count = target.remove affliction
        //affliction_count+= remove_count
    }
    let purifying_mult = card_skill_value_a(card.card_id, card.level).unwrap_or(1.0);
    return  damage *purifying_mult * affliction_count;
}
