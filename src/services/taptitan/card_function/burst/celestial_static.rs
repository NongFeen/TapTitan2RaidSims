use crate::models::{
    boss::{Boss, BossPartName},
    cards::Card,
};

use super::CardProcSnapshot;

pub fn on_proc(card: &Card, _boss: &mut Boss, _target_part: BossPartName) -> CardProcSnapshot {
    CardProcSnapshot {
        card_id: card.card_id,
        proc_chance: 0.0,
        damage_multiplier: 1.0,
        notes: vec![
            "Needs a stack bank in the simulator; limbs build stacks and non-limbs consume them."
                .to_string(),
        ],
    }
}
