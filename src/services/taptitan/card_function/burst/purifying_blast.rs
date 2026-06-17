use crate::models::{
    boss::{Boss, BossPartName},
    cards::Card,
};

use super::CardProcSnapshot;

pub fn on_proc(card: &Card, boss: &mut Boss, target_part: BossPartName) -> CardProcSnapshot {
    let target = boss.part_mut(target_part);
    let had_affliction = !target.afflictions.is_empty();
    if had_affliction {
        target.afflictions.clear();
    }

    CardProcSnapshot {
        card_id: card.card_id,
        proc_chance: 0.12,
        damage_multiplier: if had_affliction { 2.0 } else { 1.0 },
        notes: vec![
            "Consumes afflictions on the target part to boost burst damage.".to_string(),
        ],
    }
}
