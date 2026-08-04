//! Damage pipeline (Chapter 6 §6.5) — element tables, physical/magic formulas,
//! and miss-roll calculations.

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
pub fn get_damage_skill_int(skill_tt: i64, def_tt: i64) -> i64 {
    match (skill_tt, def_tt) {
        (1, 1) => 18,
        (1, 2) => 27,
        (1, 3) => 21,
        (1, 4) => 13,
        (2, 1) => 10,
        (2, 2) => 19,
        (2, 3) => 29,
        (2, 4) => 19,
        (3, 1) => 26,
        (3, 2) => 15,
        (3, 3) => 27,
        (3, 4) => 34,
        (4, 1) => 54,
        (4, 2) => 42,
        (4, 3) => 29,
        (4, 4) => 42,
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

/// Physical damage (Type 1) base calculation.
///
/// `num36 = round(Atk * Element * 2.0 - Def * 1.6)`
/// `+= round((attLv - defLv) / 1.5) + round(attLv / 20.0) * 8`
/// `+= round(GetDamageSkillInt(skillTT, defTT) * DoManh * (1.0 + skillLv * 0.033))`
/// `*= num37`
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
    let element = get_damage_thuoctinh(att_tt, def_tt);
    let mut num36 = banker_round(atk as f64 * element * 2.0 - def as f64 * 1.6);
    num36 +=
        banker_round((att_lv - def_lv) as f64 / 1.5) + banker_round(att_lv as f64 / 20.0) * 8.0;
    num36 = banker_round(
        num36
            + get_damage_skill_int(skill_tt, def_tt) as f64
                * do_manh as f64
                * (1.0 + skill_lv as f64 * 0.033),
    );
    num36 = banker_round(num36 * num37);
    num36 as i64
}

/// Magic damage (Type 2) base calculation.
/// Uses `_Int` instead of `_Atk`, no `num37` combo multiplier.
pub fn calc_magic_damage(
    int1: i64,
    def: i64,
    att_tt: i64,
    def_tt: i64,
    att_lv: i64,
    def_lv: i64,
    skill_tt: i64,
    do_manh: i64,
    skill_lv: i64,
) -> i64 {
    let element = get_damage_thuoctinh(att_tt, def_tt);
    let mut num36 = banker_round(int1 as f64 * element * 2.0 - def as f64 * 1.6);
    num36 +=
        banker_round((att_lv - def_lv) as f64 / 1.5) + banker_round(att_lv as f64 / 20.0) * 8.0;
    num36 = banker_round(
        num36
            + get_damage_skill_int(skill_tt, def_tt) as f64
                * do_manh as f64
                * (1.0 + skill_lv as f64 * 0.033),
    );
    num36 as i64
}

/// Apply ordered buff modifiers to the damage value (§5.2).
pub fn apply_buff_modifiers(
    num36: &mut i64,
    // target buffs
    target_type3_id: i64,
    target_type3_lv: i64,
    target_type4_id: i64,
    target_type4_lv: i64,
    target_type15_id: i64,
    target_type15_lv: i64,
    // attacker buffs
    att_type4_id: i64,
    att_type4_lv: i64,
    att_type15_id: i64,
    att_type15_lv: i64,
    att_type19_id: i64,
    att_type19_lv: i64,
    // skill data
    sl_danh: i64,
    num34: i64,
) {
    let d = *num36 as f64;

    // target _Type3_Id==11014 → +round(num36*0.01*Type3_Lv)
    if target_type3_id == 11014 {
        *num36 += banker_round(d * 0.01 * target_type3_lv as f64) as i64;
    }
    // target _Type4_Id==11002 → -round(num36*0.01*Type4_Lv)
    if target_type4_id == 11002 {
        *num36 -= banker_round(d * 0.01 * target_type4_lv as f64) as i64;
    }
    // target _Type4_Id==12025 → +round(num36*0.02*Type4_Lv)
    if target_type4_id == 12025 {
        *num36 += banker_round(d * 0.02 * target_type4_lv as f64) as i64;
    }
    // attacker _Type4_Id==13012 → +round(num36*Type4_Lv*0.033)
    if att_type4_id == 13012 {
        *num36 += banker_round(*num36 as f64 * att_type4_lv as f64 * 0.033) as i64;
    }
    // target _Type4_Id==13012 → -round(num36*Type4_Lv*0.033)
    if target_type4_id == 13012 {
        *num36 -= banker_round(*num36 as f64 * target_type4_lv as f64 * 0.033) as i64;
    }
    // target _Type15_Id==13011 → +round(num36*Type15_Lv*0.033)
    if target_type15_id == 13011 {
        *num36 += banker_round(*num36 as f64 * target_type15_lv as f64 * 0.033) as i64;
    }
    // attacker _Type15_Id==13011 → -round(num36*Type15_Lv*0.033)
    if att_type15_id == 13011 {
        *num36 -= banker_round(*num36 as f64 * att_type15_lv as f64 * 0.033) as i64;
    }
    // attacker _Type19_Id ∈ {14053,14040,12025}
    if matches!(att_type19_id, 14053 | 14040 | 12025) {
        let rate = match att_type19_id {
            14053 | 14040 => {
                if sl_danh > 0 {
                    0.1 / sl_danh as f64
                } else {
                    0.1
                }
            }
            12025 => {
                if sl_danh > 0 {
                    0.05 / sl_danh as f64
                } else {
                    0.05
                }
            }
            _ => 0.0,
        };
        *num36 += banker_round(*num36 as f64 * att_type19_lv as f64 * rate) as i64;
    }
    // AoE falloff: if num34 > 1 → num36 = round(num36 / (num34 * 0.75))
    if num34 > 1 {
        *num36 = banker_round(*num36 as f64 / (num34 as f64 * 0.75)) as i64;
    }
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
            3 // was `num` initial
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
    // Inner switch: 14021 and 14013
    match id_skill {
        14021 => match lv_skill {
            1 => 2,
            2 => 3,
            3 => 4,
            4 => 5,
            5 => 6,
            _ => 3,
        },
        14013 => {
            // Falls through to GROUP_f's ladder
            match lv_skill {
                1..=3 => 2,
                4..=6 => 3,
                7..=9 => 4,
                10 => 5,
                _ => 3,
            }
        }
        _ => 3, // default
    }
}

/// Exp calculation from level difference (§3.6 / §7.2).
/// Returns base exp for a kill.
pub fn calc_kill_exp(caster_lv: i64, npc_lv: i64) -> i64 {
    let diff = (caster_lv - npc_lv).abs();
    if diff > 20 {
        return 0;
    }
    if caster_lv >= npc_lv {
        match diff {
            0..=2 => banker_round(5.0 + caster_lv as f64 / 5.0) as i64,
            3..=5 => banker_round(4.0 + caster_lv as f64 / 5.0) as i64,
            6..=10 => banker_round(3.0 + caster_lv as f64 / 5.0) as i64,
            11..=15 => banker_round(2.0 + caster_lv as f64 / 5.0) as i64,
            16..=20 => banker_round(1.0 + caster_lv as f64 / 5.0) as i64,
            _ => 0,
        }
    } else {
        // npc is higher level
        banker_round((npc_lv - caster_lv) as f64 + npc_lv as f64 / 5.0) as i64
    }
}

/// Combo exp bonus: `round(base * 1.086)`.
pub fn calc_combo_exp(base_exp: i64) -> i64 {
    banker_round(base_exp as f64 * 1.086) as i64
}

/// Drop roll (§5.7): returns item slot index (1-6) or 0 (no drop).
/// Band widths: 25, 23, 20, 4, 3, 1.
pub fn get_random_drop_slot(roll: i32) -> usize {
    // roll is in [1, 999]
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
        assert_eq!(get_damage_skill_int(0, 0), 10); // default
    }

    #[test]
    fn element_relation() {
        assert_eq!(get_thuoctinh_khac(1, 4), 2); // earth beats wind
        assert_eq!(get_thuoctinh_khac(1, 2), 1); // earth loses to water
        assert_eq!(get_thuoctinh_khac(1, 3), 0); // neutral
    }

    #[test]
    fn basic_physical_damage() {
        // atk=100, def=50, same element, lv diff=0, basic skill
        let dmg = calc_physical_damage(100, 50, 1, 1, 10, 10, 1, 10, 1, 2.0);
        assert!(dmg > 0, "should produce positive damage");
    }

    #[test]
    fn magic_damage_uses_int() {
        let dmg = calc_magic_damage(100, 50, 1, 1, 10, 10, 1, 10, 1);
        assert!(dmg > 0, "should produce positive damage");
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
}
