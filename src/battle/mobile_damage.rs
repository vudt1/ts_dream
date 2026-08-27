//! Kotlin/mobile battle damage rules.
//!
//! This module deliberately contains only combat arithmetic and outcome rolls.
//! Packet projection remains in `runner.rs`, so replacing the damage pipeline
//! does not change the PC frame format.

use crate::battle::engine::WarInfo;
use crate::battle::rng::DotNetRandom;

const MAX_BASE_DAMAGE: i64 = 50_000;
const ATTRIBUTE_INT: u8 = 27;
const ICE_BOUND_STATUS_ID: i64 = 6;

/// Mobile skill fields consumed by damage and status resolution.
#[derive(Debug, Clone, Copy, Default)]
pub struct MobileSkillInput {
    pub element: u8,
    pub numerical: i64,
    pub attribute: u8,
    pub round: i64,
    pub hit_status: i64,
}

/// Result of one mobile-formula attack roll.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MobileAttackResult {
    pub hit: bool,
    pub thunder: bool,
    pub damage: i64,
    pub inflicted_status: i64,
    pub status_rounds: i64,
}

/// Calculate one attack using the mobile formula.
///
/// `skill=None` means basic attack. The random stream is injected so tests can
/// reproduce the same result for a fixed battle seed.
pub fn calculate_attack(
    rng: &mut DotNetRandom,
    attacker: &WarInfo,
    target: &WarInfo,
    skill: Option<MobileSkillInput>,
) -> MobileAttackResult {
    let hit_rate = (90 + (attacker.agi - target.agi) / 2).clamp(5, 99);
    if rng.next_range(0, 100) >= hit_rate as i32 {
        return MobileAttackResult {
            hit: false,
            thunder: false,
            damage: 0,
            inflicted_status: 0,
            status_rounds: 0,
        };
    }

    let skill = skill.unwrap_or(MobileSkillInput {
        element: attacker.thuoctinh.clamp(0, u8::MAX as i64) as u8,
        numerical: 100,
        attribute: 0,
        round: 0,
        hit_status: 0,
    });
    let base = calculate_base_damage(attacker, skill);

    let reduced = base - target.lv - target.def;
    let element_multiplier = element_multiplier(skill.element, target.thuoctinh);
    let reborn_diff = (attacker.reborn - target.reborn).max(0);
    let reborn_multiplier = 1.0 + 0.1 * reborn_diff as f64;
    let random_factor = 0.9 + rng.next_range(0, 10_000) as f64 / 10_000.0 * 0.2;
    let thunder = rng.next_range(0, 100) < (5 + attacker.agi / 10).clamp(1, 50) as i32;
    let thunder_multiplier = if thunder { 1.5 } else { 1.0 };
    let pvp_multiplier = if target.typ == 2 || target.typ == 4 {
        0.5
    } else {
        1.0
    };
    let status_multiplier = if target.type3_id == ICE_BOUND_STATUS_ID {
        1.5
    } else {
        1.0
    };
    let defend_multiplier = if target.id_skill == 17_001 { 0.5 } else { 1.0 };

    let damage = (reduced as f64
        * element_multiplier
        * reborn_multiplier
        * random_factor
        * thunder_multiplier
        * pvp_multiplier
        * status_multiplier
        * defend_multiplier)
        .round()
        .max(1.0) as i64;

    let (inflicted_status, status_rounds) = if skill.hit_status > 0 && skill.round > 0 {
        let status_chance = (50 + (attacker.int1 - target.int1) / 2).clamp(10, 90);
        if rng.next_range(0, 100) < status_chance as i32 {
            (skill.hit_status, skill.round)
        } else {
            (0, 0)
        }
    } else {
        (0, 0)
    };

    MobileAttackResult {
        hit: true,
        thunder,
        damage,
        inflicted_status,
        status_rounds,
    }
}

fn calculate_base_damage(attacker: &WarInfo, skill: MobileSkillInput) -> i64 {
    let level_damage = attacker.lv * 2;
    let base = if skill.attribute == ATTRIBUTE_INT {
        let base_damage = (attacker.int1 as f64 * 0.5 + attacker.lv as f64) as i64;
        base_damage * skill.numerical / 100 + level_damage
    } else if skill.numerical == 100 && skill.attribute == 0 {
        let base_damage = (attacker.atk as f64 * 0.6 + attacker.lv as f64) as i64;
        base_damage + level_damage
    } else {
        let base_damage =
            (attacker.atk as f64 * 0.6 + attacker.int1 as f64 * 0.1 + attacker.lv as f64) as i64;
        base_damage * skill.numerical / 100 + level_damage
    };
    base.clamp(i64::MIN / 2, MAX_BASE_DAMAGE)
}

/// Mobile element table: earth→water→fire→wind→earth; light↔dark.
pub fn element_multiplier(attacker: u8, defender: i64) -> f64 {
    if attacker == defender.clamp(0, u8::MAX as i64) as u8
        || matches!(attacker, 5 | 6)
        || matches!(defender, 5 | 6)
    {
        return 1.0;
    }
    match (attacker, defender) {
        (1, 2) | (2, 3) | (3, 4) | (4, 1) | (7, 8) | (8, 7) => 1.5,
        (1, 4) | (2, 1) | (3, 2) | (4, 3) => 0.75,
        _ => 1.0,
    }
}
