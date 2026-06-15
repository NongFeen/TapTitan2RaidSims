use std::clone;

use serde::{Deserialize, Serialize};
use strum_macros::{EnumIter, EnumString};

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

#[derive(Debug, Clone,Deserialize, Serialize)]
pub struct BossPart{
   pub part_name: BossPartName,
   pub part_state: PartState,
   pub max_armor: u64,
   pub max_health: u64,
   pub current_armor: u64,
   pub current_health: u64,
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
        }
    }
    pub fn is_limb(&self) -> bool {
        self.part_name.is_limb()
    }
    pub fn update(&mut self){
        // update debuff tick
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
}

impl Boss {
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
        println!("--- Running Boss Update Tick ---");
        for part in self.parts_mut() {
            part.update();
        }
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
}
