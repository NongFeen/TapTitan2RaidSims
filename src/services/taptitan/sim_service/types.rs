use super::*;

pub(super) const SIMS_ROUNDS: u64 = 20;
pub(super) const TICKS_PER_ROUND: u32 = 600;
pub(super) const TICKS_PER_SECOND: f64 = 20.0;
pub(super) const PRINT_SIM_PATTERN_PROGRESS: bool = true;
pub(super) const SIM_PATTERN_PROGRESS_STEP_PERCENT: usize = 10;
pub(super) const PRINT_EVERY_SIM_PATTERN: bool = false;
// const PRINT_EVERY_SIM_PATTERN: bool = true;
pub(super) const PRINT_PROC_CACHE: bool = false;
pub(super) const ENABLE_PARALLEL_SIM: bool = true;
pub(super) const SIM_WORKER_COUNT: usize = 1; // 0 = use available_parallelism()

pub(super) const GLOBAL_RAID_BURST_DAMAGE_MULT: f64 = 1.3;
pub(super) const GLOBAL_RAID_BURST_CHANCE_MULT: f64 = 1.3;
pub(super) const GLOBAL_RAID_SUPPORT_EFFECT_MULT: f64 = 1.15;
pub(super) const GLOBAL_RAID_AFFLICTION_CHANCE_MULT: f64 = 1.3;
pub(super) const GLOBAL_RAID_AFFLICTION_DAMAGE_MULT: f64 = 1.3;
pub(super) const GLOBAL_RAID_ALL_DAMAGE_MULT: f64 = 1.15;
pub(super) const GLOBAL_RAID_ATTACK_DURATION_ADD_SECONDS: f64 = 3.0;
pub(super) const GLOBAL_RAID_AFFLICTION_DURATION_MULT: f64 = 1.5;

pub(super) const COSMIC_HAYMAKER_TAPS_PER_PROC: u16 = 70;
pub(super) const CELESTIAL_STATIC_STACKS_PER_PROC: usize = 8;
pub(super) const COSMIC_HAYMAKER_FAST_PROC_KEY: u16 = 20000;
pub(super) const FAST_CALC_CARDS: [CardName; 20] = [
    CardName::MoonBeam,
    CardName::Fragmentize,
    CardName::SkullBash,
    CardName::RazorWind,
    CardName::PsychicShackles,
    CardName::FlakShot,
    CardName::CosmicHaymaker,
    CardName::BarbedMorningstar,
    CardName::CrushingInstinct,
    CardName::InsanityVoid,
    CardName::InspiringForce,
    CardName::SoulFire,
    CardName::VictoryMarch,
    CardName::PrismaticRift,
    CardName::AncestralFavor,
    CardName::GraspingVines,
    CardName::TeamTactics,
    CardName::SkeletalSmash,
    CardName::AstralEcho,
    CardName::BattleDrums,
];

#[derive(Debug, Clone, Copy)]
pub(super) struct GlobalRaidModifiers {
    pub(super) burst_damage_mult: f64,
    pub(super) burst_chance_mult: f64,
    pub(super) support_effect_mult: f64,
    pub(super) affliction_chance_mult: f64,
    pub(super) affliction_damage_mult: f64,
    pub(super) all_damage_mult: f64,
    pub(super) attack_duration_add_seconds: f64,
    pub(super) affliction_duration_mult: f64,
}

pub(super) fn global_raid_modifiers(selected: GlobalRaidModifier) -> GlobalRaidModifiers {
    GlobalRaidModifiers {
        burst_damage_mult: if selected == GlobalRaidModifier::BurstDamage {
            GLOBAL_RAID_BURST_DAMAGE_MULT
        } else {
            1.0
        },
        burst_chance_mult: if selected == GlobalRaidModifier::BurstChance {
            GLOBAL_RAID_BURST_CHANCE_MULT
        } else {
            1.0
        },
        support_effect_mult: if selected == GlobalRaidModifier::SupportEffect {
            GLOBAL_RAID_SUPPORT_EFFECT_MULT
        } else {
            1.0
        },
        affliction_chance_mult: if selected == GlobalRaidModifier::AfflictionChance {
            GLOBAL_RAID_AFFLICTION_CHANCE_MULT
        } else {
            1.0
        },
        affliction_damage_mult: if selected == GlobalRaidModifier::AfflictionDamage {
            GLOBAL_RAID_AFFLICTION_DAMAGE_MULT
        } else {
            1.0
        },
        all_damage_mult: if selected == GlobalRaidModifier::AllDamage {
            GLOBAL_RAID_ALL_DAMAGE_MULT
        } else {
            1.0
        },
        attack_duration_add_seconds: if selected == GlobalRaidModifier::AttackDuration {
            GLOBAL_RAID_ATTACK_DURATION_ADD_SECONDS
        } else {
            0.0
        },
        affliction_duration_mult: if selected == GlobalRaidModifier::AfflictionDuration {
            GLOBAL_RAID_AFFLICTION_DURATION_MULT
        } else {
            1.0
        },
    }
}

#[cfg(test)]
mod global_raid_modifier_tests {
    use super::*;

    #[test]
    fn each_selection_activates_at_most_one_modifier() {
        let selections = [
            GlobalRaidModifier::None,
            GlobalRaidModifier::BurstDamage,
            GlobalRaidModifier::BurstChance,
            GlobalRaidModifier::SupportEffect,
            GlobalRaidModifier::AfflictionChance,
            GlobalRaidModifier::AfflictionDamage,
            GlobalRaidModifier::AllDamage,
            GlobalRaidModifier::AttackDuration,
            GlobalRaidModifier::AfflictionDuration,
        ];

        for selected in selections {
            let modifiers = global_raid_modifiers(selected);
            let active_count = [
                modifiers.burst_damage_mult != 1.0,
                modifiers.burst_chance_mult != 1.0,
                modifiers.support_effect_mult != 1.0,
                modifiers.affliction_chance_mult != 1.0,
                modifiers.affliction_damage_mult != 1.0,
                modifiers.all_damage_mult != 1.0,
                modifiers.attack_duration_add_seconds != 0.0,
                modifiers.affliction_duration_mult != 1.0,
            ]
            .into_iter()
            .filter(|active| *active)
            .count();

            assert_eq!(
                active_count,
                usize::from(selected != GlobalRaidModifier::None),
                "unexpected active modifier count for {selected:?}",
            );
        }
    }
}

#[derive(Debug, Clone)]
pub struct SimStats {
    pub player_stat: Arc<PlayerRaidData>,
    pub boss_stat: Boss,
    pub attackable_part: Vec<BossPartName>,
    pub usable_card: Vec<CardName>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimRunResult {
    pub total_decks: usize,
    pub total_attack_patterns: usize,
    pub rounds_per_pattern: u64,
    pub ticks_per_round: u32,
    pub decks: Vec<SimDeckResult>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimDeckResult {
    pub deck: Vec<CardName>,
    pub deck_names: Vec<String>,
    pub total_attack_patterns: usize,
    pub best_pattern: Option<SimPatternResult>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimPatternResult {
    pub pattern: String,
    pub average_damage: u64,
    pub average_damage_display: String,
    pub lowest_round_damage: u64,
    pub lowest_round_damage_display: String,
    pub highest_round_damage: u64,
    pub highest_round_damage_display: String,
    pub card_damage: Vec<SimCardDamageResult>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimCardDamageResult {
    pub card: CardName,
    pub card_name: String,
    pub average_damage: u64,
    pub average_damage_display: String,
}

pub struct SimProgress {
    pub(super) current_pattern: AtomicUsize,
    pub(super) total_patterns: usize,
}

pub(super) type DeckPatternWork = (Vec<Card>, Vec<AttackPattern>);
pub(super) type IndexedDeckPatternWork = (usize, Vec<Card>, Vec<AttackPattern>);

pub(super) struct RoundSupportCache {
    support: SupportModifiers,
    state_signature: u16,
    dynamic: bool,
}

impl RoundSupportCache {
    pub(super) fn new(deck: &mut [Card], boss: &mut Boss) -> Self {
        let support = combined_support_modifiers(deck, boss);
        boss.set_support_modifiers(support.clone());

        Self {
            support,
            state_signature: boss.part_state_signature(),
            dynamic: deck_has_dynamic_support_modifier(deck),
        }
    }

    pub(super) fn current<'a>(
        &'a mut self,
        deck: &mut [Card],
        boss: &mut Boss,
    ) -> &'a SupportModifiers {
        if self.dynamic {
            let state_signature = boss.part_state_signature();
            if state_signature != self.state_signature {
                self.support = combined_support_modifiers(deck, boss);
                self.state_signature = state_signature;
                boss.set_support_modifiers(self.support.clone());
            }
        }

        &self.support
    }

    pub(super) fn bonus_tap_proc_chance_mult(&self) -> f64 {
        self.support.bonus_tap_proc_chance_mult
    }
}

#[derive(Debug, Clone, Copy)]
pub(super) struct SimDamageContext {
    base_tap_without_part_state: f32,
    burst_add_total: f32,
    affliction_add_total: f32,
    head_armor_add: f32,
    head_body_add: f32,
    torso_armor_add: f32,
    torso_body_add: f32,
    limb_armor_add: f32,
    limb_body_add: f32,
}

impl SimDamageContext {
    pub(super) fn new(player_raid_data: &PlayerRaidData, boss: &Boss) -> Self {
        let flat_boss_add = player_raid_data.get_total_boss_add(boss.boss_name);
        let base_set_add = if player_raid_data.raid_set.jukk_juggernaut {
            100.0
        } else {
            0.0
        } + if player_raid_data.raid_set.rose_anniversary {
            100.0
        } else {
            0.0
        };
        let base_add_total = (player_raid_data.raid_card_research.base_damage
            + player_raid_data.gem_stone_research.base_damage) as f32;
        let base_tap_without_part_state = player_raid_data.player_raid_base_damage as f32
            + base_add_total
            + flat_boss_add
            + base_set_add;

        let burst_add_total = (player_raid_data.raid_card_research.base_burst_damage
            + player_raid_data.gem_stone_research.base_burst_damage)
            as f32
            + player_raid_data.get_total_card_type_boss_add(boss.boss_name, CardType::Burst)
            + if player_raid_data.raid_set.airforce_ace {
                120.0
            } else {
                0.0
            };

        let affliction_add_total = (player_raid_data.raid_card_research.base_affliction_damage
            + player_raid_data.gem_stone_research.base_affliction_damage)
            as f32
            + player_raid_data.get_total_card_type_boss_add(boss.boss_name, CardType::Affliction)
            + if player_raid_data.raid_set.dancer_venom {
                120.0
            } else {
                0.0
            };

        Self {
            base_tap_without_part_state,
            burst_add_total,
            affliction_add_total,
            head_armor_add: player_raid_data
                .get_total_part_state_add(BossPartName::Head, PartState::Armor),
            head_body_add: player_raid_data
                .get_total_part_state_add(BossPartName::Head, PartState::Body),
            torso_armor_add: player_raid_data
                .get_total_part_state_add(BossPartName::Torso, PartState::Armor),
            torso_body_add: player_raid_data
                .get_total_part_state_add(BossPartName::Torso, PartState::Body),
            limb_armor_add: player_raid_data
                .get_total_part_state_add(BossPartName::LeftHand, PartState::Armor),
            limb_body_add: player_raid_data
                .get_total_part_state_add(BossPartName::LeftHand, PartState::Body),
        }
    }

    pub(super) fn true_base_tap(self, part_name: BossPartName, state: PartState) -> f32 {
        self.base_tap_without_part_state + self.part_state_add(part_name, state)
    }

    pub(super) fn card_type_add(self, card_type: CardType) -> f32 {
        match card_type {
            CardType::Burst => self.burst_add_total,
            CardType::Affliction => self.affliction_add_total,
            CardType::Support => 0.0,
        }
    }

    pub(super) fn part_state_add(self, part_name: BossPartName, state: PartState) -> f32 {
        let is_armor = matches!(state, PartState::Armor | PartState::Cursed);

        match part_name {
            BossPartName::Head if is_armor => self.head_armor_add,
            BossPartName::Head => self.head_body_add,
            BossPartName::Torso if is_armor => self.torso_armor_add,
            BossPartName::Torso => self.torso_body_add,
            _ if is_armor => self.limb_armor_add,
            _ => self.limb_body_add,
        }
    }
}
