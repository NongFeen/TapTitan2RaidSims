use super::*;

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

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize, Serialize)]
pub enum GlobalRaidModifier {
    #[default]
    None,
    BurstDamage,
    BurstChance,
    SupportEffect,
    AfflictionChance,
    AfflictionDamage,
    AllDamage,
    AttackDuration,
    AfflictionDuration,
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
    pub(super) thriving_plague_part_count: usize,
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

    pub fn thriving_plague_part_count(&self) -> usize {
        self.thriving_plague_part_count
    }
}

impl BossPart {
    pub fn sync_state_from_current_values(&mut self) {
        self.part_state = if self.current_armor > 0 {
            PartState::Armor
        } else if self.current_health > 0 {
            PartState::Body
        } else {
            PartState::Skeleton
        };
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
