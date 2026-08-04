use super::attack_pattern::{AttackPattern, generate_attack_patterns};
use super::card_function::support::totem_of_power::{self, PendingTotem};
use super::csv::deck_pair_rules;
use crate::models::boss::{Boss, BossPartName, GlobalRaidModifier, PartState};
use crate::models::cards::{Card, CardName, CardType};
use crate::models::damage_source::DamageSource;
use crate::models::player_raid_data::PlayerRaidData;
use crate::models::sim_payload::SimPayLoad;
use crate::models::support_modifier::SupportModifiers;
use itertools::Itertools;
use rand::random;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering as AtomicOrdering},
};
use strum::IntoEnumIterator;

mod deck_rules;
mod fast_runner;
mod helpers;
mod proc_cache;
mod runner;
mod types;

use deck_rules::*;
use helpers::*;
use proc_cache::*;
pub use proc_cache::{PreDeterminedProc, ProcScenario};
use types::*;
pub use types::{
    SimCardDamageResult, SimDeckResult, SimPatternResult, SimProgress, SimRunResult, SimStats,
};

//release version 20R all cards 2m 1.56 sec
pub struct SimService;
