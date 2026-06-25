use crate::models::{
    boss::{Boss, BossPartName}, cards::Card, damage_source::DamageSource,
};
pub fn get_proc_chance(_card: &Card, _boss: &Boss) -> f64 {
    0.12
}
pub fn on_proc(
    card: &Card,
    boss: &mut Boss,
    target_part: BossPartName,
    damage: f64,
    round_index: u32,
){
    let final_damage = if round_index == 2 { 
        damage * 1.35 
    } else { 
        damage 
    };
    boss.on_hit_with_source(
        target_part,
        final_damage.max(0.0).round() as u64,
        DamageSource::Card(card.card_id),
    );
}
