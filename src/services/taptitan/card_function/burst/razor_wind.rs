use crate::models::{
    boss::{Boss, BossPartName, PartState},
    cards::Card,
};

use super::CardProcSnapshot;

pub fn on_proc(card: &Card, boss: &mut Boss, target_part: BossPartName) -> CardProcSnapshot {
    let can_hit = matches!(boss.part(target_part).part_state, PartState::Body | PartState::Skeleton);

    CardProcSnapshot {
        card_id: card.card_id,
        proc_chance: 0.12,
        damage_multiplier: 1.5,
        notes: if can_hit {
            Vec::new()
        } else {
            vec!["Target is not in body state.".to_string()]
        },
    }
}
