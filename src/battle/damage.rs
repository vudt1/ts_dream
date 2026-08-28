//! Damage helpers used by `runner.rs` (Chapter 6 §6.5).
//!
//! Physical/magic damage formulas live in `combat_formula.rs` (the authoritative
//! pipeline for the PC server per ADR 0002). This module retains only the
//! helpers `runner.rs` still consumes: per-roll RNG (`randomize_with_percent`,
//! `randomize_array`), per-skill outcome rolls (`get_random_miss_*`,
//! `get_random_skill_npc`, `get_random_drop`), per-flush exp/drop rewards
//! (`hit_exp`, `calc_kill_exp`, `calc_combo_exp`), and the per-skill turn
//! table (`get_turn`). Banker's rounding (`banker_round`) is shared.
//!
//! All arithmetic uses double precision with banker's rounding (round half
//! to even), matching the reference implementation exactly.

use crate::battle::rng::DotNetRandom;
use crate::data::tables::Npc;

/// Banker's rounding (round half to even).
pub fn banker_round(x: f64) -> f64 {
    x.round_ties_even()
}

/// Attack miss roll — returns `1` (hit) or `0` (miss).
/// `percent = 100 + round((lv1-lv2)/10) + round((lvtb1-lvtb2)/10)`.
pub fn get_random_miss_attack(
    rng: &mut DotNetRandom,
    lv1: i64,
    lv2: i64,
    lvtb1: i64,
    lvtb2: i64,
) -> i64 {
    let num = banker_round((lv1 - lv2) as f64 / 10.0) as i64;
    let num2 = banker_round((lvtb1 - lvtb2) as f64 / 10.0) as i64;
    let percent = 100 + num + num2;
    randomize_with_percent(rng, 1, 0, percent)
}

/// Status-effect land roll. Returns 1 (lands) / 0 (miss).
/// `percent = 30 + max(int1,atk1)/30 - spx2/30 + round((lv1-lv2)/20) + round((lvtb1-lvtb2)/20) + reborn1*5 - reborn2*5`.
#[allow(clippy::too_many_arguments)]
pub fn get_random_miss_troi(
    rng: &mut DotNetRandom,
    lv1: i64,
    lv2: i64,
    lvtb1: i64,
    lvtb2: i64,
    int1: i64,
    atk1: i64,
    spx2: i64,
    reborn1: i64,
    reborn2: i64,
) -> i64 {
    let num = banker_round((lv1 - lv2) as f64 / 20.0) as i64;
    let num2 = banker_round((lvtb1 - lvtb2) as f64 / 20.0) as i64;
    let num3 = atk1.max(int1);
    let mut percent = 30 + num3 / 30;
    percent -= spx2 / 30;
    percent += num;
    percent += num2;
    percent += reborn1 * 5;
    percent -= reborn2 * 5;
    randomize_with_percent(rng, 1, 0, percent)
}

/// Flee roll. `percent = 60 + (lv1-lv2) + (lvtb1-lvtb2)`.
pub fn get_random_miss_flee(
    rng: &mut DotNetRandom,
    lv1: i64,
    lv2: i64,
    lvtb1: i64,
    lvtb2: i64,
) -> i64 {
    let num = lv1 - lv2;
    let num2 = lvtb1 - lvtb2;
    let percent = 60 + num + num2;
    // Flee percent may exceed 100 → clamp (the percent roll clamps to [0,100]).
    randomize_with_percent(rng, 1, 0, percent)
}

/// Combo roll — always hits (percent 100) => returns 1.
pub fn get_random_miss_combo(rng: &mut DotNetRandom) -> i64 {
    randomize_with_percent(rng, 1, 0, 100)
}

/// Sequentially fold pairwise 50% rolls across `items`.
/// Returns the surviving element of `items` (0 when empty).
pub fn randomize_array(rng: &mut DotNetRandom, items: &[i64]) -> i64 {
    if items.is_empty() {
        return 0;
    }
    let mut value = items[0];
    for &item in &items[1..] {
        value = randomize_with_percent(rng, value, item, 50);
    }
    value
}

/// Pairwise percent pick between `value1`/`value2` — clamped to [0,100].
/// negative percent behaves like 0 (roll `<= p*10` always false → value2).
pub fn randomize_with_percent(
    rng: &mut DotNetRandom,
    value1: i64,
    value2: i64,
    percent: i64,
) -> i64 {
    let p = percent.clamp(0, 100);
    let roll = rng.next_range(1, 1000);
    if i64::from(roll) <= p * 10 {
        value1
    } else {
        value2
    }
}

/// NPC skill pick — defaults missing skills to 10000.
/// Draws a fresh 1..100 roll per attempt (RNG-parity sensitive).
pub fn get_random_skill_npc(
    rng: &mut DotNetRandom,
    _lv: i64,
    reborn: i64,
    skills: [i64; 4],
) -> i64 {
    let s1 = if skills[0] == 0 { 10000 } else { skills[0] };
    let s2 = if skills[1] == 0 { 10000 } else { skills[1] };
    let s3 = if skills[2] == 0 { 10000 } else { skills[2] };
    if i64::from(rng.next_range(1, 100)) <= 5 * (reborn + 1) {
        return s3;
    }
    if i64::from(rng.next_range(1, 100)) <= 15 * (reborn + 1)
        && i64::from(rng.next_range(1, 100)) > 5 * reborn
    {
        return s2;
    }
    if i64::from(rng.next_range(1, 100)) <= 30 * (reborn + 1)
        && i64::from(rng.next_range(1, 100)) > 15 * reborn
    {
        return s2;
    }
    s1
}

/// Drop band roll against cumulative drop percent widths.
/// `percents` = `[percent_item1..6]` (server defaults 25,23,20,4,3,1). Returns the
/// granted item id (from npc `item[0..6]`) or 0 when the roll falls past the bands.
pub fn get_random_drop(rng: &mut DotNetRandom, npc: &Npc, percents: &[i64; 6]) -> i64 {
    let item: [i64; 6] = npc.item;
    let num = i64::from(rng.next_range(1, 1000));
    let mut lo = 0i64;
    for (i, &band) in percents.iter().enumerate() {
        let hi = lo + band;
        if num > lo && num <= hi {
            return item[i];
        }
        lo = hi;
    }
    0
}

/// Band-slot variant used by unit tests — returns 1..6 (slot) or 0 (no drop).
/// Bands: 25, 23, 20, 4, 3, 1 (cumulative thresholds).
pub fn get_random_drop_slot(roll: i32) -> usize {
    let r = roll as i64;
    if r <= 25 {
        1
    } else if r <= 48 {
        2
    } else if r <= 68 {
        3
    } else if r <= 72 {
        4
    } else if r <= 75 {
        5
    } else if r <= 76 {
        6
    } else {
        0
    }
}

/// In-turn per-hit exp for TeamDef (type-7) targets.
/// `round(npcLv / 2.0 + (npcLv - attackerLv))`.
pub fn hit_exp(attacker_lv: i64, npc_lv: i64) -> i64 {
    banker_round(npc_lv as f64 / 2.0 + (npc_lv - attacker_lv) as f64) as i64
}

/// Per-flush exp from the level-diff table (§3.6 / §7.2).
/// Returns base exp for a kill.
pub fn calc_kill_exp(caster_lv: i64, npc_lv: i64) -> i64 {
    let diff = caster_lv - npc_lv;
    if diff <= 20 {
        if diff < 0 {
            banker_round((npc_lv - caster_lv) as f64 + npc_lv as f64 / 5.0) as i64
        } else {
            match diff {
                0..=2 => banker_round(5.0 + npc_lv as f64 / 5.0) as i64,
                3..=5 => banker_round(4.0 + npc_lv as f64 / 5.0) as i64,
                6..=10 => banker_round(3.0 + npc_lv as f64 / 5.0) as i64,
                11..=15 => banker_round(2.0 + npc_lv as f64 / 5.0) as i64,
                16..=20 => banker_round(1.0 + npc_lv as f64 / 5.0) as i64,
                _ => 0,
            }
        }
    } else {
        0
    }
}

/// Combo exp bonus: `round(base * 1.086)`.
pub fn calc_combo_exp(base_exp: i64) -> i64 {
    banker_round(base_exp as f64 * 1.086) as i64
}

/// Buff/debuff duration in turns (§5.4).
pub fn get_turn(id_skill: i64, lv_skill: i64) -> i64 {
    // GROUP_a: {13002,14008,13003,13005,13012}
    if matches!(id_skill, 13002 | 14008 | 13003 | 13005 | 13012) {
        return if lv_skill - 1 > 1 {
            if lv_skill - 3 <= 2 {
                3
            } else {
                2
            }
        } else {
            3
        };
    }
    // GROUP_b: {10033,10015,10026,13021,13025,13032,10025,14020,12025,14040,14044,14046,14053}
    if matches!(
        id_skill,
        10033
            | 10015
            | 10026
            | 13021
            | 13025
            | 13032
            | 10025
            | 14020
            | 12025
            | 14040
            | 14044
            | 14046
            | 14053
    ) {
        return match lv_skill {
            1 | 2 => 2,
            3 | 4 => 3,
            5 => 4,
            _ => 3,
        };
    }
    // GROUP_c: {10004,11002,12024,13011,13030,14015,14029,20018,11024,11032,13020}
    if matches!(
        id_skill,
        10004 | 11002 | 12024 | 13011 | 13030 | 14015 | 14029 | 20018 | 11024 | 11032 | 13020
    ) {
        return 3;
    }
    // GROUP_d: {13015,13016,13017,13018,10016,10017,10018,10019}
    if matches!(
        id_skill,
        13015 | 13016 | 13017 | 13018 | 10016 | 10017 | 10018 | 10019
    ) {
        return 4;
    }
    // GROUP_e: {11014,20014,20022,20023}
    if matches!(id_skill, 11014 | 20014 | 20022 | 20023) {
        return 5;
    }
    // GROUP_f: {20025,20026,20027,10010,10031,13014,20024,14012}
    if matches!(
        id_skill,
        20025 | 20026 | 20027 | 10010 | 10031 | 13014 | 20024 | 14012
    ) {
        return match lv_skill {
            1..=3 => 2,
            4..=6 => 3,
            7..=9 => 4,
            10 => 5,
            _ => 3,
        };
    }
    // Inner switch: 14021 returns via its own ladder; 14013 falls through to GROUP_f's.
    match id_skill {
        14021 => match lv_skill {
            1 => 2,
            2 => 3,
            3 => 4,
            4 => 5,
            5 => 6,
            _ => 3,
        },
        14013 => match lv_skill {
            1..=3 => 2,
            4..=6 => 3,
            7..=9 => 4,
            10 => 5,
            _ => 3,
        },
        _ => 3,
    }
}
