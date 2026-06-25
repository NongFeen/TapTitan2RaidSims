use std::fmt;

use crate::models::{boss::{BossPartName, PartState}, cards::CardType};

pub struct SupportModifiers{
    //additive
        //part mult
    pub head_damage_add : f64,
    pub torso_damage_add : f64,
    pub limb_damage_add : f64, 
        //state mult
    pub body_damage_add : f64,
    pub armor_damage_add : f64, 
        //card mult
    pub burst_damage_add : f64,
    pub affliction_damage_add : f64,
        //all dmg
    pub all_damage_add : f64,
    
    //multiplcative
    pub burst_chance_mult : f64,
    pub affliction_chance_mult : f64
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
            
            burst_chance_mult: 1.0,      
            affliction_chance_mult: 1.0, 
        }
    }
}
impl SupportModifiers {
    pub fn accumulate(mods: &[SupportModifiers]) -> SupportModifiers {
        mods.iter().fold(SupportModifiers::default(), |mut acc, m| {
            acc.head_damage_add += m.head_damage_add;
            acc.torso_damage_add += m.torso_damage_add;
            acc.limb_damage_add += m.limb_damage_add;
            acc.body_damage_add += m.body_damage_add;
            acc.armor_damage_add += m.armor_damage_add;
            acc.burst_damage_add += m.burst_damage_add;
            acc.affliction_damage_add += m.affliction_damage_add;
            acc.all_damage_add += m.all_damage_add;
            acc.burst_chance_mult += m.burst_chance_mult - 1.0;
            acc.affliction_chance_mult += m.affliction_chance_mult - 1.0;
            return acc
        })
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
            PartState::Armor => self.armor_damage_add,
            _ => 0.0,
        }
    }

    /// all_damage_add is its own separate mult slot (stacks with everything else additively).
    pub fn all_mult_bonus(&self) -> f64 {
        self.all_damage_add
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
            "SupportModifiers {{ head: +{:.0}%, torso: +{:.0}%, limb: +{:.0}%, body: +{:.0}%, armor: +{:.0}%, burst_dmg: +{:.0}%, affliction_dmg: +{:.0}%, all: +{:.0}%, burst_chance: x{:.2}, affliction_chance: x{:.2} }}",
            self.head_damage_add * 100.0,
            self.torso_damage_add * 100.0,
            self.limb_damage_add * 100.0,
            self.body_damage_add * 100.0,
            self.armor_damage_add * 100.0,
            self.burst_damage_add * 100.0,
            self.affliction_damage_add * 100.0,
            self.all_damage_add * 100.0,
            self.burst_chance_mult,
            self.affliction_chance_mult,
        )
    }
}