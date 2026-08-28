//! PC `Skill.Dat` loader (ADR 0002 follow-up #1).
//!
//! Reverse-engineered from the supplied PC source `SkillData.cs` and
//! `SkillInfo.cs` (Pack=1, 86 bytes per record, after an 86-byte zero header).
//!
//! **Record layout** (Pack=1, little-endian):
//!
//! | Offset | Size | Field       | Decoder                    |
//! |--------|------|-------------|----------------------------|
//! | 0      | 1    | `namelength`| none (raw)                 |
//! | 1      | 20   | `name[20]`  | reverse bytes (i ↔ 19-i)   |
//! | 21     | 1    | `type`      | `(val ^ 0xFD) - 4` u8      |
//! | 22     | 2    | `id`        | `(val ^ 0x6E90) - 4` LE u16|
//! | 24     | 2    | `sp_cost`   | same as id                 |
//! | 26     | 1    | `elem`      | `DecodeItem8`              |
//! | 27     | 4    | `unk3`      | `(val ^ 0xBAEB716) - 2`    |
//! | 31     | 1    | `unk4`      | `DecodeItem8`              |
//! | 32     | 1    | `grade`     | `DecodeItem8`              |
//! | 33     | 1    | `skill_type`| `DecodeItem8`              |
//! | 34     | 1    | `nb_target` | `DecodeItem8`              |
//! | 35     | 1    | `unk8`      | `DecodeItem8`              |
//! | 36     | 1    | `delay`     | `DecodeItem8`              |
//! | 37     | 1    | `state`     | `DecodeItem8`              |
//! | 38     | 1    | `unk11`     | `DecodeItem8`              |
//! | 39     | 1    | `unk12`     | `DecodeItem8`              |
//! | 40     | 1    | `sk_point`  | `DecodeItem8`              |
//! | 41     | 1    | `unk14`     | `DecodeItem8`              |
//! | 42     | 1    | `max_lvl`   | `DecodeItem8`              |
//! | 43     | 2    | `require_sk`| same as id                 |
//! | 45     | 2    | `unk17`     | same as id                 |
//! | 47     | 1    | `unk18`     | `DecodeItem8`              |
//! | 48     | 2    | `unk19`     | same as id                 |
//! | 50     | 1    | `unk20`     | `DecodeItem8`              |
//! | 51     | 2    | `unk21`     | same as id                 |
//! | 53     | 2    | `unk22`     | same as id                 |
//! | 55     | 1    | `des_length`| none (raw)                 |
//! | 56     | 30   | `des[30]`   | reverse bytes (j ↔ 29-j)   |
//!
//! **Identity**: the skill id is encoded in the record itself (offset 22, with
//! `DecodeItem16`), not the record index. Skill 10000 (basic attack) lives in
//! the file at the position where the encoded id equals 10000 after decoding.
//!
//! **Bridge to `BinarySkillDef`**: the PC loader maps its raw fields into the
//! shared `BinarySkillDef` struct consumed by `battle::runner::BattleData`.
//! Unknown fields (e.g. PC-specific `grade`, `nb_target`, `unk11..unk22`)
//! default to 0; `description` is dropped (it is GUI-only, never packet).

use crate::data::tables::BinarySkillDef;
use crate::encoding;
use crate::error::{Result, TsError};
use std::collections::HashMap;

/// PC `Skill.Dat` raw record (matches the supplied `SkillInfo.cs`).
#[derive(Debug, Clone, Default)]
pub struct PcSkillDef {
    pub namelength: u8,
    /// Reversed-decoded VISCII 1-byte name (20 bytes, first `namelength` are
    /// the visible characters; remaining bytes are junk).
    pub name: String,
    pub type_: u8,
    pub id: u16,
    pub sp_cost: u16,
    pub elem: u8,
    /// Unknown u32 (semantics TBD).
    pub unk3: u32,
    pub unk4: u8,
    pub grade: u8,
    pub skill_type: u8,
    pub nb_target: u8,
    pub unk8: u8,
    pub delay: u8,
    pub state: u8,
    pub unk11: u8,
    pub unk12: u8,
    pub sk_point: u8,
    pub unk14: u8,
    pub max_lvl: u8,
    pub require_sk: u16,
    pub unk17: u16,
    pub unk18: u8,
    pub unk19: u16,
    pub unk20: u8,
    pub unk21: u16,
    pub unk22: u16,
    pub des_length: u8,
    /// Reversed-decoded VISCII 1-byte description (30 bytes).
    pub des: String,
}

/// Loader for PC `Skill.Dat` (Pack=1, 86 bytes per record, 86-byte zero header).
pub struct SkillDatLoaderPc;

impl SkillDatLoaderPc {
    /// Parse the full `Skill.Dat` buffer into a map keyed by skill id.
    pub fn load(bytes: &[u8]) -> Result<HashMap<u16, BinarySkillDef>> {
        let (bins, _) = Self::load_with_pc(bytes)?;
        Ok(bins)
    }

    /// Parse the full `Skill.Dat` buffer and return both:
    /// 1. A `BinarySkillDef` map (consumed by the battle engine), and
    /// 2. A `PcSkillDef` map (raw PC fields, useful for diagnostics and
    ///    fields that don't bridge to the shared struct).
    pub fn load_with_pc(
        bytes: &[u8],
    ) -> Result<(HashMap<u16, BinarySkillDef>, HashMap<u16, PcSkillDef>)> {
        const HEADER: usize = 86;
        const RECORD_SIZE: usize = 86;

        if bytes.len() < HEADER {
            return Err(TsError::Data(format!(
                "PC Skill.Dat buffer too small: {} bytes, expected at least {}",
                bytes.len(),
                HEADER
            )));
        }

        let payload = &bytes[HEADER..];
        let total_records = payload.len() / RECORD_SIZE;
        let mut bins = HashMap::with_capacity(total_records.min(100_000));
        let mut pcs = HashMap::with_capacity(total_records.min(100_000));

        for idx in 0..total_records {
            let start = idx * RECORD_SIZE;
            let rec = &payload[start..start + RECORD_SIZE];
            let pc = parse_record(rec)?;
            if pc.id == 0 {
                continue;
            }
            let bin = pc_to_binary(&pc);
            bins.insert(bin.id, bin);
            pcs.insert(pc.id, pc);
        }

        Ok((bins, pcs))
    }
}

// ── Decoders (mirror the C# `DecodeItem8/16/32` helpers) ──────────────

#[inline]
fn decode_item8(val: u8) -> u8 {
    val.wrapping_sub(4) ^ 0xFD
}

#[inline]
fn decode_item16(val: u16) -> u16 {
    val.wrapping_sub(4) ^ 0x6E90
}

#[inline]
fn decode_item32(val: u32) -> u32 {
    val.wrapping_sub(2) ^ 0x0BAEB716
}

// ── Reverse-in-place helpers (mirror the C# name/des byte-swap loops) ──

fn reverse_name(name: &mut [u8; 20]) {
    for i in 0..10 {
        let b = name[19 - i];
        name[19 - i] = name[i];
        name[i] = b;
    }
}

fn reverse_des(des: &mut [u8; 30]) {
    for j in 0..15 {
        let b = des[29 - j];
        des[29 - j] = des[j];
        des[j] = b;
    }
}

/// Decode a VISCII 1.1 byte slice up to `length` bytes. The PC binary uses
/// VISCII 1-byte characters; we route through `encoding::viscii_to_unicode`
/// for the full 102-entry Vietnamese table (with the `Đ` extension at 0xD0
/// and 0xDD) and fall back to Latin-1 pass-through for bytes outside the
/// table — the same fallback the canonical decoder uses.
fn decode_viscii(bytes: &[u8], length: usize) -> String {
    let take = length.min(bytes.len());
    let mut out = String::with_capacity(take);
    for &b in &bytes[..take] {
        if b == 0 {
            continue;
        }
        out.push(encoding::viscii_to_unicode(b));
    }
    out
}

fn parse_record(rec: &[u8]) -> Result<PcSkillDef> {
    if rec.len() != 86 {
        return Err(TsError::Data(format!(
            "PC Skill.Dat record has length {}, expected 86",
            rec.len()
        )));
    }

    let namelength = rec[0];
    let mut name_buf: [u8; 20] = rec[1..21].try_into().unwrap();
    reverse_name(&mut name_buf);
    let name = decode_viscii(&name_buf, namelength as usize);

    let type_ = decode_item8(rec[21]);
    let id = decode_item16(u16::from_le_bytes([rec[22], rec[23]]));
    let sp_cost = decode_item16(u16::from_le_bytes([rec[24], rec[25]]));
    let elem = decode_item8(rec[26]);
    let unk3 = decode_item32(u32::from_le_bytes([rec[27], rec[28], rec[29], rec[30]]));
    let unk4 = decode_item8(rec[31]);
    let grade = decode_item8(rec[32]);
    let skill_type = decode_item8(rec[33]);
    let nb_target = decode_item8(rec[34]);
    let unk8 = decode_item8(rec[35]);
    let delay = decode_item8(rec[36]);
    let state = decode_item8(rec[37]);
    let unk11 = decode_item8(rec[38]);
    let unk12 = decode_item8(rec[39]);
    let sk_point = decode_item8(rec[40]);
    let unk14 = decode_item8(rec[41]);
    let max_lvl = decode_item8(rec[42]);
    let require_sk = decode_item16(u16::from_le_bytes([rec[43], rec[44]]));
    let unk17 = decode_item16(u16::from_le_bytes([rec[45], rec[46]]));
    let unk18 = decode_item8(rec[47]);
    let unk19 = decode_item16(u16::from_le_bytes([rec[48], rec[49]]));
    let unk20 = decode_item8(rec[50]);
    let unk21 = decode_item16(u16::from_le_bytes([rec[51], rec[52]]));
    let unk22 = decode_item16(u16::from_le_bytes([rec[53], rec[54]]));

    let des_length = rec[55];
    let mut des_buf: [u8; 30] = rec[56..86].try_into().unwrap();
    reverse_des(&mut des_buf);
    let des = decode_viscii(&des_buf, des_length as usize);

    Ok(PcSkillDef {
        namelength,
        name,
        type_,
        id,
        sp_cost,
        elem,
        unk3,
        unk4,
        grade,
        skill_type,
        nb_target,
        unk8,
        delay,
        state,
        unk11,
        unk12,
        sk_point,
        unk14,
        max_lvl,
        require_sk,
        unk17,
        unk18,
        unk19,
        unk20,
        unk21,
        unk22,
        des_length,
        des,
    })
}

/// Bridge: convert a parsed `PcSkillDef` into the shared `BinarySkillDef` the
/// battle engine consumes.
///
/// Field mappings (PC → BinarySkillDef):
/// - `id` → `id`
/// - `type_` (1 byte) → `kind`
/// - `sp_cost` → `require_sp`
/// - `elem` → `element`
/// - `unk3` (u32) → `numerical` (raw; semantics differ from mobile `numerical`)
/// - `skill_type` → `attribute` (best-effort — PC semantics for this byte
///   aren't fully verified, but it lines up with mobile `attribute` byte)
/// - `round` field is missing in PC layout → derived from `state` byte
/// - `delay` → `spend_second`
/// - `state` → `round` (best-effort; the C# `state` field name suggests
///   "skill state/effect" rather than duration — mapping is provisional)
/// - `max_lvl` → `max_lv`
/// - `require_sk` → pre_skill_id1
/// - `unk21` → pre_skill_id2 (best-effort; PC has many unknown u16 fields)
/// - `name` → `name`
pub fn pc_to_binary(pc: &PcSkillDef) -> BinarySkillDef {
    let mut pre_skills = Vec::with_capacity(6);
    if pc.require_sk != 0 {
        pre_skills.push(pc.require_sk);
    }
    if pc.unk21 != 0 {
        pre_skills.push(pc.unk21);
    }

    BinarySkillDef {
        id: pc.id,
        name: pc.name.clone(),
        kind: pc.type_,
        require_sp: pc.sp_cost,
        element: pc.elem,
        numerical: pc.unk3,
        attribute: pc.skill_type,
        level: 0,
        fight_way: pc.nb_target,
        fight_area: pc.unk4,
        round: pc.state,
        spend_second: pc.delay,
        hit_status: pc.unk11,
        how_much_times: pc.grade,
        limit_lv: pc.unk12,
        learn_point: pc.sk_point,
        level_up_point: pc.unk14,
        max_lv: pc.max_lvl,
        pre_skills,
        atk_kind: pc.unk17,
        turn_kind: (pc.unk19 & 0xFF) as u8,
        learn_limit: pc.unk18,
        use_limit: pc.unk20 as u16,
        fight_way_grow_type: pc.unk22,
        description: String::new(),
    }
}
