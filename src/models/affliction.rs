use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AfflictionKind {
    Burning,
    Poison,
    Decay,
    Fusion,
    Shadow,
    Plague,
    Disease,
    Swarm,
    Rust,
    Bubble,
    Maelstrom,
    Amplify,
    SandsOfTime,
    CosmicBarb,
    GuardBreak
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AfflictionRefreshRule {
    Independent,
    RefreshAll,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AfflictionDamageRule {
    Tick,
    OnExpireByDuration,
    TickAndOnExpireByDuration,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AfflictionStack {
    pub remaining_duration: u8,
    pub attached_duration: u8,
}

impl AfflictionStack {
    pub fn new(duration: u8) -> Self {
        Self {
            remaining_duration: duration,
            attached_duration: duration,
        }
    }

    pub fn refresh(&mut self, duration: u8) {
        self.remaining_duration = duration;
        self.attached_duration = duration;
    }

    pub fn tick(&mut self) {
        self.remaining_duration = self.remaining_duration.saturating_sub(1);
    }

    pub fn is_expired(&self) -> bool {
        self.remaining_duration == 0
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Affliction {
    pub kind: AfflictionKind,
    #[serde(default)]
    pub stacks: Vec<AfflictionStack>,
    pub damage_per_tick: u64,
    pub expire_damage_per_duration: u64,
}

impl AfflictionKind {
    pub fn refresh_rule(self) -> AfflictionRefreshRule {
        match self {
            AfflictionKind::Poison | AfflictionKind::Fusion => AfflictionRefreshRule::RefreshAll,
            _ => AfflictionRefreshRule::Independent,
        }
    }

    pub fn damage_rule(self) -> AfflictionDamageRule {
        match self {
            AfflictionKind::Fusion => AfflictionDamageRule::OnExpireByDuration,
            _ => AfflictionDamageRule::Tick,
        }
    }

    pub fn max_stacks(self) -> Option<u8> {
        match self {
            AfflictionKind::Poison => Some(15),
            _ => None,
        }
    }
}

impl Affliction {
    pub fn new(
        kind: AfflictionKind,
        stack_count: u8,
        duration: u8,
        damage_per_tick: u64,
        expire_damage_per_duration: u64,
    ) -> Self {
        let stacks = (0..stack_count)
            .map(|_| AfflictionStack::new(duration))
            .collect();
        Self {
            kind,
            stacks,
            damage_per_tick,
            expire_damage_per_duration,
        }
    }

    pub fn stack_count(&self) -> usize {
        self.stacks.len()
    }

    pub fn apply_stacks(&mut self, stack_count: u8, duration: u8) {
        match self.kind.refresh_rule() {
            AfflictionRefreshRule::RefreshAll => {
                for stack in &mut self.stacks {
                    stack.refresh(duration);
                }
            }
            AfflictionRefreshRule::Independent => {}
        }

        let max_stacks = self.kind.max_stacks().map(|cap| cap as usize);
        let available_slots = max_stacks
            .map(|cap| cap.saturating_sub(self.stacks.len()))
            .unwrap_or(stack_count as usize);

        let stacks_to_add = usize::min(stack_count as usize, available_slots);
        self.stacks
            .extend((0..stacks_to_add).map(|_| AfflictionStack::new(duration)));
    }

    pub fn tick(&mut self) -> u64 {
        let mut total_damage = 0u64;

        for stack in &mut self.stacks {
            match self.kind.damage_rule() {
                AfflictionDamageRule::Tick => {
                    if stack.remaining_duration > 0 {
                        total_damage = total_damage
                            .saturating_add(self.damage_per_tick);
                    }
                }
                AfflictionDamageRule::OnExpireByDuration => {}
                AfflictionDamageRule::TickAndOnExpireByDuration => {
                    if stack.remaining_duration > 0 {
                        total_damage = total_damage
                            .saturating_add(self.damage_per_tick);
                    }
                }
            }

            stack.tick();

            if stack.is_expired() {
                total_damage = total_damage.saturating_add(self.expire_damage_per_duration.saturating_mul(stack.attached_duration as u64));
            }
        }

        self.stacks.retain(|stack| !stack.is_expired());
        total_damage
    }

    pub fn is_expired(&self) -> bool {
        self.stacks.is_empty()
    }
}
