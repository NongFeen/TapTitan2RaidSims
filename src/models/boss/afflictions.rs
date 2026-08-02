use super::*;

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

    pub(super) fn parts(&self) -> [&Vec<Affliction>; 8] {
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

    pub(super) fn apply(&mut self, part_name: BossPartName, affliction: Affliction) {
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

    pub(super) fn is_empty(&self) -> bool {
        self.parts()
            .iter()
            .all(|afflictions| afflictions.is_empty())
    }

    pub(super) fn has_active_kind(&self, part_name: BossPartName, kind: AfflictionKind) -> bool {
        self.part(part_name).iter().any(|affliction| {
            affliction.kind == kind
                && affliction
                    .stacks
                    .iter()
                    .any(|stack| stack.remaining_duration > 0.0)
        })
    }

    pub(super) fn has_part_damage_taken_debuff(&self) -> bool {
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

    pub(super) fn has_active_kind_anywhere(&self, kind: AfflictionKind) -> bool {
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

    pub(super) fn thriving_plague_part_count(&self) -> usize {
        self.parts()
            .iter()
            .filter(|afflictions| {
                afflictions
                    .iter()
                    .any(|affliction| affliction.kind == AfflictionKind::ThrivingPlagueDebuff)
            })
            .count()
    }

    pub(super) fn tick_afflictions(
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

    pub(super) fn remove_expired(&mut self) {
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
