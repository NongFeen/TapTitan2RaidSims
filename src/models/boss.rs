use rand::seq::IndexedRandom;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, sync::Arc};
use strum_macros::{EnumIter, EnumString};

use crate::models::affliction::{Affliction, AfflictionKind};
use crate::models::cards::CardName;
use crate::models::damage_source::DamageSource;
use crate::models::player_raid_data::PlayerRaidData;
use crate::models::support_modifier::SupportModifiers;
use crate::services::taptitan::card_function;

#[derive(
    Debug,
    Deserialize,
    Serialize,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    Clone,
    Copy,
    EnumIter,
    EnumString,
)]
pub enum BossPartName {
    Head,
    Torso,
    LeftShoulder,
    RightShoulder,
    LeftHand,
    RightHand,
    LeftLeg,
    RightLeg,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
pub enum PartState {
    Cursed,
    Armor,
    Body,
    Skeleton,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
pub enum BossName {
    Lojak,
    Takedar,
    Jukk,
    Sterl,
    Mohaca,
    Terro,
    Klonk,
    Priker,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DamageResult {
    pub source: DamageSource,
    pub damage: u64,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct BossPart {
    pub part_name: BossPartName,
    pub part_state: PartState,
    pub max_armor: u64,
    pub max_health: u64,
    pub current_armor: u64,
    pub current_health: u64,
    #[serde(default)]
    pub radioactivity_afflicted_seconds: f64,
}

pub struct BossTickView<'a> {
    pub head: &'a BossPart,
    pub torso: &'a BossPart,
    pub left_shoulder: &'a BossPart,
    pub right_shoulder: &'a BossPart,
    pub left_hand: &'a BossPart,
    pub right_hand: &'a BossPart,
    pub left_leg: &'a BossPart,
    pub right_leg: &'a BossPart,
    thriving_plague_part_count: usize,
}

impl BossTickView<'_> {
    pub fn part(&self, part_name: BossPartName) -> &BossPart {
        match part_name {
            BossPartName::Head => self.head,
            BossPartName::Torso => self.torso,
            BossPartName::LeftShoulder => self.left_shoulder,
            BossPartName::RightShoulder => self.right_shoulder,
            BossPartName::LeftHand => self.left_hand,
            BossPartName::RightHand => self.right_hand,
            BossPartName::LeftLeg => self.left_leg,
            BossPartName::RightLeg => self.right_leg,
        }
    }

    pub fn parts(&self) -> [&BossPart; 8] {
        [
            self.head,
            self.torso,
            self.left_shoulder,
            self.right_shoulder,
            self.left_hand,
            self.right_hand,
            self.left_leg,
            self.right_leg,
        ]
    }

    pub fn thriving_plague_part_count(&self) -> usize {
        self.thriving_plague_part_count
    }
}

impl BossPart {
    pub fn new(
        part: BossPartName,
        state: PartState,
        m_armor: u64,
        m_health: u64,
        c_armor: u64,
        c_health: u64,
    ) -> Self {
        Self {
            part_name: part,
            part_state: state,
            max_armor: m_armor,
            max_health: m_health,
            current_armor: c_armor,
            current_health: c_health,
            radioactivity_afflicted_seconds: 0.0,
        }
    }
    pub fn is_limb(&self) -> bool {
        self.part_name.is_limb()
    }
    pub fn on_hit(&mut self, damage: u64) {
        match self.part_state {
            PartState::Armor | PartState::Cursed => {
                self.current_armor = self.current_armor.saturating_sub(damage);
                //ignore leftover damage
                if self.current_armor == 0 {
                    self.part_state = PartState::Body;
                }
            }

            PartState::Body => {
                self.current_health = self.current_health.saturating_sub(damage);

                if self.current_health == 0 {
                    self.part_state = PartState::Skeleton;
                }
            }
            PartState::Skeleton => {} //do nothing
        }
    }
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct BossAfflictions {
    #[serde(default)]
    pub head: Vec<Affliction>,
    #[serde(default)]
    pub torso: Vec<Affliction>,
    #[serde(default)]
    pub left_shoulder: Vec<Affliction>,
    #[serde(default)]
    pub right_shoulder: Vec<Affliction>,
    #[serde(default)]
    pub left_hand: Vec<Affliction>,
    #[serde(default)]
    pub right_hand: Vec<Affliction>,
    #[serde(default)]
    pub left_leg: Vec<Affliction>,
    #[serde(default)]
    pub right_leg: Vec<Affliction>,
}

impl BossAfflictions {
    pub fn part(&self, part_name: BossPartName) -> &[Affliction] {
        match part_name {
            BossPartName::Head => &self.head,
            BossPartName::Torso => &self.torso,
            BossPartName::LeftShoulder => &self.left_shoulder,
            BossPartName::RightShoulder => &self.right_shoulder,
            BossPartName::LeftHand => &self.left_hand,
            BossPartName::RightHand => &self.right_hand,
            BossPartName::LeftLeg => &self.left_leg,
            BossPartName::RightLeg => &self.right_leg,
        }
    }

    pub fn part_mut(&mut self, part_name: BossPartName) -> &mut Vec<Affliction> {
        match part_name {
            BossPartName::Head => &mut self.head,
            BossPartName::Torso => &mut self.torso,
            BossPartName::LeftShoulder => &mut self.left_shoulder,
            BossPartName::RightShoulder => &mut self.right_shoulder,
            BossPartName::LeftHand => &mut self.left_hand,
            BossPartName::RightHand => &mut self.right_hand,
            BossPartName::LeftLeg => &mut self.left_leg,
            BossPartName::RightLeg => &mut self.right_leg,
        }
    }

    fn parts(&self) -> [&Vec<Affliction>; 8] {
        [
            &self.head,
            &self.torso,
            &self.left_shoulder,
            &self.right_shoulder,
            &self.left_hand,
            &self.right_hand,
            &self.left_leg,
            &self.right_leg,
        ]
    }

    fn apply(&mut self, part_name: BossPartName, affliction: Affliction) {
        let afflictions = self.part_mut(part_name);
        let incoming_stack_count = affliction.stack_count() as u32;
        let incoming_duration = affliction
            .stacks
            .first()
            .map(|stack| stack.attached_duration)
            .unwrap_or(0.0);
        let incoming_tick_damage = affliction.damage_per_second;
        let incoming_expire_damage = affliction.remove_damage;
        let incoming_tick_interval = affliction.tick_interval_seconds;
        let incoming_sands_of_time_boosted = affliction
            .stacks
            .iter()
            .any(|stack| stack.sands_of_time_boosted);

        if let Some(existing) = afflictions
            .iter_mut()
            .find(|current| current.kind == affliction.kind)
        {
            existing.apply_stacks(
                incoming_stack_count,
                incoming_duration,
                incoming_tick_damage,
                incoming_expire_damage,
                incoming_tick_interval,
                incoming_sands_of_time_boosted,
            );
            return;
        }

        afflictions.push(affliction);
    }

    fn is_empty(&self) -> bool {
        self.parts()
            .iter()
            .all(|afflictions| afflictions.is_empty())
    }

    fn has_active_kind(&self, part_name: BossPartName, kind: AfflictionKind) -> bool {
        self.part(part_name).iter().any(|affliction| {
            affliction.kind == kind
                && affliction
                    .stacks
                    .iter()
                    .any(|stack| stack.remaining_duration > 0.0)
        })
    }

    fn has_part_damage_taken_debuff(&self) -> bool {
        self.parts().iter().any(|afflictions| {
            afflictions.iter().any(|affliction| {
                is_part_damage_taken_debuff(affliction.kind)
                    && affliction
                        .stacks
                        .iter()
                        .any(|stack| stack.remaining_duration > 0.0)
            })
        })
    }

    fn has_active_kind_anywhere(&self, kind: AfflictionKind) -> bool {
        self.parts().iter().any(|afflictions| {
            afflictions.iter().any(|affliction| {
                affliction.kind == kind
                    && affliction
                        .stacks
                        .iter()
                        .any(|stack| stack.remaining_duration > 0.0)
            })
        })
    }

    fn thriving_plague_part_count(&self) -> usize {
        self.parts()
            .iter()
            .filter(|afflictions| {
                afflictions
                    .iter()
                    .any(|affliction| affliction.kind == AfflictionKind::ThrivingPlagueDebuff)
            })
            .count()
    }

    fn tick_afflictions(
        &mut self,
        boss: &BossTickView,
        elapsed_seconds: f64,
    ) -> Vec<card_function::AfflictionDamageEvent> {
        let mut damage_events = Vec::new();

        for part_name in BossPartName::all() {
            for affliction in self.part_mut(part_name) {
                damage_events.extend(card_function::tick_affliction(
                    affliction,
                    boss,
                    part_name,
                    elapsed_seconds,
                ));
            }
        }

        damage_events
    }

    fn remove_expired(&mut self) {
        for afflictions in [
            &mut self.head,
            &mut self.torso,
            &mut self.left_shoulder,
            &mut self.right_shoulder,
            &mut self.left_hand,
            &mut self.right_hand,
            &mut self.left_leg,
            &mut self.right_leg,
        ] {
            for affliction in afflictions.iter_mut() {
                affliction.remove_expired_stacks();
            }

            afflictions.retain(|affliction| !affliction.is_expired());
        }
    }
}

impl BossPartName {
    pub fn all() -> [BossPartName; 8] {
        [
            BossPartName::Head,
            BossPartName::Torso,
            BossPartName::LeftShoulder,
            BossPartName::RightShoulder,
            BossPartName::LeftHand,
            BossPartName::RightHand,
            BossPartName::LeftLeg,
            BossPartName::RightLeg,
        ]
    }

    pub fn is_limb(&self) -> bool {
        match self {
            BossPartName::Head | BossPartName::Torso => false,
            _ => true, // Everything else (shoulders, hands, legs) are limbs
        }
    }
}

fn part_state_bits(state: PartState) -> u8 {
    match state {
        PartState::Cursed => 0,
        PartState::Armor => 1,
        PartState::Body => 2,
        PartState::Skeleton => 3,
    }
}

fn is_part_damage_taken_debuff(kind: AfflictionKind) -> bool {
    matches!(
        kind,
        AfflictionKind::GuardBreakDebuff
            | AfflictionKind::MaelstromDebuff
            | AfflictionKind::TotemOfPowerDebuff
    )
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct BossDamageMultiplierCache {
    ready: bool,
    raid_all_mult: f32,
    boss_mult: f32,
    part_mult: [f32; 8],
    state_mult: [f32; 4],
}

impl Default for BossDamageMultiplierCache {
    fn default() -> Self {
        Self {
            ready: false,
            raid_all_mult: 1.0,
            boss_mult: 1.0,
            part_mult: [1.0; 8],
            state_mult: [1.0; 4],
        }
    }
}

impl BossDamageMultiplierCache {
    fn from_player(player_raid_data: &PlayerRaidData, boss_name: BossName) -> Self {
        let jade_set = if player_raid_data.raid_set.jade_anniversary {
            0.04
        } else {
            0.0
        };

        let mut cache = Self {
            ready: true,
            raid_all_mult: 1.0 + jade_set + player_raid_data.title,
            boss_mult: 1.0
                + player_raid_data
                    .titan_soul_research
                    .get_boss_mult(boss_name),
            part_mult: [1.0; 8],
            state_mult: [1.0; 4],
        };

        for part_name in BossPartName::all() {
            cache.part_mult[part_name_index(part_name)] = 1.0
                + player_raid_data
                    .titan_soul_research
                    .get_part_mult(part_name);
        }

        for state in [
            PartState::Cursed,
            PartState::Armor,
            PartState::Body,
            PartState::Skeleton,
        ] {
            cache.state_mult[part_state_index(state)] =
                1.0 + player_raid_data.titan_soul_research.get_state_mult(state);
        }

        cache
    }
}

fn part_name_index(part_name: BossPartName) -> usize {
    match part_name {
        BossPartName::Head => 0,
        BossPartName::Torso => 1,
        BossPartName::LeftShoulder => 2,
        BossPartName::RightShoulder => 3,
        BossPartName::LeftHand => 4,
        BossPartName::RightHand => 5,
        BossPartName::LeftLeg => 6,
        BossPartName::RightLeg => 7,
    }
}

fn part_state_index(state: PartState) -> usize {
    match state {
        PartState::Cursed => 0,
        PartState::Armor => 1,
        PartState::Body => 2,
        PartState::Skeleton => 3,
    }
}
#[derive(Debug, Clone, Deserialize, Serialize)]

pub struct Boss {
    pub boss_name: BossName,
    pub head: BossPart,
    pub torso: BossPart,
    pub left_shoulder: BossPart,
    pub right_shoulder: BossPart,
    pub left_hand: BossPart,
    pub right_hand: BossPart,
    pub left_leg: BossPart,
    pub right_leg: BossPart,
    #[serde(skip, default)]
    pub afflictions: BossAfflictions,
    #[serde(default)]
    pub damage_results: Vec<DamageResult>,
    #[serde(skip, default)]
    pub total_damage: u64,
    #[serde(skip, default)]
    pub tap_damage_total: u64,
    #[serde(skip, default)]
    pub card_damage_totals: HashMap<CardName, u64>,
    #[serde(skip, default)]
    pub(crate) part_damage_taken_debuffs_present: bool,
    #[serde(skip, default)]
    pub(crate) radioactivity_debuffs_present: bool,
    #[serde(skip, default)]
    pub(crate) tracked_card_names: [Option<CardName>; 3],
    #[serde(skip, default)]
    pub(crate) tracked_card_damage_totals: [u64; 3],
    #[serde(skip, default)]
    pub player_raid_data: Option<Arc<PlayerRaidData>>,
    #[serde(skip, default)]
    pub support_modifiers: SupportModifiers,
    #[serde(skip, default)]
    pub(crate) damage_multiplier_cache: BossDamageMultiplierCache,
}

impl Boss {
    pub fn set_player_raid_data(&mut self, player_raid_data: Arc<PlayerRaidData>) {
        self.damage_multiplier_cache =
            BossDamageMultiplierCache::from_player(&player_raid_data, self.boss_name);
        self.player_raid_data = Some(player_raid_data);
    }

    pub fn set_support_modifiers(&mut self, support_modifiers: SupportModifiers) {
        self.support_modifiers = support_modifiers;
    }

    pub fn prepare_card_damage_tracking(&mut self, card_names: &[CardName]) {
        self.tracked_card_names = [None; 3];
        self.tracked_card_damage_totals = [0; 3];

        for (index, card_name) in card_names.iter().take(3).enumerate() {
            self.tracked_card_names[index] = Some(*card_name);
        }
    }

    pub fn part_mut(&mut self, part_name: BossPartName) -> &mut BossPart {
        match part_name {
            BossPartName::Head => &mut self.head,
            BossPartName::Torso => &mut self.torso,
            BossPartName::LeftShoulder => &mut self.left_shoulder,
            BossPartName::RightShoulder => &mut self.right_shoulder,
            BossPartName::LeftHand => &mut self.left_hand,
            BossPartName::RightHand => &mut self.right_hand,
            BossPartName::LeftLeg => &mut self.left_leg,
            BossPartName::RightLeg => &mut self.right_leg,
        }
    }

    pub fn part(&self, part_name: BossPartName) -> &BossPart {
        match part_name {
            BossPartName::Head => &self.head,
            BossPartName::Torso => &self.torso,
            BossPartName::LeftShoulder => &self.left_shoulder,
            BossPartName::RightShoulder => &self.right_shoulder,
            BossPartName::LeftHand => &self.left_hand,
            BossPartName::RightHand => &self.right_hand,
            BossPartName::LeftLeg => &self.left_leg,
            BossPartName::RightLeg => &self.right_leg,
        }
    }

    pub fn parts(&self) -> [&BossPart; 8] {
        [
            &self.head,
            &self.torso,
            &self.left_shoulder,
            &self.right_shoulder,
            &self.left_hand,
            &self.right_hand,
            &self.left_leg,
            &self.right_leg,
        ]
    }

    pub fn afflictions(&self, part_name: BossPartName) -> &[Affliction] {
        self.afflictions.part(part_name)
    }

    pub fn afflictions_mut(&mut self, part_name: BossPartName) -> &mut Vec<Affliction> {
        self.afflictions.part_mut(part_name)
    }

    pub fn apply_affliction(&mut self, part_name: BossPartName, affliction: Affliction) {
        if is_part_damage_taken_debuff(affliction.kind) {
            self.part_damage_taken_debuffs_present = true;
        }
        if affliction.kind == AfflictionKind::RadioactivityDebuff {
            self.radioactivity_debuffs_present = true;
        }

        self.afflictions.apply(part_name, affliction);
    }

    pub fn update(&mut self) {
        self.update_with_elapsed(1.0 / 20.0);
    }

    pub fn update_with_elapsed(&mut self, elapsed_seconds: f64) {
        if self.afflictions.is_empty() {
            return;
        }

        if self.radioactivity_debuffs_present {
            self.update_persistent_affliction_timers(elapsed_seconds);
        }

        let damage_events = {
            let tick_view = BossTickView {
                head: &self.head,
                torso: &self.torso,
                left_shoulder: &self.left_shoulder,
                right_shoulder: &self.right_shoulder,
                left_hand: &self.left_hand,
                right_hand: &self.right_hand,
                left_leg: &self.left_leg,
                right_leg: &self.right_leg,
                thriving_plague_part_count: self.afflictions.thriving_plague_part_count(),
            };
            let afflictions = &mut self.afflictions;
            let damage_events = afflictions.tick_afflictions(&tick_view, elapsed_seconds);
            afflictions.remove_expired();
            damage_events
        };
        self.part_damage_taken_debuffs_present = self.afflictions.has_part_damage_taken_debuff();
        self.radioactivity_debuffs_present = self
            .afflictions
            .has_active_kind_anywhere(AfflictionKind::RadioactivityDebuff);

        for event in damage_events {
            self.on_hit_with_source(event.part_name, event.damage, event.source);
        }
    }

    fn update_persistent_affliction_timers(&mut self, elapsed_seconds: f64) {
        let afflictions = &self.afflictions;

        if afflictions.has_active_kind(BossPartName::Head, AfflictionKind::RadioactivityDebuff) {
            self.head.radioactivity_afflicted_seconds += elapsed_seconds;
        }
        if afflictions.has_active_kind(BossPartName::Torso, AfflictionKind::RadioactivityDebuff) {
            self.torso.radioactivity_afflicted_seconds += elapsed_seconds;
        }
        if afflictions.has_active_kind(
            BossPartName::LeftShoulder,
            AfflictionKind::RadioactivityDebuff,
        ) {
            self.left_shoulder.radioactivity_afflicted_seconds += elapsed_seconds;
        }
        if afflictions.has_active_kind(
            BossPartName::RightShoulder,
            AfflictionKind::RadioactivityDebuff,
        ) {
            self.right_shoulder.radioactivity_afflicted_seconds += elapsed_seconds;
        }
        if afflictions.has_active_kind(BossPartName::LeftHand, AfflictionKind::RadioactivityDebuff)
        {
            self.left_hand.radioactivity_afflicted_seconds += elapsed_seconds;
        }
        if afflictions.has_active_kind(BossPartName::RightHand, AfflictionKind::RadioactivityDebuff)
        {
            self.right_hand.radioactivity_afflicted_seconds += elapsed_seconds;
        }
        if afflictions.has_active_kind(BossPartName::LeftLeg, AfflictionKind::RadioactivityDebuff) {
            self.left_leg.radioactivity_afflicted_seconds += elapsed_seconds;
        }
        if afflictions.has_active_kind(BossPartName::RightLeg, AfflictionKind::RadioactivityDebuff)
        {
            self.right_leg.radioactivity_afflicted_seconds += elapsed_seconds;
        }
    }
    pub fn record_damage(&mut self, source: DamageSource, damage: u64) {
        self.total_damage = self.total_damage.saturating_add(damage);

        match source {
            DamageSource::Tap => {
                self.tap_damage_total = self.tap_damage_total.saturating_add(damage);
            }
            DamageSource::Card(card_name) => {
                if self.record_tracked_card_damage(card_name, damage) {
                    return;
                }

                let total = self.card_damage_totals.entry(card_name).or_insert(0);
                *total = total.saturating_add(damage);
            }
        }
    }

    fn record_tracked_card_damage(&mut self, card_name: CardName, damage: u64) -> bool {
        for (index, tracked_name) in self.tracked_card_names.iter().enumerate() {
            if *tracked_name == Some(card_name) {
                self.tracked_card_damage_totals[index] =
                    self.tracked_card_damage_totals[index].saturating_add(damage);
                return true;
            }
        }

        false
    }

    pub fn card_damage_total(&self, card_name: CardName) -> u64 {
        for (index, tracked_name) in self.tracked_card_names.iter().enumerate() {
            if *tracked_name == Some(card_name) {
                return self.tracked_card_damage_totals[index];
            }
        }

        self.card_damage_totals
            .get(&card_name)
            .copied()
            .unwrap_or(0)
    }

    pub fn on_hit_with_source(
        &mut self,
        part_name: BossPartName,
        damage: u64,
        source: DamageSource,
    ) {
        let final_damage = self.final_damage_for(part_name, damage, &source);
        // if(source.label() == AcidDrench.display_name()){
        // if (source != DamageSource::Tap) {
        //     println!(
        //         "[Damage] {} : {}",
        //         source.label(),
        //         Self::format_compact(final_damage)
        //     );
        // }
        // }
        self.record_damage(source, final_damage);
        self.on_hit(part_name, final_damage);
    }

    pub fn preview_damage_with_source(
        &self,
        part_name: BossPartName,
        damage: u64,
        source: &DamageSource,
    ) -> u64 {
        self.final_damage_for(part_name, damage, source)
    }

    pub fn on_hit(&mut self, part_name: BossPartName, damage: u64) {
        match part_name {
            BossPartName::Head => self.head.on_hit(damage),
            BossPartName::Torso => self.torso.on_hit(damage),
            BossPartName::LeftShoulder => self.left_shoulder.on_hit(damage),
            BossPartName::RightShoulder => self.right_shoulder.on_hit(damage),
            BossPartName::LeftHand => self.left_hand.on_hit(damage),
            BossPartName::RightHand => self.right_hand.on_hit(damage),
            BossPartName::LeftLeg => self.left_leg.on_hit(damage),
            BossPartName::RightLeg => self.right_leg.on_hit(damage),
        }
    }
    pub fn get_state_from_part(&self, part_name: BossPartName) -> PartState {
        self.part(part_name).part_state
    }

    pub fn part_state_signature(&self) -> u16 {
        BossPartName::all()
            .iter()
            .enumerate()
            .fold(0u16, |signature, (index, part_name)| {
                signature
                    | ((part_state_bits(self.get_state_from_part(*part_name)) as u16)
                        << (index * 2))
            })
    }

    pub fn get_total_damage(&self) -> u64 {
        if self.total_damage == 0 && !self.damage_results.is_empty() {
            return self.damage_results.iter().map(|entry| entry.damage).sum();
        }

        self.total_damage
    }

    pub fn get_damage_result(&self) -> String {
        if self.total_damage == 0 && !self.damage_results.is_empty() {
            return self
                .damage_results
                .iter()
                .map(|entry| {
                    format!(
                        "{} : {}",
                        entry.source.label(),
                        Self::format_compact(entry.damage)
                    )
                })
                .collect::<Vec<_>>()
                .join("\n");
        }

        let mut entries = Vec::new();

        if self.tap_damage_total > 0 {
            entries.push((
                DamageSource::Tap.label(),
                Self::format_compact(self.tap_damage_total),
            ));
        }

        for (index, card_name) in self.tracked_card_names.iter().enumerate() {
            let Some(card_name) = card_name else {
                continue;
            };
            let damage = self.tracked_card_damage_totals[index];
            if damage == 0 {
                continue;
            }

            entries.push((
                DamageSource::Card(*card_name).label(),
                Self::format_compact(damage),
            ));
        }

        for (card_name, damage) in &self.card_damage_totals {
            entries.push((
                DamageSource::Card(*card_name).label(),
                Self::format_compact(*damage),
            ));
        }

        entries.sort_by(|left, right| left.0.cmp(right.0));

        entries
            .into_iter()
            .map(|(label, damage)| format!("{} : {}", label, damage))
            .collect::<Vec<_>>()
            .join("\n")
    }

    #[allow(non_snake_case)]
    pub fn getDamageResult(&self) -> String {
        self.get_damage_result()
    }

    pub fn get_random_body_part(&self) -> Option<BossPart> {
        let mut rng = rand::rng();

        // 1. Collect references to all 8 parts using your existing helper
        let all_parts = self.parts();

        // 2. Filter the parts to keep ONLY those where state == PartState::Body
        let body_parts: Vec<&BossPart> = all_parts
            .into_iter()
            .filter(|part| part.part_state == PartState::Body)
            .collect();

        // 3. Pick a random element out of the valid options and return a Clone
        // Returns None if no body parts exist (e.g., everything is armor or skeleton)
        body_parts.choose(&mut rng).map(|&part| part.clone())
    }
    fn format_compact(damage: u64) -> String {
        let damage_f = damage as f64;
        if damage >= 1_000_000_000_000 {
            format!("{:.12}T", damage_f / 1_000_000_000_000.0)
        } else if damage >= 1_000_000_000 {
            format!("{:.9}B", damage_f / 1_000_000_000.0)
        } else if damage >= 1_000_000 {
            format!("{:.6}M", damage_f / 1_000_000.0)
        } else if damage >= 1_000 {
            format!("{:.3}K", damage_f / 1_000.0)
        } else {
            damage.to_string()
        }
    }

    fn final_damage_for(
        &self,
        part_name: BossPartName,
        raw_damage: u64,
        source: &DamageSource,
    ) -> u64 {
        let cache = &self.damage_multiplier_cache;
        if !cache.ready {
            return raw_damage;
        }

        let state = self.get_state_from_part(part_name);

        let card_type = match source {
            DamageSource::Tap => None,
            DamageSource::Card(card_name) => Some(card_name.card_type()),
        };
        let support_bonus = self
            .support_modifiers
            .total_damage_bonus(part_name, state, card_type);
        let support_damage_mult = self.support_modifiers.damage_multiplier(card_type);

        //for guardbreak / maelStrom
        let part_debuff_bonus = self.part_damage_taken_bonus(part_name);
        // println!("support mult {}", 1.0 + support_bonus + part_debuff_bonus);
        let total_multiplier = cache.raid_all_mult
            * cache.boss_mult
            * cache.part_mult[part_name_index(part_name)]
            * cache.state_mult[part_state_index(state)]
            * (1.0 + support_bonus + part_debuff_bonus) as f32
            * support_damage_mult as f32;

        (raw_damage as f64 * total_multiplier as f64).max(0.0) as u64
    }

    fn part_damage_taken_bonus(&self, part_name: BossPartName) -> f64 {
        if !self.part_damage_taken_debuffs_present {
            return 0.0;
        }

        self.afflictions(part_name)
            .iter()
            .map(|affliction| match affliction.kind {
                AfflictionKind::GuardBreakDebuff | AfflictionKind::MaelstromDebuff => {
                    let active_stacks = affliction
                        .stacks
                        .iter()
                        .filter(|stack| stack.remaining_duration > 0.0)
                        .count() as f64;
                    affliction.source_skill.value_b.unwrap_or(0.0) * active_stacks
                }
                AfflictionKind::TotemOfPowerDebuff => {
                    let active_stacks = affliction
                        .stacks
                        .iter()
                        .filter(|stack| stack.remaining_duration > 0.0)
                        .count() as f64;
                    affliction.source_skill.value_a.unwrap_or(0.0) * active_stacks
                }
                _ => 0.0,
            })
            .sum()
    }
}
