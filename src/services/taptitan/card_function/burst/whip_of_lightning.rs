use crate::models::{
    boss::{Boss, BossPartName},
    cards::Card,
};

use super::CardProcSnapshot;

pub fn roll_proc_chance(_card: &Card, boss: &Boss) -> f64 {
    let afflicted_parts = boss
        .parts()
        .into_iter()
        .filter(|part| !part.afflictions.is_empty())
        .count() as f64;

    (0.02 + (0.02 * afflicted_parts)).min(0.12)
}

pub fn on_proc(card: &Card, _boss: &mut Boss, _target_part: BossPartName) -> CardProcSnapshot {
    CardProcSnapshot {
        card_id: card.card_id,
        proc_chance: 0.02,
        damage_multiplier: 1.0,
        notes: vec![
            "Proc chance is boosted by the number of afflicted parts on the boss."
                .to_string(),
        ],
    }
}
