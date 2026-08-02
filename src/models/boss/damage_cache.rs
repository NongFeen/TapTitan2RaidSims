use super::*;

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

pub(super) fn part_state_bits(state: PartState) -> u8 {
    match state {
        PartState::Cursed => 0,
        PartState::Armor => 1,
        PartState::Body => 2,
        PartState::Skeleton => 3,
    }
}

pub(super) fn is_part_damage_taken_debuff(kind: AfflictionKind) -> bool {
    matches!(
        kind,
        AfflictionKind::GuardBreakDebuff
            | AfflictionKind::MaelstromDebuff
            | AfflictionKind::TotemOfPowerDebuff
    )
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct BossDamageMultiplierCache {
    pub(super) ready: bool,
    pub(super) raid_all_mult: f32,
    pub(super) boss_mult: f32,
    pub(super) part_mult: [f32; 8],
    pub(super) state_mult: [f32; 4],
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
    pub(super) fn from_player(player_raid_data: &PlayerRaidData, boss_name: BossName) -> Self {
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

pub(super) fn part_name_index(part_name: BossPartName) -> usize {
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

pub(super) fn part_state_index(state: PartState) -> usize {
    match state {
        PartState::Cursed => 0,
        PartState::Armor => 1,
        PartState::Body => 2,
        PartState::Skeleton => 3,
    }
}
