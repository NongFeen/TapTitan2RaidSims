use crate::models::{
    boss::{Boss, BossPartName, PartState}, card_skill_data::card_skill_value_a, cards::Card, damage_source::DamageSource,
};
pub fn get_proc_chance(_card: &Card, _boss: &Boss) -> f64 {
    1.0
    //0.12
}
const MAX_STACK:usize = 100;
const STACK_USE:usize = 8;
const STACK_GAIN:usize = 1;

pub fn on_proc(
    card: &mut Card,
    boss: &mut Boss,
    target_part: BossPartName,
    damage: f64,
) {
    let celes_mult = card_skill_value_a(card.card_id, card.level).unwrap_or(1.0);
    let current_state = boss.get_state_from_part(target_part);

    if target_part.is_limb() {
        if card.celestial_stacks < MAX_STACK {
            card.celestial_stacks += STACK_GAIN;
        }
    } 
    else if current_state != PartState::Skeleton && card.celestial_stacks >= STACK_USE {
        card.celestial_stacks -= STACK_USE;
        
        let final_damage = damage * celes_mult;

        boss.on_hit_with_source(
            target_part,
            final_damage.max(0.0).round() as u64,
            DamageSource::Card(card.card_id),
        );
    }
}
