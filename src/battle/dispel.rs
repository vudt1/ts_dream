//! Battle Dispel, Cleanse, and Type 18 Cleanse+Heal Engine.
//!
//! Implements the three dispel/cleanse archetypes:
//! - **Archetype A (Specific Counter)**: 1:1 status cures and hostile barrier/mirror dispels.
//! - **Archetype B (Mass Cleanse)**: Friendly debuff purge (Type 3 + Type 15) preserving
//!   friendly protective buffs (Type 4) and job specialization auras (Type 19).
//! - **Archetype C (Cleanse + Heal, Type 18)**: Friendly debuff purge combined with INT-scaled
//!   HP/SP restoration and the caster-coordinate SP infinite loop gate.

use crate::battle::damage::banker_round;
use crate::battle::engine::WarInfo;
use crate::battle::packets::{self, attack_status, miss_status, sub_effect, troi_byte};

// ── Archetype A: Specific Counter Skills ───────────────────────────────────

/// Giải kết giới (10009) — Hostile dispel against Kết giới (`type4_id == 10010`).
pub const SKILL_GIAI_KET_GIOI: i64 = 10009;
/// Giải trừ kính (10014) — Hostile dispel against Kính (`type4_id == 10015`).
pub const SKILL_GIAI_TRU_KINH: i64 = 10014;
/// Thức tỉnh 1 (11004) — Friendly sleep cure (tier 1).
pub const SKILL_THUC_TINH_1: i64 = 11004;
/// Băng tan 1 (11009) — Friendly freeze thaw (tier 1).
pub const SKILL_BANG_TAN_1: i64 = 11009;
/// Băng tan (11015) — Friendly freeze thaw (all freeze variants).
pub const SKILL_BANG_TAN: i64 = 11015;
/// Giải hồn (14007) — Friendly sleep cure (`type3_id == 14008`).
pub const SKILL_GIAI_HON: i64 = 14007;
/// Giải độc (14014) — Friendly poison cure (`14015`, `20023`, `21008`).
pub const SKILL_GIAI_DOC: i64 = 14014;
/// Giải hỗn loạn (14022) — Friendly confusion cure (`14021` only; does not cure `20014`).
pub const SKILL_GIAI_HON_LOAN: i64 = 14022;

// ── Archetype B: Mass Cleanse Skills ───────────────────────────────────────

/// Trạng thái giải trừ (11012) — Single-target friendly debuff cleanse.
pub const SKILL_TRANG_THAI_GIAI_TRU: i64 = 11012;
/// Tiên Giai Trọng Giải (11025) — 3-target friendly debuff cleanse.
pub const SKILL_TIEN_GIAI_TRONG_GIAI: i64 = 11025;
/// Siêu Trọng Giải Thuật (11031) — 6-target friendly debuff cleanse.
pub const SKILL_SIEU_TRONG_GIAI: i64 = 11031;
/// Quang Dũ Thuật (22005) — Friendly debuff cleanse.
pub const SKILL_QUANG_DU_THUAT: i64 = 22005;

// ── Archetype C: Cleanse + Heal (Type 18) Skills ───────────────────────────

/// Thủy Thần 1 (11016) — Summon / Type 18 cleanse + heal.
pub const SKILL_THUY_THAN_1: i64 = 11016;
/// Thủy Thần 2 (11017) — Summon / Type 18 cleanse + heal.
pub const SKILL_THUY_THAN_2: i64 = 11017;
/// Thủy Thần 3 (11018) — Summon / Type 18 cleanse + heal.
pub const SKILL_THUY_THAN_3: i64 = 11018;
/// Thủy Thần 4 (11019) — Summon / Type 18 cleanse + heal.
pub const SKILL_THUY_THAN_4: i64 = 11019;
/// Từ Nhân Trạng Giải (20015) — Hiền Sĩ Type 18 cleanse + heal + SP restore.
pub const SKILL_TU_NHAN_TRANG_GIAI: i64 = 20015;
/// Giải Thoát (20019) — Tiên / Hiền Sĩ Type 18 cleanse + heal + SP restore.
pub const SKILL_GIAI_THOAT: i64 = 20019;
/// Cường Hiệu Giải Trừ (20050) — Advanced Type 18 cleanse + heal + SP restore.
pub const SKILL_CUONG_HIEU_GIAI_TRU: i64 = 20050;

/// Outcome of executing a dispel, mass cleanse, or Type 18 cleanse+heal on a single target.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DispelExecutionOutcome {
    pub hit: bool,
    pub type3_purged: bool,
    pub type15_purged: bool,
    pub type4_purged: bool,
    pub hp_healed: i64,
    pub sp_restored: i64,
    pub broadcast_packets: Vec<String>,
}

/// Returns `true` if `skill_id` belongs to Archetype A (specific 1:1 counter dispel/cure).
#[must_use]
pub fn is_specific_counter(skill_id: i64) -> bool {
    matches!(
        skill_id,
        SKILL_GIAI_KET_GIOI
            | SKILL_GIAI_TRU_KINH
            | SKILL_THUC_TINH_1
            | SKILL_BANG_TAN_1
            | SKILL_BANG_TAN
            | SKILL_GIAI_HON
            | SKILL_GIAI_DOC
            | SKILL_GIAI_HON_LOAN
    )
}

/// Returns `true` if `skill_id` belongs to Archetype C (Type 18 cleanse + HP/SP heal).
#[must_use]
pub fn is_cleanse_and_heal_skill(skill_id: i64) -> bool {
    matches!(
        skill_id,
        SKILL_THUY_THAN_1
            | SKILL_THUY_THAN_2
            | SKILL_THUY_THAN_3
            | SKILL_THUY_THAN_4
            | SKILL_TU_NHAN_TRANG_GIAI
            | SKILL_GIAI_THOAT
            | SKILL_CUONG_HIEU_GIAI_TRU
    )
}

/// Purges the Type 3 hard-disable slot on `target` and returns the corresponding wire packets.
pub fn purge_type3(target: &mut WarInfo) -> Vec<String> {
    target.type3_id = 0;
    target.type3_lv = 0;
    target.type3_turn = 0;
    vec![
        packets::purge_status(target.row, target.col, packets::PURGE_TYPE_DISABLE),
        packets::clear_unit_status(target.row, target.col, packets::STATUS_SEAL_RESIST_PCT),
    ]
}

/// Purges the Type 15 soft-debuff slot on `target` (restoring AGI if slowed) and returns the wire packets.
pub fn purge_type15(target: &mut WarInfo) -> Vec<String> {
    if matches!(
        target.type15_id,
        10016 | 10017 | 10018 | 10019 | 10025 | 20022
    ) {
        target.agi += 200;
    }
    target.type15_id = 0;
    target.type15_lv = 0;
    target.type15_turn = 0;
    vec![
        packets::purge_status(target.row, target.col, packets::PURGE_TYPE_DEBUFF),
        packets::clear_unit_status(target.row, target.col, packets::STATUS_DEF_MODIFIER),
        packets::clear_unit_status(target.row, target.col, packets::STATUS_ATK_MODIFIER),
    ]
}

/// Purges the Type 4 protective buff slot on `target` and returns the corresponding wire packets.
pub fn purge_type4(target: &mut WarInfo) -> Vec<String> {
    target.type4_id = 0;
    target.type4_lv = 0;
    target.type4_turn = 0;
    vec![
        packets::purge_status(target.row, target.col, packets::PURGE_TYPE_BUFF),
        packets::clear_unit_status(target.row, target.col, packets::STATUS_DEF_MODIFIER),
        packets::clear_unit_status(target.row, target.col, packets::STATUS_ATK_MODIFIER),
    ]
}

/// Archetype A: Executes a specific 1:1 counter cure or hostile barrier/mirror dispel.
pub fn apply_specific_dispel(
    caster: &WarInfo,
    target: &mut WarInfo,
    skill_id: i64,
) -> DispelExecutionOutcome {
    let mut outcome = DispelExecutionOutcome::default();

    match skill_id {
        // Hostile counters: must target an opposing team unit carrying the exact barrier/mirror.
        SKILL_GIAI_KET_GIOI => {
            if caster.team != target.team && target.type4_id == 10010 {
                outcome.hit = true;
                outcome.type4_purged = true;
                outcome.broadcast_packets.extend(purge_type4(target));
            }
        }
        SKILL_GIAI_TRU_KINH => {
            if caster.team != target.team && target.type4_id == 10015 {
                outcome.hit = true;
                outcome.type4_purged = true;
                outcome.broadcast_packets.extend(purge_type4(target));
            }
        }
        // Friendly Type 3 (hard disable) cures:
        SKILL_BANG_TAN => {
            if caster.team == target.team
                && matches!(target.type3_id, 11014 | 11027 | 11028 | 20026 | 6)
            {
                outcome.hit = true;
                outcome.type3_purged = true;
                outcome.broadcast_packets.extend(purge_type3(target));
            }
        }
        SKILL_BANG_TAN_1 => {
            if caster.team == target.team && matches!(target.type3_id, 11008 | 11014) {
                outcome.hit = true;
                outcome.type3_purged = true;
                outcome.broadcast_packets.extend(purge_type3(target));
            }
        }
        SKILL_GIAI_HON => {
            if caster.team == target.team && target.type3_id == 14008 {
                outcome.hit = true;
                outcome.type3_purged = true;
                outcome.broadcast_packets.extend(purge_type3(target));
            }
        }
        SKILL_THUC_TINH_1 => {
            if caster.team == target.team && matches!(target.type3_id, 11003 | 14008) {
                outcome.hit = true;
                outcome.type3_purged = true;
                outcome.broadcast_packets.extend(purge_type3(target));
            }
        }
        // Friendly Type 15 (soft debuff) cures:
        SKILL_GIAI_DOC => {
            if caster.team == target.team && matches!(target.type15_id, 14015 | 20023 | 21008) {
                outcome.hit = true;
                outcome.type15_purged = true;
                outcome.broadcast_packets.extend(purge_type15(target));
            }
        }
        SKILL_GIAI_HON_LOAN => {
            // Cures standard Confusion (14021) only; Cuồng Nộ (20014) requires Mass Cleanse / Type 18.
            if caster.team == target.team && target.type15_id == 14021 {
                outcome.hit = true;
                outcome.type15_purged = true;
                outcome.broadcast_packets.extend(purge_type15(target));
            }
        }
        _ => {}
    }

    outcome
}

/// Archetype B: Executes a friendly mass cleanse of Type 3 and Type 15 debuffs while
/// preserving friendly Type 4 protective buffs and Type 19 specialization auras.
pub fn apply_mass_cleanse(caster: &WarInfo, target: &mut WarInfo) -> DispelExecutionOutcome {
    let mut outcome = DispelExecutionOutcome::default();

    // Friendly-only check & must have at least one active debuff (Type 3 or Type 15).
    if caster.team != target.team || (target.type3_id == 0 && target.type15_id == 0) {
        return outcome;
    }

    outcome.hit = true;
    if target.type3_id > 0 {
        outcome.type3_purged = true;
        outcome.broadcast_packets.extend(purge_type3(target));
    }
    if target.type15_id > 0 {
        outcome.type15_purged = true;
        outcome.broadcast_packets.extend(purge_type15(target));
    }

    outcome
}

/// Archetype C: Executes a Type 18 friendly debuff cleanse combined with INT-scaled HP/SP
/// restoration and the caster-coordinate SP infinite loop gate.
pub fn apply_cleanse_and_heal(
    caster: &WarInfo,
    target: &mut WarInfo,
    skill_id: i64,
    skill_lv: i64,
) -> DispelExecutionOutcome {
    let mut outcome = DispelExecutionOutcome::default();

    // Friendly-only check.
    if caster.team != target.team {
        return outcome;
    }

    outcome.hit = true;

    if target.type3_id > 0 {
        outcome.type3_purged = true;
        outcome.broadcast_packets.extend(purge_type3(target));
    }
    if target.type15_id > 0 {
        outcome.type15_purged = true;
        outcome.broadcast_packets.extend(purge_type15(target));
    }

    let lv = skill_lv.max(1);
    let caster_int = caster.int1.max(0) as f64;

    let (raw_hp, raw_sp) = match skill_id {
        SKILL_CUONG_HIEU_GIAI_TRU => (
            banker_round(caster_int * 0.40) as i64 + lv * 40,
            banker_round(caster_int * 0.20) as i64 + lv * 20,
        ),
        SKILL_TU_NHAN_TRANG_GIAI | SKILL_GIAI_THOAT => (
            banker_round(caster_int * 0.35) as i64 + lv * 25,
            banker_round(caster_int * 0.15) as i64 + lv * 12,
        ),
        _ => (
            banker_round(caster_int * 0.30) as i64 + lv * 20,
            banker_round(caster_int * 0.10) as i64 + lv * 10,
        ),
    };

    // Caster SP infinite loop gate: checked strictly by grid coordinates (row, col).
    let is_self_cell = caster.row == target.row && caster.col == target.col;
    let effective_sp = if is_self_cell { 0 } else { raw_sp.max(0) };
    let effective_hp = raw_hp.max(0);

    let new_hp = (target.hp + effective_hp).min(target.hp_max);
    let actual_hp = (new_hp - target.hp).max(0);
    target.hp = new_hp;

    let new_sp = (target.sp + effective_sp).min(target.sp_max);
    let actual_sp = (new_sp - target.sp).max(0);
    target.sp = new_sp;

    outcome.hp_healed = actual_hp;
    outcome.sp_restored = actual_sp;

    outcome
}

/// Top-level dispatcher routing a dispel/cleanse skill to Archetype A, B, or C.
pub fn execute_dispel(
    caster: &WarInfo,
    target: &mut WarInfo,
    skill_id: i64,
    skill_lv: i64,
    skill_type: i64,
) -> DispelExecutionOutcome {
    if is_cleanse_and_heal_skill(skill_id) || skill_type == 18 {
        apply_cleanse_and_heal(caster, target, skill_id, skill_lv)
    } else if is_specific_counter(skill_id) {
        apply_specific_dispel(caster, target, skill_id)
    } else {
        apply_mass_cleanse(caster, target)
    }
}

/// Encodes the per-target sub-record inside the `35 01` (`skilling`) packet for a dispel/cleanse outcome.
#[must_use]
pub fn build_action_fragment(
    skill_id: i64,
    skill_type: i64,
    outcome: &DispelExecutionOutcome,
    row: u8,
    col: u8,
) -> String {
    if !outcome.hit {
        return packets::skilling_int(
            row,
            col,
            miss_status::MISS,
            attack_status::ATTACK,
            1,
            troi_byte::MISS,
            0,
            1,
        );
    }

    if is_cleanse_and_heal_skill(skill_id) || skill_type == 18 {
        let count = 2
            + u8::from(outcome.type3_purged)
            + u8::from(outcome.type15_purged);
        let hp_u16 = outcome.hp_healed.clamp(0, i64::from(u16::MAX)) as u16;
        let sp_u16 = outcome.sp_restored.clamp(0, i64::from(u16::MAX)) as u16;

        let mut fragment = packets::skilling_int(
            row,
            col,
            miss_status::ATTACK,
            attack_status::ATTACK,
            count,
            troi_byte::HP,
            hp_u16,
            0,
        );
        fragment.push_str(&packets::sub_action_sp(sp_u16));
        if outcome.type3_purged {
            fragment.push_str(sub_effect::PURGE_TYPE3);
        }
        if outcome.type15_purged {
            fragment.push_str(sub_effect::PURGE_TYPE15);
        }
        return fragment;
    }

    if is_specific_counter(skill_id) {
        let troi = if outcome.type3_purged {
            troi_byte::TYPE3
        } else if outcome.type15_purged {
            troi_byte::TYPE15
        } else if outcome.type4_purged {
            troi_byte::TYPE4
        } else {
            troi_byte::MISS
        };
        return packets::skilling_int(
            row,
            col,
            miss_status::ATTACK,
            attack_status::ATTACK,
            1,
            troi,
            0,
            1,
        );
    }

    // Archetype B (Mass Cleanse): base sub-effect (MISS/0/1) + PURGE_TYPE3 and/or PURGE_TYPE15.
    let count = 1 + u8::from(outcome.type3_purged) + u8::from(outcome.type15_purged);
    let mut fragment = packets::skilling_int(
        row,
        col,
        miss_status::ATTACK,
        attack_status::ATTACK,
        count,
        troi_byte::MISS,
        0,
        1,
    );
    if outcome.type3_purged {
        fragment.push_str(sub_effect::PURGE_TYPE3);
    }
    if outcome.type15_purged {
        fragment.push_str(sub_effect::PURGE_TYPE15);
    }
    fragment
}
