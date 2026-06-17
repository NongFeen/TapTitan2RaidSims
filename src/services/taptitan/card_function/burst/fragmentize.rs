use crate::models::{
    boss::{Boss, BossPartName, PartState},
    cards::Card,
};

use super::CardProcSnapshot;

pub fn on_proc(card: &Card, boss: &mut Boss, target_part: BossPartName) -> CardProcSnapshot {
    let damage_multiplier = match boss.part(target_part).part_state {
        PartState::Armor => 1.1,
        PartState::Cursed => 1.5,
        _ => 1.0,
    };

    CardProcSnapshot {
        card_id: card.card_id,
        proc_chance: 0.12,
        damage_multiplier,
        notes: Vec::new(),
    }
}
