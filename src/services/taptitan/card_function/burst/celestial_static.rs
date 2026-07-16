use crate::models::{
    boss::{Boss, BossPartName, PartState},
    cards::Card,
    damage_source::DamageSource,
};
pub fn get_proc_chance(_card: &Card, _boss: &Boss) -> f64 {
    1.0
}
const MAX_STACK: usize = 100;
const STACK_USE: usize = 8;
const STACK_GAIN: usize = 1;

pub fn on_proc(card: &mut Card, boss: &mut Boss, target_part: BossPartName, damage: f64) -> u64 {
    let celes_mult = card.skill.value_a.unwrap_or(1.0);
    let current_state = boss.get_state_from_part(target_part);
    let final_damage = (damage * celes_mult) as u64;
    if target_part.is_limb() {
        if card.celestial_stacks < MAX_STACK {
            card.celestial_stacks += STACK_GAIN;
        }
    } else if current_state != PartState::Skeleton && card.celestial_stacks >= STACK_USE {
        card.celestial_stacks -= STACK_USE;

        boss.on_hit_with_source(target_part, final_damage, DamageSource::Card(card.card_id));
    }
    final_damage
}
