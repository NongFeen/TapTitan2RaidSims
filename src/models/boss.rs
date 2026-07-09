use rand::seq::IndexedRandom;
use serde::{Deserialize, Serialize};
use strum_macros::{EnumIter, EnumString};

use crate::models::affliction::{Affliction, AfflictionKind};
use crate::models::card_skill_data::{card_skill_value_a, card_skill_value_b};
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
    pub afflictions: Vec<Affliction>,
    #[serde(default)]
    pub radioactivity_afflicted_seconds: f64,
}

#[derive(Debug, Clone, Copy)]
pub struct BossPartTickView {
    pub part_name: BossPartName,
    pub part_state: PartState,
    pub max_armor: u64,
    pub max_health: u64,
    pub current_armor: u64,
    pub current_health: u64,
    pub radioactivity_afflicted_seconds: f64,
    pub has_thriving_plague: bool,
}

#[derive(Debug, Clone, Copy)]
pub struct BossTickView {
    pub head: BossPartTickView,
    pub torso: BossPartTickView,
    pub left_shoulder: BossPartTickView,
    pub right_shoulder: BossPartTickView,
    pub left_hand: BossPartTickView,
    pub right_hand: BossPartTickView,
    pub left_leg: BossPartTickView,
    pub right_leg: BossPartTickView,
}

impl BossTickView {
    pub fn part(&self, part_name: BossPartName) -> &BossPartTickView {
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

    pub fn parts(&self) -> [&BossPartTickView; 8] {
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
            afflictions: Vec::new(),
            radioactivity_afflicted_seconds: 0.0,
        }
    }
    pub fn is_limb(&self) -> bool {
        self.part_name.is_limb()
    }
    pub fn update(
        &mut self,
        boss: &BossTickView,
        elapsed_seconds: f64,
    ) -> Vec<card_function::AfflictionDamageEvent> {
        self.tick_afflictions(boss, elapsed_seconds)
    }
    pub fn apply_affliction(&mut self, affliction: Affliction) {
        let incoming_stack_count = affliction.stack_count() as u32;
        let incoming_duration = affliction
            .stacks
            .first()
            .map(|stack| stack.attached_duration)
            .unwrap_or(0.0);
        let incoming_tick_damage = affliction.damage_per_second;
        let incoming_expire_damage = affliction.remove_damage;
        let incoming_tick_interval = affliction.tick_interval_seconds;

        if let Some(existing) = self
            .afflictions
            .iter_mut()
            .find(|current| current.kind == affliction.kind)
        {
            existing.apply_stacks(
                incoming_stack_count,
                incoming_duration,
                incoming_tick_damage,
                incoming_expire_damage,
                incoming_tick_interval,
            );
            return;
        }

        self.afflictions.push(affliction);
    }
    fn tick_afflictions(
        &mut self,
        boss: &BossTickView,
        elapsed_seconds: f64,
    ) -> Vec<card_function::AfflictionDamageEvent> {
        let mut damage_events = Vec::new();
        let part_name = self.part_name;

        for affliction in &mut self.afflictions {
            damage_events.extend(card_function::tick_affliction(
                affliction,
                boss,
                part_name,
                elapsed_seconds,
            ));
        }

        // self.remove_expired_afflictions();
        damage_events
    }
    pub fn remove_expired_afflictions(&mut self) {
        for affliction in &mut self.afflictions {
            affliction.remove_expired_stacks();
        }

        self.afflictions.retain(|affliction| !affliction.is_expired());
    }
    fn tick_view(&self) -> BossPartTickView {
        BossPartTickView {
            part_name: self.part_name,
            part_state: self.part_state,
            max_armor: self.max_armor,
            max_health: self.max_health,
            current_armor: self.current_armor,
            current_health: self.current_health,
            radioactivity_afflicted_seconds: self.radioactivity_afflicted_seconds,
            has_thriving_plague: self
                .afflictions
                .iter()
                .any(|affliction| affliction.kind == AfflictionKind::ThrivingPlagueDebuff),
        }
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

impl BossPartName {
    pub fn is_limb(&self) -> bool {
        match self {
            BossPartName::Head | BossPartName::Torso => false,
            _ => true, // Everything else (shoulders, hands, legs) are limbs
        }
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
    #[serde(default)]
    pub damage_results: Vec<DamageResult>,
    #[serde(skip, default)]
    pub total_damage: u64,
    #[serde(skip, default)]
    pub player_raid_data: Option<PlayerRaidData>,
    #[serde(skip, default)]
    pub support_modifiers: SupportModifiers,
}

impl Boss {
    pub fn set_player_raid_data(&mut self, player_raid_data: PlayerRaidData) {
        self.player_raid_data = Some(player_raid_data);
    }

    pub fn set_support_modifiers(&mut self, support_modifiers: SupportModifiers) {
        self.support_modifiers = support_modifiers;
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

    pub fn apply_affliction(&mut self, part_name: BossPartName, affliction: Affliction) {
        match part_name {
            BossPartName::Head => self.head.apply_affliction(affliction),
            BossPartName::Torso => self.torso.apply_affliction(affliction),
            BossPartName::LeftShoulder => self.left_shoulder.apply_affliction(affliction),
            BossPartName::RightShoulder => self.right_shoulder.apply_affliction(affliction),
            BossPartName::LeftHand => self.left_hand.apply_affliction(affliction),
            BossPartName::RightHand => self.right_hand.apply_affliction(affliction),
            BossPartName::LeftLeg => self.left_leg.apply_affliction(affliction),
            BossPartName::RightLeg => self.right_leg.apply_affliction(affliction),
        }
    }

    fn parts_mut(&mut self) -> [&mut BossPart; 8] {
        [
            &mut self.head,
            &mut self.torso,
            &mut self.left_shoulder,
            &mut self.right_shoulder,
            &mut self.left_hand,
            &mut self.right_hand,
            &mut self.left_leg,
            &mut self.right_leg,
        ]
    }

    pub fn update(&mut self) {
        self.update_with_elapsed(1.0 / 20.0);
    }

    pub fn update_with_elapsed(&mut self, elapsed_seconds: f64) {
        if self.parts().iter().all(|part| part.afflictions.is_empty()) {
            return;
        }

        self.update_persistent_affliction_timers(elapsed_seconds);

        let snapshot = self.tick_view();
        let mut damage_events = Vec::new();

        for part in self.parts_mut() {
            damage_events.extend(part.update(&snapshot, elapsed_seconds));
        }
        for part in self.parts_mut() {
            part.remove_expired_afflictions();
        }

        for event in damage_events {
            self.on_hit_with_source(event.part_name, event.damage, event.source);
        }
    }

    fn tick_view(&self) -> BossTickView {
        BossTickView {
            head: self.head.tick_view(),
            torso: self.torso.tick_view(),
            left_shoulder: self.left_shoulder.tick_view(),
            right_shoulder: self.right_shoulder.tick_view(),
            left_hand: self.left_hand.tick_view(),
            right_hand: self.right_hand.tick_view(),
            left_leg: self.left_leg.tick_view(),
            right_leg: self.right_leg.tick_view(),
        }
    }

    fn update_persistent_affliction_timers(&mut self, elapsed_seconds: f64) {
        for part in self.parts_mut() {
            let has_radioactivity = part.afflictions.iter().any(|affliction| {
                affliction.kind == AfflictionKind::RadioactivityDebuff
                    && affliction
                        .stacks
                        .iter()
                        .any(|stack| stack.remaining_duration > 0.0)
            });

            if has_radioactivity {
                part.radioactivity_afflicted_seconds += elapsed_seconds;
            }
        }
    }
    pub fn record_damage(&mut self, source: DamageSource, damage: u64) {
        self.total_damage = self.total_damage.saturating_add(damage);

        let source_label = source.label();

        if let Some(existing) = self
            .damage_results
            .iter_mut()
            .find(|entry| entry.source.label() == source_label)
        {
            existing.damage = existing.damage.saturating_add(damage);
            return;
        }

        self.damage_results.push(DamageResult { source, damage });
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

    pub fn get_total_damage(&self) -> u64 {
        if self.total_damage == 0 && !self.damage_results.is_empty() {
            return self.damage_results.iter().map(|entry| entry.damage).sum();
        }

        self.total_damage
    }

    pub fn get_damage_result(&self) -> String {
        self.damage_results
            .iter()
            .map(|entry| {
                format!(
                    "{} : {}",
                    entry.source.label(),
                    Self::format_compact(entry.damage)
                )
            })
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
        let Some(player_raid_data) = self.player_raid_data.as_ref() else {
            return raw_damage;
        };

        let state = self.get_state_from_part(part_name);
        let jade_set = if player_raid_data.raid_set.jade_anniversary {
            0.04
        } else {
            0.0
        };
        let raid_all_mult = 1.0 + jade_set + player_raid_data.title;
        let tts_boss_mult = 1.0
            + player_raid_data
                .titan_soul_research
                .get_boss_mult(self.boss_name);
        let tts_part_mult = 1.0
            + player_raid_data
                .titan_soul_research
                .get_part_mult(part_name);
        let tts_state_mult = 1.0 + player_raid_data.titan_soul_research.get_state_mult(state);

        let card_type = match source {
            DamageSource::Tap => None,
            DamageSource::Card(card_name) => Some(card_name.card_type()),
        };
        let support_bonus = self
            .support_modifiers
            .total_damage_bonus(part_name, state, card_type);

        //for guardbreak / maelStrom
        let part_debuff_bonus = self.part_damage_taken_bonus(part_name);
        // println!("support mult {}", 1.0 + support_bonus + part_debuff_bonus);
        let total_multiplier = raid_all_mult
            * tts_boss_mult
            * tts_part_mult
            * tts_state_mult
            * (1.0 + support_bonus + part_debuff_bonus) as f32;

        (raw_damage as f64 * total_multiplier as f64).max(0.0) as u64
    }

    fn part_damage_taken_bonus(&self, part_name: BossPartName) -> f64 {
        self.part(part_name)
            .afflictions
            .iter()
            .map(|affliction| match affliction.kind {
                AfflictionKind::GuardBreakDebuff | AfflictionKind::MaelstromDebuff => {
                    let active_stacks = affliction
                        .stacks
                        .iter()
                        .filter(|stack| stack.remaining_duration > 0.0)
                        .count() as f64;
                    card_skill_value_b(affliction.source_card, affliction.source_level)
                        .unwrap_or(0.0)
                        * active_stacks
                }
                AfflictionKind::TotemOfPowerDebuff => {
                    let active_stacks = affliction
                        .stacks
                        .iter()
                        .filter(|stack| stack.remaining_duration > 0.0)
                        .count() as f64;
                    card_skill_value_a(affliction.source_card, affliction.source_level)
                        .unwrap_or(0.0)
                        * active_stacks
                }
                _ => 0.0,
            })
            .sum()
    }
}
