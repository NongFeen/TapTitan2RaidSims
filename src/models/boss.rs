use rand::seq::IndexedRandom;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, sync::Arc};
use strum_macros::{EnumIter, EnumString};

use crate::models::affliction::{Affliction, AfflictionKind};
use crate::models::cards::{CardName, CardType};
use crate::models::damage_source::DamageSource;
use crate::models::player_raid_data::PlayerRaidData;
use crate::models::support_modifier::SupportModifiers;
use crate::services::taptitan::card_function;

mod afflictions;
mod boss;
mod damage_cache;
mod parts;

pub use afflictions::BossAfflictions;
pub use boss::Boss;
pub(crate) use damage_cache::BossDamageMultiplierCache;
use damage_cache::*;
pub use parts::{
    BossName, BossPart, BossPartName, BossTickView, CurseType, DamageResult, GlobalRaidModifier,
    PartState,
};
