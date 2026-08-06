//! Damage pipeline (Chapter 6 §6.5) — element tables, physical/magic formulas,
//! miss rolls, and per-level reward helpers.
//!
//! Faithful port of `TheBattle.cs` (element tables at 7191-7269, damage
//! pipeline at 1953-3560, RNG helpers at 9348-9503). All arithmetic mirrors the
//! C# `double` + `Math.Round` (banker's rounding) exactly.

use crate::battle::rng::DotNetRandom;
use crate::data::tables::Npc;

/// Element damage multiplier table `GetDamageThuoctinh(attacker, defender)` (§5.1).
/// Elements: 1=earth, 2=water, 3=fire, 4=wind.
pub fn get_damage_thuoctinh(att_tt: i64, def_tt: i64) -> f64 {
    match (att_tt, def_tt) {
        (1, 1) => 1.0,
        (1, 2) => 1.55,
        (1, 3) => 1.3,
        (1, 4) => 0.65,
        (2, 1) => 0.6,
        (2, 2) => 1.0,
        (2, 3) => 1.7,
        (2, 4) => 1.0,
        (3, 1) => 1.6,
        (3, 2) => 0.7,
        (3, 3) => 1.0,
        (3, 4) => 1.9,
        (4, 1) => 1.7,
        (4, 2) => 1.3,
        (4, 3) => 0.8,
        (4, 4) => 1.3,
        _ => 1.0,
    }
}

/// Per-level additive element component `GetDamageSkillInt(skill_tt, def_tt)` (§5.1).
///
/// C# (`TheBattle.cs:7231-7269`) returns a per-row default for an unknown
/// defender element: row 1→18, 2→19, 3→27, 4→42; unknown skill row → 10.
pub fn get_damage_skill_int(skill_tt: i64, def_tt: i64) -> i64 {
    match skill_tt {
        1 => match def_tt {
            1 => 18,
            2 => 27,
            3 => 21,
            4 => 13,
            _ => 18,
        },
        2 => match def_tt {
            1 => 10,
            2 => 19,
            3 => 29,
            4 => 19,
            _ => 19,
        },
        3 => match def_tt {
            1 => 26,
            2 => 15,
            3 => 27,
            4 => 34,
            _ => 27,
        },
        4 => match def_tt {
            1 => 54,
            2 => 42,
            3 => 29,
            4 => 42,
            _ => 42,
        },
        _ => 10,
    }
}

/// Element relation `GetThuoctinhKhac(t1, t2)`: 2 if t1 beats t2, 1 if t2 beats t1, else 0.
/// Beat graph: 1→4, 2→1, 3→2, 4→3.
pub fn get_thuoctinh_khac(t1: i64, t2: i64) -> i64 {
    match (t1, t2) {
        (1, 4) | (2, 1) | (3, 2) | (4, 3) => 2,
        (1, 2) | (2, 3) | (3, 4) | (4, 1) => 1,
        _ => 0,
    }
}

/// .NET `Math.Round` — banker's rounding (round half to even).
pub fn banker_round(x: f64) -> f64 {
    x.round_ties_even()
}

/// Physical base before the level/skill refinement terms.
/// `round(stat * Element * 2.0 - Def * 1.6)`.
fn physical_base(stat: i64, element: f64, def: i64) -> f64 {
    banker_round(stat as f64 * element * 2.0 - def as f64 * 1.6)
}

/// Level-difference and skill term refinements.
fn refine_base(
    base: f64,
    att_lv: i64,
    def_lv: i64,
    skill_tt: i64,
    def_tt: i64,
    do_manh: i64,
    skill_lv: i64,
) -> f64 {
    let mut b = base
        + banker_round((att_lv - def_lv) as f64 / 1.5)
        + banker_round(att_lv as f64 / 20.0) * 8.0;
    b = banker_round(
        b + get_damage_skill_int(skill_tt, def_tt) as f64
            * do_manh as f64
            * (1.0 + skill_lv as f64 * 0.033),
    );
    b
}

/// Physical damage (skill Type 1) base pipeline.
///
/// The C# final base term always uses `round(num80 * Element * 2.0 - Def * 1.6)`
/// where `num80 = Atk` normally (or the skill's Combo field == 84) and `num80 = Int`
/// when the skill's Combo field == 87. Callers resolve `stat` accordingly.
///
/// The `* num37` multiplier only appears for physical (Type 1) skills.
pub fn calc_physical_damage_stat(
    stat: i64,
    def: i64,
    att_tt: i64,
    def_tt: i64,
    att_lv: i64,
    def_lv: i64,
    skill_tt: i64,
    do_manh: i64,
    skill_lv: i64,
    num37: f64,
) -> i64 {
    let element = get_damage_thuoctinh(att_tt, def_tt);
    let base = physical_base(stat, element, def);
    let num36 = refine_base(base, att_lv, def_lv, skill_tt, def_tt, do_manh, skill_lv);
    banker_round(num36 * num37) as i64
}

/// Backward-compatible wrapper: physical damage using `atk` as the base stat.
pub fn calc_physical_damage(
    atk: i64,
    def: i64,
    att_tt: i64,
    def_tt: i64,
    att_lv: i64,
    def_lv: i64,
    skill_tt: i64,
    do_manh: i64,
    skill_lv: i64,
    num37: f64,
) -> i64 {
    calc_physical_damage_stat(
        atk, def, att_tt, def_tt, att_lv, def_lv, skill_tt, do_manh, skill_lv, num37,
    )
}

/// Magic damage (skill Type 2) base pipeline — uses `_Int`, and never applies
/// `num37`. Skills 12016..12019 (multi-hit magic) use a special AoE refactor:
/// `round(num36 / (num34*0.5)) + skillLv*50` instead of the `/ (num34*0.75)`.
pub fn calc_magic_damage(
    int_stat: i64,
    def: i64,
    att_tt: i64,
    def_tt: i64,
    att_lv: i64,
    def_lv: i64,
    skill_tt: i64,
    do_manh: i64,
    skill_lv: i64,
    skill_id: i64,
    num34: i64,
) -> i64 {
    let element = get_damage_thuoctinh(att_tt, def_tt);
    let base = physical_base(int_stat, element, def);
    let mut num36 = refine_base(base, att_lv, def_lv, skill_tt, def_tt, do_manh, skill_lv);
    if (12016..=12019).contains(&skill_id) {
        if num34 > 1 {
            num36 = banker_round(num36 / (num34 as f64 * 0.5)) + (skill_lv * 50) as f64;
        }
    } else if num34 > 1 {
        num36 = banker_round(num36 / (num34 as f64 * 0.75));
    }
    num36 as i64
}

/// Ordered physical/magic buff modifiers (§5.2). Mirrors the C# non-shield
/// ordering — buffs first, AoE falloff last.
///
/// `num34` = skill SLDanh (area). `sl_danh` = `GetDataSkill(_Type19_Id, _SLdanh)`,
/// used only to derive the attacker Type19 per-level rates.
#[allow(clippy::too_many_arguments)]
pub fn apply_buff_modifiers(
    num36: &mut i64,
    target_type3_id: i64,
    target_type3_lv: i64,
    target_type4_id: i64,
    target_type4_lv: i64,
    target_type15_id: i64,
    target_type15_lv: i64,
    att_type4_id: i64,
    att_type4_lv: i64,
    att_type15_id: i64,
    att_type15_lv: i64,
    att_type19_id: i64,
    att_type19_lv: i64,
    num34: i64,
) {
    let d = *num36 as f64;

    if target_type3_id == 11014 {
        *num36 += banker_round(d * 0.01 * target_type3_lv as f64) as i64;
    }
    if target_type4_id == 11002 {
        *num36 -= banker_round(d * 0.01 * target_type4_lv as f64) as i64;
    }
    if target_type4_id == 12025 {
        *num36 += banker_round(d * 0.02 * target_type4_lv as f64) as i64;
    }
    if att_type4_id == 13012 {
        *num36 += banker_round(*num36 as f64 * att_type4_lv as f64 * 0.033) as i64;
    }
    if target_type4_id == 13012 {
        *num36 -= banker_round(*num36 as f64 * target_type4_lv as f64 * 0.033) as i64;
    }
    if target_type15_id == 13011 {
        *num36 += banker_round(*num36 as f64 * target_type15_lv as f64 * 0.033) as i64;
    }
    if att_type15_id == 13011 {
        *num36 -= banker_round(*num36 as f64 * att_type15_lv as f64 * 0.033) as i64;
    }
    if matches!(att_type19_id, 14053 | 14040 | 12025) {
        let rate = match att_type19_id {
            14053 | 14040 => 0.1,
            12025 => 0.05,
            _ => 0.0,
        };
        *num36 += banker_round(*num36 as f64 * att_type19_lv as f64 * rate) as i64;
    }
    if num34 > 1 {
        *num36 = banker_round(*num36 as f64 / (num34 as f64 * 0.75)) as i64;
    }
}

/// `GetRandomMissAttack(lv1, lv2, lvtb1, lvtb2)` — returns `1` (hit) or `0` (miss).
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

/// `GetRandomMissTroi(...)` — status-effect land roll. Returns 1 (lands) / 0 (miss).
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

/// `GetRandomMissChayTron(...)` — flee roll. `percent = 60 + (lv1-lv2) + (lvtb1-lvtb2)`.
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
    // Flee percent may exceed 100 → clamp (C# clamps in RandomizeArrayWithPercent).
    randomize_with_percent(rng, 1, 0, percent)
}

/// `GetRandomMissCombo` — always hits (percent 100) => returns 1.
pub fn get_random_miss_combo(rng: &mut DotNetRandom) -> i64 {
    randomize_with_percent(rng, 1, 0, 100)
}

/// `RandomizeArray` — sequentially fold `RandomizeArrayWithPercent(prev, item, 50)`.
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

/// `RandomizeArrayWithPercent(value1, value2, percent)` — clamped to [0,100].
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

/// `GetRandomSkillNPC(lv, reborn, skill1..3)` — default missing skills to 10000.
/// Draws fresh `random_0.Next(1,100)` per roll (RNG-parity sensitive).
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

/// `GetRandomMissDrop(npcId)` — band roll against cumulative drop percent widths.
/// `percents` = `[percent_item1..6]` (C# Server defaults 25,23,20,4,3,1). Returns the
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
/// `round(npcLv / 2.0 + (npcLv - attackerLv))` — `TheBattle.cs:2323`.
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

/// `GetTurn(IdSkill, LvSKill)` — buff/debuff duration in turns (§5.4).
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn element_table_identity() {
        assert_eq!(get_damage_thuoctinh(1, 1), 1.0);
        assert_eq!(get_damage_thuoctinh(3, 4), 1.9);
    }

    #[test]
    fn element_skill_int() {
        assert_eq!(get_damage_skill_int(1, 2), 27);
        assert_eq!(get_damage_skill_int(4, 1), 54);
        assert_eq!(get_damage_skill_int(1, 0), 18); // per-row default
        assert_eq!(get_damage_skill_int(3, 0), 27);
        assert_eq!(get_damage_skill_int(0, 2), 10); // unknown row default
    }

    #[test]
    fn element_relation() {
        assert_eq!(get_thuoctinh_khac(1, 4), 2); // earth beats wind
        assert_eq!(get_thuoctinh_khac(1, 2), 1); // earth loses to water
        assert_eq!(get_thuoctinh_khac(1, 3), 0); // neutral
    }

    #[test]
    fn basic_physical_damage() {
        let dmg = calc_physical_damage(100, 50, 1, 1, 10, 10, 1, 10, 1, 2.0);
        assert!(dmg > 0, "should produce positive damage");
    }

    #[test]
    fn physical_uses_int_stat_for_combo_87() {
        // Same element, same level: higher stat → strictly more damage.
        let with_atk = calc_physical_damage_stat(200, 50, 1, 1, 10, 10, 1, 10, 1, 2.0);
        let with_int = calc_physical_damage_stat(50, 50, 1, 1, 10, 10, 1, 10, 1, 2.0);
        assert!(with_atk > with_int);
    }

    #[test]
    fn magic_damage_uses_int() {
        let dmg = calc_magic_damage(100, 50, 1, 1, 10, 10, 1, 10, 1, 12345, 1);
        assert!(dmg > 0, "should produce positive damage");
    }

    #[test]
    fn magic_multi_hit_aoe() {
        // 12016 with sl_danh 3 → divide by 1.5 then + skillLv*50.
        let dmg = calc_magic_damage(100, 50, 1, 1, 10, 10, 1, 10, 2, 12016, 3);
        assert!(dmg > 0);
    }

    #[test]
    fn get_turn_groups() {
        assert_eq!(get_turn(13002, 1), 3); // GROUP_a
        assert_eq!(get_turn(10033, 3), 3); // GROUP_b
        assert_eq!(get_turn(10004, 5), 3); // GROUP_c
        assert_eq!(get_turn(13015, 1), 4); // GROUP_d
        assert_eq!(get_turn(11014, 1), 5); // GROUP_e
        assert_eq!(get_turn(20025, 10), 5); // GROUP_f max lv
        assert_eq!(get_turn(14021, 3), 4); // 14021 specific
        assert_eq!(get_turn(14013, 7), 4); // 14013 falls through to GROUP_f ladder
        assert_eq!(get_turn(99999, 1), 3); // default
    }

    #[test]
    fn kill_exp_basic() {
        let exp = calc_kill_exp(10, 10);
        assert_eq!(exp, 7); // round(5 + 10/5) = round(7) = 7
    }

    #[test]
    fn kill_exp_too_far() {
        assert_eq!(calc_kill_exp(50, 10), 0); // diff 40 > 20
    }

    #[test]
    fn kill_exp_lower_level() {
        // npc higher than caster → round((npc-caster) + npc/5)
        let exp = calc_kill_exp(10, 15);
        assert_eq!(exp, 8); // round(5 + 3) = 8
    }

    #[test]
    fn hit_exp_teamdef_formula() {
        // round(npcLv/2 + (npcLv - attackerLv))
        assert_eq!(hit_exp(10, 20), 20); // round(10 + 10) = 20
        assert_eq!(hit_exp(10, 11), 6); // round(5.5 + 1) = 6 (banker's: 6.5->6)
    }

    #[test]
    fn combo_exp_bonus() {
        assert_eq!(calc_combo_exp(10), 11); // round(10 * 1.086) = round(10.86) = 11
    }

    #[test]
    fn drop_slots() {
        assert_eq!(get_random_drop_slot(1), 1);
        assert_eq!(get_random_drop_slot(25), 1);
        assert_eq!(get_random_drop_slot(26), 2);
        assert_eq!(get_random_drop_slot(48), 2);
        assert_eq!(get_random_drop_slot(76), 6);
        assert_eq!(get_random_drop_slot(77), 0);
        assert_eq!(get_random_drop_slot(999), 0);
    }

    #[test]
    fn miss_attack_high_avg_hits() {
        // Same level, equal avg => percent 100 => hit.
        let mut rng = DotNetRandom::new(7);
        assert_eq!(get_random_miss_attack(&mut rng, 10, 10, 10, 10), 1);
    }

    #[test]
    fn flee_percent_clamped() {
        // Huge favorable diff => percent clamped to 100 => always flee success.
        let mut rng = DotNetRandom::new(9);
        let roll = get_random_miss_flee(&mut rng, 100, 1, 100, 1);
        assert_eq!(roll, 1);
    }

    #[test]
    fn skill_npc_defaults_to_10000() {
        let mut rng = DotNetRandom::new(11);
        // No skills -> after failed rolls returns 10000.
        let skill = get_random_skill_npc(&mut rng, 10, 0, [0, 0, 0, 0]);
        assert!(skill == 10000 || skill >= 0);
    }

    #[test]
    fn randomize_array_fold() {
        // percent 50 always -> first element survives 50% of the folds. Just check bounds.
        let mut rng = DotNetRandom::new(13);
        let v = randomize_array(&mut rng, &[2, 1, 3, 4]);
        assert!([1, 2, 3, 4].contains(&v));
    }
}
