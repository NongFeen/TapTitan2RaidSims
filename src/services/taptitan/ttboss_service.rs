use crate::models::ttboss::{Boss, BossPart, DamageResult, PartId, PartState};

pub fn new_boss(name: &str) -> Boss {
    Boss {
        name: name.into(),
        real_hp: 5000.0,
        max_real_hp: 5000.0,
        parts: vec![
            new_part(PartId::LeftHand,      500.0, 200.0, false),
            new_part(PartId::RightHand,     500.0, 200.0, false),
            new_part(PartId::LeftShoulder,  600.0, 250.0, false),
            new_part(PartId::RightShoulder, 600.0, 250.0, false),
            new_part(PartId::LeftLeg,       700.0, 300.0, false),
            new_part(PartId::RightLeg,      700.0, 300.0, false),
            new_part(PartId::Head,          800.0, 400.0, true),
            new_part(PartId::Torso,        1200.0, 500.0, false),
        ],
    }
}

fn new_part(id: PartId, max_hp: f64, max_armor: f64, is_cursed: bool) -> BossPart {
    let state = if is_cursed {
        PartState::Cursed
    } else {
        PartState::Armored
    };

    BossPart {
        id,
        state,
        max_hp,
        current_hp: max_hp,
        max_armor,
        current_armor: max_armor,
        is_cursed,
    }
}

pub fn attack_part(boss: &mut Boss, id: &PartId, damage: f64) -> DamageResult {
    match boss.parts.iter_mut().find(|p| &p.id == id) {
        Some(part) => {
            let mut result = take_damage(part, damage);
            if result.hp_damage > 0.0 {
                boss.real_hp = (boss.real_hp - result.hp_damage).max(0.0);
            }
            result.real_hp_remaining = boss.real_hp;
            result
        }
        None => DamageResult { was_blocked: true, ..Default::default() },
    }
}

fn take_damage(part: &mut BossPart, amount: f64) -> DamageResult {
    if part.state == PartState::Broken {
        return DamageResult { was_blocked: true, ..Default::default() };
    }

    let mut result = DamageResult::default();
    let has_armor = matches!(part.state, PartState::Armored | PartState::Cursed);

    if has_armor {
        let absorbed = amount.min(part.current_armor);
        part.current_armor -= absorbed;
        result.armor_damage = absorbed;
        if part.current_armor <= 0.0 {
            result.armor_broken = true;
        }
    } else {
        let hp_damage = amount.min(part.current_hp);
        part.current_hp -= hp_damage;
        result.hp_damage = hp_damage;
        if part.current_hp <= 0.0 {
            result.part_broken = true;
        }
    }

    refresh_state(part);
    result
}

fn refresh_state(part: &mut BossPart) {
    part.state = if part.current_hp <= 0.0 {
        PartState::Broken
    } else if part.current_armor <= 0.0 {
        PartState::Body
    } else if part.is_cursed {
        PartState::Cursed
    } else {
        PartState::Armored
    };
}

pub fn has_active_curse(boss: &Boss) -> bool {
    boss.parts.iter().any(|p| p.state == PartState::Cursed)
}

pub fn is_defeated(boss: &Boss) -> bool {
    boss.real_hp <= 0.0
}

pub fn alive_parts(boss: &Boss) -> Vec<&BossPart> {
    boss.parts.iter().filter(|p| p.state != PartState::Broken).collect()
}