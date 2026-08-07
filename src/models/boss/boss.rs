use super::*;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Boss {
    pub boss_name: BossName,
    #[serde(default)]
    pub global_raid_modifier: GlobalRaidModifier,
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
    pub(crate) result_target_mask: Option<u8>,
    #[serde(skip, default)]
    pub player_raid_data: Option<Arc<PlayerRaidData>>,
    #[serde(skip, default)]
    pub support_modifiers: SupportModifiers,
    #[serde(skip, default)]
    pub(crate) damage_multiplier_cache: BossDamageMultiplierCache,
}

impl Boss {
    pub fn sync_part_states_from_current_values(&mut self) {
        self.head.sync_state_from_current_values();
        self.torso.sync_state_from_current_values();
        self.left_shoulder.sync_state_from_current_values();
        self.right_shoulder.sync_state_from_current_values();
        self.left_hand.sync_state_from_current_values();
        self.right_hand.sync_state_from_current_values();
        self.left_leg.sync_state_from_current_values();
        self.right_leg.sync_state_from_current_values();
    }

    pub fn set_player_raid_data(&mut self, player_raid_data: Arc<PlayerRaidData>) {
        self.damage_multiplier_cache =
            BossDamageMultiplierCache::from_player(&player_raid_data, self.boss_name);
        self.player_raid_data = Some(player_raid_data);
    }

    pub fn set_result_target_parts(&mut self, target_parts: &[BossPartName]) {
        self.damage_results.clear();
        self.result_target_mask = Some(target_parts.iter().fold(0u8, |mask, part_name| {
            mask | (1u8 << part_name_index(*part_name))
        }));
    }

    fn damage_counts_toward_result(&self, part_name: BossPartName) -> bool {
        self.result_target_mask
            .map_or(true, |mask| mask & (1u8 << part_name_index(part_name)) != 0)
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

    pub(super) fn update_persistent_affliction_timers(&mut self, elapsed_seconds: f64) {
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

    pub(super) fn record_tracked_card_damage(&mut self, card_name: CardName, damage: u64) -> bool {
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
        if self.damage_counts_toward_result(part_name) {
            self.record_damage(source, final_damage);
        }
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

    pub(super) fn final_damage_for(
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

    pub(super) fn part_damage_taken_bonus(&self, part_name: BossPartName) -> f64 {
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

#[cfg(test)]
mod state_sync_tests {
    use serde_json::json;

    use super::*;

    fn part(part_name: &str, current_armor: u64, current_health: u64) -> serde_json::Value {
        json!({
            "part_name": part_name,
            "part_state": "Cursed",
            "max_armor": 100,
            "max_health": 100,
            "current_armor": current_armor,
            "current_health": current_health
        })
    }

    #[test]
    fn syncs_all_part_states_from_current_durability() {
        let mut boss: Boss = serde_json::from_value(json!({
            "boss_name": "Jukk",
            "head": part("Head", 10, 20),
            "torso": part("Torso", 1, 0),
            "left_shoulder": part("LeftShoulder", 0, 5),
            "right_shoulder": part("RightShoulder", 0, 1),
            "left_hand": part("LeftHand", 0, 0),
            "right_hand": part("RightHand", 0, 0),
            "left_leg": part("LeftLeg", 99, 0),
            "right_leg": part("RightLeg", 0, 99)
        }))
        .expect("test boss should deserialize");

        boss.sync_part_states_from_current_values();

        let actual = boss.parts().map(|part| part.part_state);
        assert_eq!(
            actual,
            [
                PartState::Armor,
                PartState::Armor,
                PartState::Body,
                PartState::Body,
                PartState::Skeleton,
                PartState::Skeleton,
                PartState::Armor,
                PartState::Body,
            ]
        );
    }

    #[test]
    fn damage_on_any_configured_result_target_is_accumulated() {
        let mut boss: Boss = serde_json::from_value(json!({
            "boss_name": "Jukk",
            "head": part("Head", 100, 100),
            "torso": part("Torso", 100, 100),
            "left_shoulder": part("LeftShoulder", 100, 100),
            "right_shoulder": part("RightShoulder", 100, 100),
            "left_hand": part("LeftHand", 100, 100),
            "right_hand": part("RightHand", 100, 100),
            "left_leg": part("LeftLeg", 100, 100),
            "right_leg": part("RightLeg", 100, 100)
        }))
        .expect("test boss should deserialize");
        boss.sync_part_states_from_current_values();
        boss.set_result_target_parts(&[BossPartName::Head, BossPartName::Torso]);

        boss.on_hit_with_source(
            BossPartName::LeftShoulder,
            10,
            DamageSource::Card(CardName::ThrivingPlague),
        );
        assert_eq!(
            boss.left_shoulder.current_armor, 90,
            "off-target damage still applies"
        );
        assert_eq!(boss.get_total_damage(), 0);
        assert_eq!(boss.card_damage_total(CardName::ThrivingPlague), 0);

        boss.on_hit_with_source(
            BossPartName::Head,
            20,
            DamageSource::Card(CardName::ThrivingPlague),
        );
        assert_eq!(boss.get_total_damage(), 20);
        assert_eq!(boss.card_damage_total(CardName::ThrivingPlague), 20);

        boss.on_hit_with_source(
            BossPartName::Torso,
            15,
            DamageSource::Card(CardName::ThrivingPlague),
        );
        assert_eq!(boss.get_total_damage(), 35);
        assert_eq!(boss.card_damage_total(CardName::ThrivingPlague), 35);

        boss.set_result_target_parts(&[]);
        boss.on_hit_with_source(BossPartName::Head, 5, DamageSource::Tap);
        assert_eq!(
            boss.head.current_armor, 75,
            "damage still applies without a target"
        );
        assert_eq!(boss.get_total_damage(), 35);
    }
}
