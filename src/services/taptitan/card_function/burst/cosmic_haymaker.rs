use crate::models::{
    boss::{Boss, BossPartName},
    cards::Card,
};

use super::CardProcSnapshot;

pub fn roll_proc_chance(_card: &Card, _boss: &Boss) -> f64 {
    0.0
}

pub fn on_proc(card: &Card, _boss: &mut Boss, _target_part: BossPartName) -> CardProcSnapshot {
    CardProcSnapshot {
        card_id: card.card_id,
        proc_chance: 1.0,
        damage_multiplier: 1.0,
        notes: vec![
            "This card is tap-driven, so proc chance is forced by the simulator."
                .to_string(),
        ],
    }
}
