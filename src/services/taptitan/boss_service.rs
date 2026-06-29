use crate::models::boss::{Boss, BossName, BossPart, BossPartName, PartState};

pub struct BossService;

impl BossService {
    /// Creates a fresh boss instance initialized with custom health values.
    /// This is useful if you want to generate default tier layouts programmatically.
    pub fn create_custom_boss(name: BossName, default_armor: u64, default_health: u64) -> Boss {
        Boss {
            boss_name: name,
            head: BossPart::new(
                BossPartName::Head,
                PartState::Armor,
                default_armor,
                default_health,
                default_armor,
                default_health,
            ),
            torso: BossPart::new(
                BossPartName::Torso,
                PartState::Armor,
                default_armor,
                default_health,
                default_armor,
                default_health,
            ),
            left_shoulder: BossPart::new(
                BossPartName::LeftShoulder,
                PartState::Armor,
                default_armor,
                default_health,
                default_armor,
                default_health,
            ),
            right_shoulder: BossPart::new(
                BossPartName::RightShoulder,
                PartState::Armor,
                default_armor,
                default_health,
                default_armor,
                default_health,
            ),
            left_hand: BossPart::new(
                BossPartName::LeftHand,
                PartState::Armor,
                default_armor,
                default_health,
                default_armor,
                default_health,
            ),
            right_hand: BossPart::new(
                BossPartName::RightHand,
                PartState::Armor,
                default_armor,
                default_health,
                default_armor,
                default_health,
            ),
            left_leg: BossPart::new(
                BossPartName::LeftLeg,
                PartState::Armor,
                default_armor,
                default_health,
                default_armor,
                default_health,
            ),
            right_leg: BossPart::new(
                BossPartName::RightLeg,
                PartState::Armor,
                default_armor,
                default_health,
                default_armor,
                default_health,
            ),
            damage_results: Vec::new(),
            player_raid_data: None,
            support_modifiers: Default::default(),
        }
    }

    /// Helper to check if a specific part of the boss is completely broken (Skeleton state)
    pub fn is_part_destroyed(boss: &Boss, part: BossPartName) -> bool {
        match part {
            BossPartName::Head => boss.head.current_health == 0,
            BossPartName::Torso => boss.torso.current_health == 0,
            BossPartName::LeftShoulder => boss.left_shoulder.current_health == 0,
            BossPartName::RightShoulder => boss.right_shoulder.current_health == 0,
            BossPartName::LeftHand => boss.left_hand.current_health == 0,
            BossPartName::RightHand => boss.right_hand.current_health == 0,
            BossPartName::LeftLeg => boss.left_leg.current_health == 0,
            BossPartName::RightLeg => boss.right_leg.current_health == 0,
        }
    }
}
