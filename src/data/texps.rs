//! Texps — cumulative EXP thresholds, byte-matched to the client (`aLogin.exe`).
//!
//! The client computes the table with two functions:
//!
//! * `FUN_0065b0dc(table, level, reborn)` — **one level's contribution**:
//!   `ROUND(level ^ k[reborn]) + *(i32*)(table + 0xAC)`.
//! * `FUN_0065b398(table, level, reborn)` — **the threshold of `level`**, i.e.
//!   the running sum `Σ_{i=1..level} FUN_0065b0dc(table, i, reborn)`
//!   (`0065b398_FUN_0065b398.asm.txt:12-30`). This is what a `TexpRow` holds.
//!
//! `k[reborn]` (`0065b0dc_FUN_0065b0dc.asm.txt`):
//!
//! | reborn | branch | source | value |
//! |---:|---|---|---|
//! | 0 | `0x65B101` | `FLD double [table+0x84]` (`:29`, RAM slot) | **2.9** |
//! | 1 | `0x65B16D` | `PUSH 0x4000 / 0xC0000000 / 0x0` (`:64-66`) | **3.0** |
//! | 2 | `0x65B1D5` | `PUSH 0x4000 / 0xC3333333 / 0x33333333` (`:98-100`) | **3.05** |
//! | 3 | `0x65B23D` | `PUSH 0x4000 / 0xC0000000 / 0x0` (`:132-134`) | **3.0** |
//!
//! Reborn 0 has no embedded constant because it is a *data* slot: the three
//! `PUSH`es build the 10-byte 80-bit x87 operand that `FSTP extended` writes,
//! and with the x87 little-endian layout (bytes 0..7 = significand, bytes 8..9 =
//! biased exponent) the three pushes `exp / mant_hi / mant_lo` decode as
//! `value = (mantissa / 2^63) × 2^(exp − 0x3FFF)`:
//! `0xC000000000000000 / 2^63 = 1.5`, `2^(0x4000 − 0x3FFF) = 2` ⇒ `3.0`, and
//! `0xC333333333333333 / 2^63 = 1.525` ⇒ `3.05`.
//! All four branches reach the same struct via `*(*gvar_007DA6B8) + 4`, and that
//! struct is the `Data/Formula.Dat` payload: slot `w28` sits at `+0x80`
//! (read as `table+0x84`) and holds exactly `2.9` — see
//! `test/ts_decompile/.../0065b0dc_FUN_0065b0dc.asm.txt:17-33` and the loader
//! `src/data/loaders/formula.rs`. So reborn 0 is **2.9**, not a guess.
//!
//! The additive column is a **single shared column** — all four branches read
//! `*(i32*)(table + 0xAC)` (`:38, :72, :106, :140`) — which is `Formula.Dat`
//! slot `extra_exp` (struct `+0xA8`) = **5**. It is therefore not an inline
//! magic `+5`, but the client *does* add it once per level, so the cumulative
//! threshold keeps it. It is exposed as the server-controlled `base[lv]`
//! column ([`compute_texps_with_base`]).
//!
//! `ROUND` (`@ROUND` @ `0x00402b1c`) is Delphi `Round` = **half away from
//! zero**, not `round_ties_even` — see [`texp_round`].

use crate::data::tables::TexpRow;
use crate::protocol::MAX_LEVEL;

/// `k[reborn]` — exponent of `level` in the client's EXP formula
/// (`FUN_0065b0dc`). Reborn 0 is `Data/Formula.Dat` slot `w28` (struct `+0x80`,
/// read as `table+0x84`); reborns 1..3 are the 80-bit x87 constants at
/// `0065b0dc.asm.txt:64-66` (3.0), `:98-100` (3.05) and `:132-134` (3.0).
pub const TEXP_K: [f64; 4] = [2.9, 3.0, 3.05, 3.0];

/// Default value of the single shared column `*(i32*)(table + 0xAC)` =
/// `Data/Formula.Dat` slot `extra_exp` (struct `+0xA8`) = 5.
pub const TEXP_BASE_DEFAULT: i64 = 5;

/// Delphi `Round` — half away from zero (`@ROUND` @ `0x00402b1c`).
pub fn texp_round(x: f64) -> i64 {
    x.round() as i64
}

/// One level's EXP contribution — client `FUN_0065b0dc`:
/// `ROUND(level ^ k[reborn]) + base`.
pub fn texp_level_delta(level: i64, reborn: usize, base: i64) -> i64 {
    let k = TEXP_K[reborn.min(TEXP_K.len() - 1)];
    texp_round((level as f64).powf(k)) + base
}

/// The `base[lv]` column value for `level` (`base[0]` is level 1). Levels past
/// the end of `base` contribute 0 so a short column degrades gracefully.
fn base_at(base: &[i64], level: i64) -> i64 {
    if level < 1 {
        return 0;
    }
    base.get((level - 1) as usize).copied().unwrap_or(0)
}

/// EXP threshold of `level` — the client's `FUN_0065b398` running sum over
/// `FUN_0065b0dc(i, reborn)` for `i = 1..=level`.
pub fn texp_threshold(level: i64, reborn: usize, base: &[i64]) -> i64 {
    (1..=level.max(0))
        .map(|lv| texp_level_delta(lv, reborn, base_at(base, lv)))
        .sum()
}

/// Compute the full Texps table against a caller-supplied `base[lv]` column
/// (the shared `table+0xAC` column, one entry per level starting at level 1).
///
/// Each row is the client's cumulative threshold `FUN_0065b398`, i.e. the sum
/// of the per-level terms `ROUND(level ^ k[reborn]) + base[level]`, so a row is
/// never derived from `Texp[i-1]`; it is derived from the base column and
/// `k[reborn]` only.
pub fn compute_texps_with_base(base: &[i64]) -> Vec<TexpRow> {
    let mut rows: Vec<TexpRow> = Vec::with_capacity(MAX_LEVEL as usize);
    let mut acc = [0i64; TEXP_K.len()];
    for i in 0..MAX_LEVEL {
        let lvl = i + 1;
        for (r, slot) in acc.iter_mut().enumerate() {
            *slot += texp_level_delta(lvl, r, base_at(base, lvl));
        }
        rows.push(TexpRow {
            lv: i,
            reborn: acc,
        });
    }
    rows
}

/// Compute the full Texps table with the client's default column
/// (`Data/Formula.Dat` `extra_exp` = [`TEXP_BASE_DEFAULT`] for every level).
pub fn compute_texps() -> Vec<TexpRow> {
    let base = vec![TEXP_BASE_DEFAULT; MAX_LEVEL as usize];
    compute_texps_with_base(&base)
}

/// `TexpGetLvUp(lv, reborn, texp)` — returns the number of level-ups by
/// walking the Texps array from the given level (Chapter 6 §6.6).
///
/// `reborn` indexes 0..3 directly (reborn 3 uses `k = 3.0`, distinct from
/// reborn 2's `3.05`); out-of-range values clamp to 3.
pub fn texp_get_lv_up(texps: &[TexpRow], lv: i64, reborn: usize, texp: i64) -> i64 {
    let lv = lv as usize;
    let column = reborn.min(TEXP_K.len() - 1);
    let mut result = 0i64;
    if (lv as i64) < MAX_LEVEL {
        for (i, row) in texps
            .iter()
            .enumerate()
            .skip(lv)
            .take(MAX_LEVEL as usize - lv)
        {
            let threshold = row.reborn[column];
            if texp < threshold {
                return result;
            }
            if texp >= threshold {
                result = (i - lv) as i64 + 1;
            }
        }
    }
    result
}
