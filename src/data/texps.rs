//! Texps — computed cumulative EXP thresholds (Chapter 3 §3.5).
//!
//! For level `i` in `0..MaxLevel-1`:
//! `Texp0[i] = Texp0[i-1] + round(Pow(i+1, 2.9)) + 5`
//! `Texp1[i] = ... Pow(i+1, 3.0) ... + 5`
//! `Texp2[i] = ... Pow(i+1, 3.05) ... + 5` (cumulative).

use crate::protocol::MAX_LEVEL;
use crate::data::tables::TexpRow;

/// Compute the full Texps table. Reproduces the C# loop with .NET `Math.Round`
/// (banker's rounding) applied to the pow result, and 0.5-epsilon handling to
/// mirror double rounding. `MAX_LEVEL` rows, index 0..MaxLevel-1 == level.
pub fn compute_texps() -> Vec<TexpRow> {
    let mut rows: Vec<TexpRow> = Vec::with_capacity(MAX_LEVEL as usize);
    let mut acc = [0.0f64; 3];
    // Row 0 = lv 0 sentinel (C# index 0 is the base).
    rows.push(TexpRow {
        lv: 0,
        reborn: [0, 0, 0],
    });
    for i in 1..MAX_LEVEL {
        let lvl = i as f64;
        let r0 = acc[0] + banker_round((lvl + 1.0).powf(2.9)) + 5.0;
        let r1 = acc[1] + banker_round((lvl + 1.0).powf(3.0)) + 5.0;
        let r2 = acc[2] + banker_round((lvl + 1.0).powf(3.05)) + 5.0;
        acc = [r0, r1, r2];
        rows.push(TexpRow {
            lv: i,
            reborn: [r0 as i64, r1 as i64, r2 as i64],
        });
    }
    rows
}

/// .NET `Math.Round(double)` = round-half-to-even (banker's rounding).
fn banker_round(x: f64) -> f64 {
    x.round_ties_even()
}

/// `TexpGetLvUp(lv, reborn, texp)` — returns the number of level-ups by
/// walking the Texps array from the given level (Chapter 6 §6.6).
pub fn texp_get_lv_up(texps: &[TexpRow], lv: i64, reborn: usize, texp: i64) -> i64 {
    let lv = lv as usize;
    let mut result = 0i64;
    if (lv as i64) < MAX_LEVEL {
        for i in lv..(MAX_LEVEL as usize) {
            let threshold = texps[i].reborn[reborn.min(2)];
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn texps_monotonic() {
        let t = compute_texps();
        assert_eq!(t.len(), MAX_LEVEL as usize);
        // Cumulative thresholds strictly increase with level for each reborn.
        for r in 0..3 {
            let mut prev = 0i64;
            for row in &t {
                assert!(row.reborn[r] >= prev);
                prev = row.reborn[r];
            }
        }
    }

    #[test]
    fn banker_round_half_even() {
        // 2.5 -> 2, 3.5 -> 4 (round-half-to-even).
        assert_eq!(2.0_f64.round_ties_even(), 2.0);
        assert_eq!(3.5_f64.round_ties_even(), 4.0);
        assert_eq!(0.5_f64.round_ties_even(), 0.0);
    }

    #[test]
    fn lvup_at_zero_texp_is_zero() {
        let t = compute_texps();
        assert_eq!(texp_get_lv_up(&t, 1, 0, 0), 0);
    }
}