use crate::models::{
    boss::{Boss, BossPartName},
    cards::Card,
};

use super::CardProcSnapshot;

pub fn on_proc(card: &Card, _boss: &mut Boss, _target_part: BossPartName) -> CardProcSnapshot {
    CardProcSnapshot {
        card_id: card.card_id,
        proc_chance: 0.12,
        damage_multiplier: 1.0,
        notes: vec![
            "Target selection should pick an armor part first, then reroute if needed."
                .to_string(),
        ],
    }
}
