use rand::seq::IndexedRandom;
use serde::{Deserialize, Serialize};
use strum_macros::{EnumIter, EnumString};

use crate::models::affliction::Affliction;
use crate::models::damage_source::DamageSource;

#[derive(Debug, Deserialize, Serialize, PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Copy, EnumIter, EnumString)]
pub enum BossPartName{
    Head,
    Torso,
    LeftShoulder,
    RightShoulder,
    LeftHand,
    RightHand,
    LeftLeg,
    RightLeg,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq,Deserialize, Serialize)]
pub enum PartState {
    Cursed,
    Armor,
    Body,
    Skeleton
}
#[derive(Debug, Clone, Copy, PartialEq, Eq,Deserialize, Serialize)]
pub enum BossName{
    Lojak,
    Takedar,
    Jukk,
    Sterl,
    Mohaca,
    Terro,
    Klonk,
    Priker
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DamageResult {
    pub source: DamageSource,
    pub damage: u64,
}

#[derive(Debug, Clone,Deserialize, Serialize)]
pub struct BossPart{
   pub part_name: BossPartName,
   pub part_state: PartState,
   pub max_armor: u64,
   pub max_health: u64,
   pub current_armor: u64,
   pub current_health: u64,
   #[serde(default)]
   pub afflictions: Vec<Affliction>,
}
impl BossPart {
    pub fn new(part:BossPartName,state:PartState,m_armor:u64 ,m_health:u64,c_armor:u64 ,c_health:u64) -> Self {
        Self {
            part_name:part,
            part_state:state,
            max_armor:m_armor,
            max_health:m_health,
            current_armor:c_armor ,
            current_health:c_health,
            afflictions: Vec::new(),
        }
    }
    pub fn is_limb(&self) -> bool {
        self.part_name.is_limb()
    }
    pub fn update(&mut self){
        self.tick_afflictions();
    }
    pub fn apply_affliction(&mut self, affliction: Affliction) {
        let incoming_stack_count = affliction.stack_count() as u8;
        let incoming_duration = affliction
            .stacks
            .first()
            .map(|stack| stack.attached_duration)
            .unwrap_or(0);
        let incoming_tick_damage = affliction.damage_per_tick;
        let incoming_expire_damage = affliction.expire_damage_per_duration;

        if let Some(existing) = self
            .afflictions
            .iter_mut()
            .find(|current| current.kind == affliction.kind)
        {
            existing.apply_stacks(incoming_stack_count, incoming_duration);
            existing.damage_per_tick = existing.damage_per_tick.max(incoming_tick_damage);
            existing.expire_damage_per_duration = existing
                .expire_damage_per_duration
                .max(incoming_expire_damage);
            return;
        }

        self.afflictions.push(affliction);
    }
    fn tick_afflictions(&mut self) {
        let mut total_damage = 0u64;

        for affliction in &mut self.afflictions {
            total_damage = total_damage.saturating_add(affliction.tick());
        }

        self.afflictions.retain(|affliction| !affliction.is_expired());

        if total_damage > 0 {
            self.on_hit(total_damage);
        }
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
            PartState::Skeleton => {}//do nothing
        }
    }
}


impl BossPartName {
    pub fn is_limb(&self) -> bool {
        match self {
            BossPartName::Head | BossPartName::Torso => false,
            _ => true, // Everything else (shoulders, hands, legs) are limbs
        }
    }
}
#[derive(Debug,Clone, Deserialize, Serialize)]

pub struct Boss {
    pub boss_name: BossName,
    pub head: BossPart,
    pub torso: BossPart,
    pub left_shoulder: BossPart,
    pub right_shoulder: BossPart,
    pub left_hand: BossPart,
    pub right_hand: BossPart,
    pub left_leg: BossPart,
    pub right_leg: BossPart,
    #[serde(default)]
    pub damage_results: Vec<DamageResult>,
}

impl Boss {
    pub fn part_mut(&mut self, part_name: BossPartName) -> &mut BossPart {
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

    pub fn apply_affliction(&mut self, part_name: BossPartName, affliction: Affliction) {
        match part_name {
            BossPartName::Head => self.head.apply_affliction(affliction),
            BossPartName::Torso => self.torso.apply_affliction(affliction),
            BossPartName::LeftShoulder => self.left_shoulder.apply_affliction(affliction),
            BossPartName::RightShoulder => self.right_shoulder.apply_affliction(affliction),
            BossPartName::LeftHand => self.left_hand.apply_affliction(affliction),
            BossPartName::RightHand => self.right_hand.apply_affliction(affliction),
            BossPartName::LeftLeg => self.left_leg.apply_affliction(affliction),
            BossPartName::RightLeg => self.right_leg.apply_affliction(affliction),
        }
    }

    fn parts_mut(&mut self) -> [&mut BossPart; 8] {
        [
            &mut self.head,
            &mut self.torso,
            &mut self.left_shoulder,
            &mut self.right_shoulder,
            &mut self.left_hand,
            &mut self.right_hand,
            &mut self.left_leg,
            &mut self.right_leg,
        ]
    }

    pub fn update(&mut self) {
        // println!("--- Running Boss Update Tick ---");
        for part in self.parts_mut() {
            part.update();
        }
    }
    pub fn record_damage(&mut self, source: DamageSource, damage: u64) {
        let source_label = source.label();

        if let Some(existing) = self
            .damage_results
            .iter_mut()
            .find(|entry| entry.source.label() == source_label)
        {
            existing.damage = existing.damage.saturating_add(damage);
            return;
        }

        self.damage_results.push(DamageResult { source, damage });
    }

    pub fn on_hit_with_source(
        &mut self,
        part_name: BossPartName,
        damage: u64,
        source: DamageSource,
    ) {
        self.record_damage(source, damage);
        self.on_hit(part_name, damage);
    }
    pub fn on_hit(&mut self, part_name:BossPartName, damage:u64){
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
    pub fn get_state_from_part(&self, part_name: BossPartName) -> PartState{
        self.part(part_name).part_state
    }

    pub fn get_total_damage(&self) -> u64 {
    self.damage_results
        .iter()
        .map(|entry| entry.damage)
        .sum()
    }

    pub fn get_damage_result(&self) -> String {
    self.damage_results
        .iter()
        .map(|entry| format!("{} : {}", entry.source.label(), Self::format_compact(entry.damage)))
        .collect::<Vec<_>>()
        .join("\n")
    }

    #[allow(non_snake_case)]
    pub fn getDamageResult(&self) -> String {
        self.get_damage_result()
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
    fn format_compact(damage: u64) -> String {
    let damage_f = damage as f64;
    if damage >= 1_000_000_000_000 {
        format!("{:.12}T", damage_f / 1_000_000_000_000.0)
    } else if damage >= 1_000_000_000 {
        format!("{:.9}B", damage_f / 1_000_000_000.0)
    } else if damage >= 1_000_000 {
        format!("{:.6}M", damage_f / 1_000_000.0)
    } else if damage >= 1_000 {
        format!("{:.3}K", damage_f / 1_000.0)
    } else {
        damage.to_string()
    }
}
}
