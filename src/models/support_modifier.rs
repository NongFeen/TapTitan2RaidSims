use std::fmt;

use crate::models::{
    boss::{BossPartName, PartState},
    cards::CardType,
};

#[derive(Debug, Clone)]
pub struct SupportModifiers {
    //additive
    //part mult
    pub head_damage_add: f64,
    pub torso_damage_add: f64,
    pub limb_damage_add: f64,
    //state mult
    pub body_damage_add: f64,
    pub armor_damage_add: f64,
    //card mult
    pub burst_damage_add: f64,
    pub affliction_damage_add: f64,
    //all dmg
    pub all_damage_add: f64,

    //damage multipliers
    pub burst_damage_mult: f64,
    pub affliction_damage_mult: f64,
    pub all_damage_mult: f64,

    //raid flow
    pub attack_duration_add_seconds: f64,

    //multiplcative
    pub burst_chance_mult: f64,
    pub affliction_chance_mult: f64,
    pub bonus_tap_proc_chance_mult: f64,
}
impl Default for SupportModifiers {
    fn default() -> Self {
        Self {
            head_damage_add: 0.0,
            torso_damage_add: 0.0,
            limb_damage_add: 0.0,
            body_damage_add: 0.0,
            armor_damage_add: 0.0,
            burst_damage_add: 0.0,
            affliction_damage_add: 0.0,
            all_damage_add: 0.0,
            burst_damage_mult: 1.0,
            affliction_damage_mult: 1.0,
            all_damage_mult: 1.0,
            attack_duration_add_seconds: 0.0,

            burst_chance_mult: 1.0,
            affliction_chance_mult: 1.0,
            bonus_tap_proc_chance_mult: 1.0,
        }
    }
}
impl SupportModifiers {
    pub fn accumulate(mods: &[SupportModifiers]) -> SupportModifiers {
        mods.iter().fold(SupportModifiers::default(), |mut acc, m| {
            acc.merge(m);
            return acc;
        })
    }

    pub fn merge(&mut self, other: &SupportModifiers) {
        self.head_damage_add += other.head_damage_add;
        self.torso_damage_add += other.torso_damage_add;
        self.limb_damage_add += other.limb_damage_add;
        self.body_damage_add += other.body_damage_add;
        self.armor_damage_add += other.armor_damage_add;
        self.burst_damage_add += other.burst_damage_add;
        self.affliction_damage_add += other.affliction_damage_add;
        self.all_damage_add += other.all_damage_add;
        self.burst_damage_mult *= other.burst_damage_mult;
        self.affliction_damage_mult *= other.affliction_damage_mult;
        self.all_damage_mult *= other.all_damage_mult;
        self.attack_duration_add_seconds += other.attack_duration_add_seconds;
        self.burst_chance_mult *= other.burst_chance_mult;
        self.affliction_chance_mult *= other.affliction_chance_mult;
        self.bonus_tap_proc_chance_mult *= other.bonus_tap_proc_chance_mult;
    }

    pub fn scale_effects(mut self, effect_mult: f64) -> Self {
        self.head_damage_add *= effect_mult;
        self.torso_damage_add *= effect_mult;
        self.limb_damage_add *= effect_mult;
        self.body_damage_add *= effect_mult;
        self.armor_damage_add *= effect_mult;
        self.burst_damage_add *= effect_mult;
        self.affliction_damage_add *= effect_mult;
        self.all_damage_add *= effect_mult;
        self.attack_duration_add_seconds *= effect_mult;

        self.burst_damage_mult = scale_multiplier_effect(self.burst_damage_mult, effect_mult);
        self.affliction_damage_mult =
            scale_multiplier_effect(self.affliction_damage_mult, effect_mult);
        self.all_damage_mult = scale_multiplier_effect(self.all_damage_mult, effect_mult);
        self.burst_chance_mult = scale_multiplier_effect(self.burst_chance_mult, effect_mult);
        self.affliction_chance_mult =
            scale_multiplier_effect(self.affliction_chance_mult, effect_mult);
        self.bonus_tap_proc_chance_mult =
            scale_multiplier_effect(self.bonus_tap_proc_chance_mult, effect_mult);

        self
    }

    /// Bonus that stacks into part_mult, based on which part was attacked.
    pub fn part_mult_bonus(&self, attack_part: BossPartName) -> f64 {
        match attack_part {
            BossPartName::Head => self.head_damage_add,
            BossPartName::Torso => self.torso_damage_add,
            p if p.is_limb() => self.limb_damage_add,
            _ => 0.0,
        }
    }

    /// Bonus that stacks into state_mult, based on the part's current state.
    pub fn state_mult_bonus(&self, state: PartState) -> f64 {
        match state {
            PartState::Body => self.body_damage_add,
            PartState::Armor | PartState::Cursed => self.armor_damage_add,
            _ => 0.0,
        }
    }

    /// all_damage_add is its own separate mult slot (stacks with everything else additively).
    pub fn all_mult_bonus(&self) -> f64 {
        self.all_damage_add
    }

    pub fn damage_multiplier(&self, card_type: Option<CardType>) -> f64 {
        let type_mult = match card_type {
            Some(CardType::Burst) => self.burst_damage_mult,
            Some(CardType::Affliction) => self.affliction_damage_mult,
            _ => 1.0,
        };

        self.all_damage_mult * type_mult
    }

    pub fn total_damage_bonus(
        &self,
        attack_part: BossPartName,
        state: PartState,
        card_type: Option<CardType>, // None for tap damage, Some(Burst/Affliction) for procs
    ) -> f64 {
        let part_bonus = self.part_mult_bonus(attack_part);
        let state_bonus = self.state_mult_bonus(state);
        let type_bonus = match card_type {
            Some(CardType::Burst) => self.burst_damage_add,
            Some(CardType::Affliction) => self.affliction_damage_add,
            _ => 0.0,
        };
        // part_bonus + state_bonus + type_bonus
        part_bonus + state_bonus + type_bonus + self.all_damage_add
    }
}

impl fmt::Display for SupportModifiers {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "SupportModifiers {{ head: +{}%, torso: +{}%, limb: +{}%, body: +{}%, armor: +{}%, burst_dmg: +{}%, affliction_dmg: +{}%, all: +{}%, burst_mult: x{:.2}, affliction_mult: x{:.2}, all_mult: x{:.2}, duration: {}s, burst_chance: x{:.2}, affliction_chance: x{:.2}, bonus_tap_proc_chance: x{:.2} }}",
            self.head_damage_add * 100.0,
            self.torso_damage_add * 100.0,
            self.limb_damage_add * 100.0,
            self.body_damage_add * 100.0,
            self.armor_damage_add * 100.0,
            self.burst_damage_add * 100.0,
            self.affliction_damage_add * 100.0,
            self.all_damage_add * 100.0,
            self.burst_damage_mult,
            self.affliction_damage_mult,
            self.all_damage_mult,
            self.attack_duration_add_seconds,
            self.burst_chance_mult,
            self.affliction_chance_mult,
            self.bonus_tap_proc_chance_mult,
        )
    }
}

fn scale_multiplier_effect(mult: f64, effect_mult: f64) -> f64 {
    (1.0 + ((mult - 1.0) * effect_mult)).max(0.0)
}
