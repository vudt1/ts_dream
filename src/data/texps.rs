//! Texps — computed cumulative EXP thresholds (Chapter 3 §3.5).
//!
//! For level `i` in `0..MaxLevel-1`:
//! `Texp0[i] = Texp0[i-1] + round(Pow(i+1, 2.9)) + 5`
//! `Texp1[i] = ... Pow(i+1, 3.0) ... + 5`
//! `Texp2[i] = ... Pow(i+1, 3.05) ... + 5` (cumulative).

use crate::data::tables::TexpRow;
use crate::protocol::MAX_LEVEL;

/// Compute the full Texps table exactly as specified:
/// `for i in 0..MaxLevel`, each row accumulates from the previous,
/// `_k(i) = _k(i-1) + round(Pow(i+1, exp_k)) + 5`. So
/// `Texps[0] = 6` (Pow(1, e) = 1) — not a zero base. Rounding is
/// round-half-to-even (banker's rounding).
pub fn compute_texps() -> Vec<TexpRow> {
    let mut rows: Vec<TexpRow> = Vec::with_capacity(MAX_LEVEL as usize);
    let mut acc = [0.0f64; 3];
    for i in 0..MAX_LEVEL {
        let lvl = (i + 1) as f64;
        let r0 = acc[0] + banker_round(lvl.powf(2.9)) + 5.0;
        let r1 = acc[1] + banker_round(lvl.powf(3.0)) + 5.0;
        let r2 = acc[2] + banker_round(lvl.powf(3.05)) + 5.0;
        acc = [r0, r1, r2];
        rows.push(TexpRow {
            lv: i,
            reborn: [r0 as i64, r1 as i64, r2 as i64],
        });
    }
    rows
}

/// Round-half-to-even (banker's rounding).
fn banker_round(x: f64) -> f64 {
    x.round_ties_even()
}

/// `TexpGetLvUp(lv, reborn, texp)` — returns the number of level-ups by
/// walking the Texps array from the given level (Chapter 6 §6.6).
pub fn texp_get_lv_up(texps: &[TexpRow], lv: i64, reborn: usize, texp: i64) -> i64 {
    let lv = lv as usize;
    let mut result = 0i64;
    if (lv as i64) < MAX_LEVEL {
        for (i, row) in texps
            .iter()
            .enumerate()
            .skip(lv)
            .take(MAX_LEVEL as usize - lv)
        {
            let threshold = row.reborn[reborn.min(2)];
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
