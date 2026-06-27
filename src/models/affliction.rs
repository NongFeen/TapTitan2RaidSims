use serde::{Deserialize, Serialize};

use crate::models::cards::CardName;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AfflictionKind {
    BlazingInfernoDebuff,
    AcidDrenchDebuff,
    DecayingStrikeDebuff,
    FusionBombDebuff,
    GrimShadowDebuff,
    ThrivingPlagueDebuff,
    RadioactivityDebuff,
    RavenousSwarmDebuff,
    RuinousRainDebuff,
    CorrosiveBubblesDebuff,
    MaelstromDebuff,
    AmplifyDebuff,
    SandsOfTimeDebuff,
    ElectroZapDebuff,
    GuardBreakDebuff,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AfflictionRefreshRule {
    RefreshAll,
    Independent,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AfflictionOverflow {
    ReplaceOldest,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AfflictionStack {
    pub remaining_duration: f64,
    pub attached_duration: f64,
    #[serde(default)]
    pub damage_multiplier: f64,
    #[serde(default)]
    pub tick_elapsed: f64,
}

impl AfflictionStack {
    pub fn new(duration: f64) -> Self {
        Self {
            remaining_duration: duration,
            attached_duration: duration,
            damage_multiplier: 1.0,
            tick_elapsed: 0.0,
        }
    }

    pub fn refresh(&mut self, duration: f64) {
        self.remaining_duration = duration;
        self.attached_duration = duration;
        self.tick_elapsed = 0.0;
    }

    pub fn tick(&mut self, elapsed: f64) {
        self.remaining_duration = (self.remaining_duration - elapsed).max(0.0);
    }

    pub fn is_expired(&self) -> bool {
        self.remaining_duration <= 0.0
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Affliction {
    pub kind: AfflictionKind,
    pub source_card: CardName,
    pub source_level: u16,
    #[serde(default)]
    pub stacks: Vec<AfflictionStack>,
    pub damage_per_second: f64,
    pub remove_damage: f64,
    pub tick_interval_seconds: f64,
    #[serde(default)]
    pub tick_elapsed: f64,
    pub max_stacks: u32,
    pub refresh_rule: AfflictionRefreshRule,
    pub overflow: AfflictionOverflow,
}

impl AfflictionKind {
    pub fn from_card(card_id: CardName) -> Option<Self> {
        match card_id {
            CardName::BlazingInferno => Some(Self::BlazingInfernoDebuff),
            CardName::AcidDrench => Some(Self::AcidDrenchDebuff),
            CardName::DecayingStrike => Some(Self::DecayingStrikeDebuff),
            CardName::FusionBomb => Some(Self::FusionBombDebuff),
            CardName::GrimShadow => Some(Self::GrimShadowDebuff),
            CardName::ThrivingPlague => Some(Self::ThrivingPlagueDebuff),
            CardName::Radioactivity => Some(Self::RadioactivityDebuff),
            CardName::RavenousSwarm => Some(Self::RavenousSwarmDebuff),
            CardName::RuinousRain => Some(Self::RuinousRainDebuff),
            CardName::CorrosiveBubbles => Some(Self::CorrosiveBubblesDebuff),
            CardName::Maelstrom => Some(Self::MaelstromDebuff),
            CardName::Amplify => Some(Self::AmplifyDebuff),
            CardName::SandsOfTime => Some(Self::SandsOfTimeDebuff),
            CardName::ElectroZap => Some(Self::ElectroZapDebuff),
            CardName::GuardBreak => Some(Self::GuardBreakDebuff),
            _ => None,
        }
    }

    pub fn refresh_rule(self) -> AfflictionRefreshRule {
        match self {
            AfflictionKind::AcidDrenchDebuff | AfflictionKind::CorrosiveBubblesDebuff => {
                AfflictionRefreshRule::RefreshAll
            }
            _ => AfflictionRefreshRule::Independent,
        }
    }

    pub fn overflow(self) -> AfflictionOverflow {
        AfflictionOverflow::ReplaceOldest
    }
}

impl Affliction {
    pub fn new(
        kind: AfflictionKind,
        source_card: CardName,
        source_level: u16,
        stack_count: u32,
        duration: f64,
        damage_per_second: f64,
        remove_damage: f64,
        tick_interval_seconds: f64,
        max_stacks: u32,
    ) -> Self {
        let stacks = (0..stack_count)
            .map(|_| AfflictionStack::new(duration))
            .collect();
        Self {
            kind,
            source_card,
            source_level,
            stacks,
            damage_per_second,
            remove_damage,
            tick_interval_seconds,
            tick_elapsed: 0.0,
            max_stacks,
            refresh_rule: kind.refresh_rule(),
            overflow: kind.overflow(),
        }
    }

    pub fn stack_count(&self) -> usize {
        self.stacks.len()
    }

    pub fn apply_stacks(
        &mut self,
        stack_count: u32,
        duration: f64,
        damage_per_second: f64,
        remove_damage: f64,
        tick_interval_seconds: f64,
    ) {
        self.damage_per_second = self.damage_per_second.max(damage_per_second);
        self.remove_damage = self.remove_damage.max(remove_damage);
        self.tick_interval_seconds = tick_interval_seconds;

        match self.refresh_rule {
            AfflictionRefreshRule::RefreshAll => {
                for stack in &mut self.stacks {
                    stack.refresh(duration);
                }
            }
            AfflictionRefreshRule::Independent => {}
        }

        let max_stacks = self.max_stacks as usize;
        let available_slots = max_stacks.saturating_sub(self.stacks.len());
        let stacks_to_add = usize::min(stack_count as usize, available_slots);
        self.stacks
            .extend((0..stacks_to_add).map(|_| AfflictionStack::new(duration)));

        if stacks_to_add < stack_count as usize
            && self.overflow == AfflictionOverflow::ReplaceOldest
        {
            let overflow_count = stack_count as usize - stacks_to_add;
            for _ in 0..overflow_count {
                if let Some(oldest) = self.stacks.iter_mut().min_by(|left, right| {
                    left.remaining_duration.total_cmp(&right.remaining_duration)
                }) {
                    oldest.refresh(duration);
                }
            }
        }
    }

    pub fn is_expired(&self) -> bool {
        self.stacks.is_empty()
    }
}
