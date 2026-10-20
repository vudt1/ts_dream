//! Battle engine core (Chapter 6) — pure math + domain that is testable
//! without a live DB or wire capture. The async per-battle task (turn loop,
//! tick, targeting) builds on these primitives.

use crate::data::texps::texp_get_lv_up;
use crate::protocol::encoder;

/// EXP level-ups from a battle reward, via the Texps array (§6.6).
pub fn exp_lv_up(texps: &[crate::data::tables::TexpRow], lv: i64, reborn: usize, texp: i64) -> i64 {
    texp_get_lv_up(texps, lv, reborn, texp)
}

/// 20-cell grid keyed by `row-col` (hex). Row 0..3, col 0..4.
pub fn war_key(row: u8, col: u8) -> String {
    format!("{row:x}{col:x}")
}

/// Team for a grid row: the client derives the side as `grp < 2` ⇒ side 2 and
/// `grp >= 2` ⇒ side 1 (`aLogin.exe`, `HANDOFF-client-reverse.md §0.2`), so rows
/// 0 and 1 are the enemy side 2 while rows 2 and 3 are the player side 1.
pub fn team_for_row(row: u8) -> i64 {
    if row < 2 {
        2
    } else {
        1
    }
}

/// A single grid cell (Chapter 6 §6.0).
#[derive(Debug, Clone, Default)]
pub struct WarInfo {
    pub typ: u8,
    pub id: i64,
    pub id_npc_on_map: i64,
    pub id_char: i64,
    pub row: u8,
    pub col: u8,
    pub hp_max: i64,
    pub sp_max: i64,
    pub hp: i64,
    pub sp: i64,
    pub lv: i64,
    pub thuoctinh: i64,
    pub leader_id: i64,
    pub id_skill: i64,
    pub lv_skill: i64,
    pub row_attack: u8,
    pub col_attack: u8,
    pub int1: i64,
    pub atk: i64,
    pub def: i64,
    pub agi: i64,
    pub reborn: i64,
    pub team: i64,
    /// Mobile fly-out marker; intentionally excluded from the PC 23-byte cell packet.
    pub fly_out: bool,
    pub attacked: bool,
    pub random: i64,
    pub exp: i64,
    // (Leader quan-su context lives on the `Battle`, not the cell — see
    // `Battle.leader_id_qs`/`leader_qs_int`.)

    // Buff/debuff triplets (id, level, remaining turns)
    pub type3_id: i64,
    pub type3_lv: i64,
    pub type3_turn: i64,
    pub type4_id: i64,
    pub type4_lv: i64,
    pub type4_turn: i64,
    pub type15_id: i64,
    pub type15_lv: i64,
    pub type15_turn: i64,
    pub type19_id: i64,
    pub type19_lv: i64,
    pub type19_turn: i64,
}

impl WarInfo {
    /// The `_Packet` 23-byte snapshot (`ChangedWar`, §6.1):
    /// Type:X2 | le32(Id) | le16(IdNpcOnMap) | le32(IdChar) | row,col |
    /// le16(HpMax) | le16(SpMax) | le16(Hp) | le16(Sp) | lv, TT.
    ///
    /// This is byte-for-byte the 23-byte record of `0B 05` (`FUN_00644294`) and
    /// the 24-byte record of `0B FA` (`FUN_006438ac`) with the leading `kind`
    /// byte omitted (`01_battle_start_scene.md §2.2` and `§3.2`; verified from
    /// the client ASM `client_pseudo_c/006438ac_FUN_006438ac.asm.txt:74-240`).
    pub fn packet_hex(&self) -> String {
        let mut s = String::with_capacity(46);
        s.push_str(&format!("{:02X}", self.typ));
        s.push_str(&encoder::le32(self.id as u32));
        s.push_str(&encoder::le16(self.id_npc_on_map as u16));
        s.push_str(&encoder::le32(self.id_char as u32));
        s.push_str(&format!("{:02X}{:02X}", self.row, self.col));
        s.push_str(&encoder::le16(clamp16(self.hp_max)));
        s.push_str(&encoder::le16(clamp16(self.sp_max)));
        s.push_str(&encoder::le16(clamp16(self.hp)));
        s.push_str(&encoder::le16(clamp16(self.sp)));
        s.push_str(&format!(
            "{:02X}{:02X}",
            clamp_u8(self.lv),
            clamp_u8(self.thuoctinh)
        ));
        s
    }
}

fn clamp16(v: i64) -> u16 {
    v.clamp(0, 0xFFFF) as u16
}

fn clamp_u8(v: i64) -> u8 {
    v.clamp(0, 0xFF) as u8
}

/// Max-HP formula `(rb, job, lvl, hpx)` — byte-matched to the client
/// `FUN_0065a554` (`test/ts_decompile/functions/0065a554_...asm.txt`).
///
/// The client dispatches on `reborn` (outer, arg `[EBP+0xc]`) and, for reborn
/// 2 only, on `job` (inner, arg `[EBP+0x8]`, values 1..4). Every branch is
/// `ROUND(pow(level, w37) * hpx * mult + level * w35) + base + hpx * add`:
///
/// | reborn | job | `mult` | `base` | `add` | ASM |
/// |---:|---:|---:|---:|---:|---|
/// | 0 | — | 2.0 | 80 | 2 | `0x65a583` |
/// | 1 | — | 2.0 | 180 | 4 | `0x65a66d` |
/// | 2 | 1 | 2.5 | 280 | 25 | `0x65a778` |
/// | 2 | 2 | 3.0 | 380 | 30 | `0x65a841` |
/// | 2 | 3 | 2.0 | 180 | 23 | `0x65a90e` |
/// | 2 | 4 | 2.0 | 180 | 21 | `0x65a9db` |
///
/// `w37` (exponent), `w35` (level coefficient) and reborn-0's `base`
/// (`base_hp`) are read from `Data/Formula.Dat` (`w37 = 0.35`, `w35 = 1.0`,
/// `base_hp = 80`); the other multiplicands/addends are inline ASM immediates.
/// `ROUND` is half away from zero (`@ROUND` @ `0x00402b1c`).
///
/// Reborn 3 (`:18`) and a reborn-2 `job` outside 1..4 (`:184`) fall through to
/// the epilogue, where the client returns 0. The server keeps the safe job-4
/// column (`base 180 + hpx*21`) instead of 0 so an unexpected value can never
/// zero a live stat. See `tests/max_hp_sp_formula_test.rs` for the sample set.
pub fn get_hp_max(rb: i64, job: i64, lvl: i64, hpx: i64) -> i64 {
    let lv = lvl as f64;
    let p = lv.powf(0.35);
    let (mult, base, add): (f64, i64, i64) = match rb {
        0 => (2.0, 80, 2),
        1 => (2.0, 180, 4),
        2 => match job {
            1 => (2.5, 280, 25),
            2 => (3.0, 380, 30),
            3 => (2.0, 180, 23),
            _ => (2.0, 180, 21),
        },
        _ => (2.0, 180, 21),
    };
    (p * hpx as f64 * mult + lv).round() as i64 + base + hpx * add
}

/// Max-SP formula `(rb, job, lvl, spx)` — byte-matched to the client
/// `FUN_0065ad10` (`test/ts_decompile/functions/0065ad10_...asm.txt`), same
/// dispatch shape as [`get_hp_max`]:
///
/// | reborn | job | `mult` | `base` | ASM |
/// |---:|---:|---:|---:|---|
/// | 0 | — | 2.0 | 60 | `0x65ad3f` |
/// | 1 | — | 2.0 | 110 | `0x65ae06` |
/// | 2 | 1, 2 | 2.0 | 160 | `0x65aee3` |
/// | 2 | 3 | 3.0 | 310 | `0x65af8a` |
/// | 2 | 4 | 3.5 | 410 | `0x65b031` |
///
/// Exponent `w40 = 0.25`, level coefficient `w38 = 1.0`; reborn-0's `base` is
/// `base_sp = 60`. Both come from `Data/Formula.Dat`. Reborn 3 and an
/// unexpected reborn-2 job fall to the epilogue (client returns 0); the server
/// keeps the safe job-4 column (`base 410`).
pub fn get_sp_max(rb: i64, job: i64, lvl: i64, spx: i64) -> i64 {
    let lv = lvl as f64;
    let p = lv.powf(0.25);
    let (mult, base): (f64, i64) = match rb {
        0 => (2.0, 60),
        1 => (2.0, 110),
        2 => match job {
            1 | 2 => (2.0, 160),
            3 => (3.0, 310),
            _ => (3.5, 410),
        },
        _ => (3.5, 410),
    };
    (p * spx as f64 * mult + lv).round() as i64 + base
}
