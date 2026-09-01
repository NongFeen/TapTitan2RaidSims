use super::attack_pattern::{
    AttackPattern, generate_all_attack_patterns, generate_attack_patterns,
};
use super::card_function::support::totem_of_power::{self, PendingTotem};
use super::csv::deck_pair_rules;
use crate::models::boss::{Boss, BossPartName, GlobalRaidModifier, PartState};
use crate::models::cards::{Card, CardName, CardType};
use crate::models::damage_source::DamageSource;
use crate::models::player_raid_data::PlayerRaidData;
use crate::models::sim_payload::SimPayLoad;
use crate::models::support_modifier::SupportModifiers;
use itertools::Itertools;
use rand::{Rng, SeedableRng};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering as AtomicOrdering},
};
use strum::IntoEnumIterator;

mod deck_rules;
mod deterministic_rng;
mod fast_runner;
mod helpers;
mod proc_cache;
mod runner;
mod types;

use deck_rules::*;
use deterministic_rng::SimRng;
use helpers::*;
use proc_cache::*;
pub use proc_cache::{PreDeterminedProc, ProcScenario, configure_sim_worker_count};
use types::*;
pub use types::{
    SIMS_ROUNDS, SimCardDamageResult, SimDeckResult, SimPatternResult, SimProgress, SimRunResult,
    SimStats, SimulationPhase,
};

/// Public wrapper around `helpers::format_compact` -- that module stays
/// private, but the persisted-deck-result codec (outside this module tree)
/// needs to regenerate the same `_display` formatting when reconstructing a
/// full `SimDeckResult` from a narrowed, persisted row.
pub fn format_compact(damage: u64) -> String {
    helpers::format_compact(damage)
}

//release version 20R all cards 2m 1.56 sec
pub struct SimService;

#[cfg(test)]
#[path = "../../../tests/unit/services/taptitan/sim_service/sim_to_real_tests.rs"]
mod sim_to_real_tests;
