//! Parity tests for `src/data/texps.rs` against the client EXP table
//! (ticket 11).
//!
//! The `k[]` exponents are decoded from `aLogin.exe` (`FUN_0065b0dc`,
//! `test/ts_decompile/functions/0065b0dc_FUN_0065b0dc.asm.txt`) and the shared
//! additive column from `Data/Formula.Dat`. The "golden" cumulative
//! thresholds below are reproduced from that decode (half-away-from-zero
//! `ROUND`), **not** from the Rust implementation, so they lock the values.
//!
//! Every value here is pure logic — no DB, no network.

use std::fs;
use std::path::PathBuf;

use ts_dream::data::loaders::FormulaDatLoader;
use ts_dream::data::texps::{
    compute_texps, compute_texps_with_base, texp_get_lv_up, texp_level_delta, texp_round,
    texp_threshold, TEXP_BASE_DEFAULT, TEXP_K,
};
use ts_dream::protocol::MAX_LEVEL;

fn data_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("Data")
}

fn default_base() -> Vec<i64> {
    vec![TEXP_BASE_DEFAULT; MAX_LEVEL as usize]
}

fn zero_base() -> Vec<i64> {
    vec![0; MAX_LEVEL as usize]
}

/// `k[reborn]` is the decoded client exponent: reborn 0 is the
/// `Data/Formula.Dat` slot `w28` (2.9, read from RAM at `0x65B101`), reborns
/// 1..3 are the 80-bit x87 constants in the ASM (`3.0`, `3.05`, `3.0`).
#[test]
fn k_table_is_client_decoded() {
    assert_eq!(TEXP_K, [2.9, 3.0, 3.05, 3.0]);
}

/// `@ROUND` (`0x00402b1c`) is Delphi `Round` = half away from zero, not
/// `round_ties_even` (banker's rounding).
#[test]
fn round_is_half_away_from_zero() {
    assert_eq!(texp_round(0.5), 1);
    assert_eq!(texp_round(1.5), 2);
    assert_eq!(texp_round(2.5), 3, "banker's rounding would give 2");
    assert_eq!(texp_round(-0.5), -1);
    assert_eq!(texp_round(-2.5), -3, "banker's rounding would give -2");
    assert_eq!(texp_round(2.4), 2);
    assert_eq!(texp_round(2.6), 3);
}

/// One level's contribution is `ROUND(level^k[reborn]) + base`.
#[test]
fn level_delta_per_reborn() {
    // ROUND(1^k) = 1 for every reborn.
    for r in 0..4 {
        assert_eq!(texp_level_delta(1, r, TEXP_BASE_DEFAULT), 1 + 5);
    }

    // 10^2.9 = 794.33 -> 794, 10^3 = 1000, 10^3.05 = 1122.02 -> 1122.
    assert_eq!(texp_level_delta(10, 0, TEXP_BASE_DEFAULT), 799);
    assert_eq!(texp_level_delta(10, 1, TEXP_BASE_DEFAULT), 1005);
    assert_eq!(texp_level_delta(10, 2, TEXP_BASE_DEFAULT), 1127);
    assert_eq!(texp_level_delta(10, 3, TEXP_BASE_DEFAULT), 1005);

    // 50^3 = 125000 -> 125005 for both reborn 1 and 3.
    assert_eq!(texp_level_delta(50, 1, TEXP_BASE_DEFAULT), 125_005);
    assert_eq!(texp_level_delta(50, 3, TEXP_BASE_DEFAULT), 125_005);
}

/// Cumulative thresholds match the client decode at fixed checkpoints
/// (base column = `Formula.Dat` `extra_exp` = 5).
#[test]
fn cumulative_thresholds_match_client() {
    const L1: [i64; 4] = [6, 6, 6, 6];
    const L2: [i64; 4] = [18, 19, 19, 19];
    const L10: [i64; 4] = [2502, 3075, 3410, 3075];
    const L50: [i64; 4] = [1126642, 1625875, 1953632, 1625875];
    const L99: [i64; 4] = [15864933, 24502995, 30458810, 24502995];
    const L100: [i64; 4] = [16495895, 25503000, 31717740, 25503000];

    let base = default_base();
    let texps = compute_texps_with_base(&base);

    for (lv, want) in [
        (1usize, L1),
        (2, L2),
        (10, L10),
        (50, L50),
        (99, L99),
        (100, L100),
    ] {
        assert_eq!(texps[lv - 1].reborn, want, "level {lv}");
    }

    // `texp_threshold` (an explicit sum) must agree with the table rows.
    for (lv, want) in [(1usize, L1), (10, L10), (100, L100)] {
        for (r, expected) in want.iter().enumerate() {
            assert_eq!(
                texp_threshold(lv as i64, r, &base),
                *expected,
                "threshold level {lv} reborn {r}"
            );
        }
    }
}

/// `compute_texps()` is just the default (`extra_exp = 5`) column.
#[test]
fn compute_texps_uses_default_column() {
    assert_eq!(compute_texps().len(), MAX_LEVEL as usize);
    let base = default_base();
    let explicit = compute_texps_with_base(&base);
    let default = compute_texps();
    for (a, b) in explicit.iter().zip(default.iter()) {
        assert_eq!(a.reborn, b.reborn);
    }
}

/// The additive column is a distinct, server-supplied `base[lv]` array; each
/// level reads its own entry (no chaining from `Texp[i-1]`).
#[test]
fn base_column_is_per_level_array() {
    let zero = zero_base();
    let t0 = compute_texps_with_base(&zero);
    assert_eq!(t0[0].reborn, [1, 1, 1, 1]);
    assert_eq!(t0[9].reborn, [2452, 3025, 3360, 3025]);
    assert_eq!(t0[49].reborn, [1126392, 1625625, 1953382, 1625625]);

    // A constant column of 10 shifts every row by `10 * level`.
    let ten = vec![10i64; MAX_LEVEL as usize];
    let t10 = compute_texps_with_base(&ten);
    for i in 0..MAX_LEVEL as usize {
        for r in 0..4 {
            assert_eq!(
                t10[i].reborn[r],
                t0[i].reborn[r] + 10 * (i as i64 + 1),
                "level {} reborn {r}",
                i + 1
            );
        }
    }

    // A per-level column: only level 1 is 0, the rest 100.
    let mut column = vec![100i64; MAX_LEVEL as usize];
    column[0] = 0;
    let per_level = compute_texps_with_base(&column);
    // Level 1 has no additive; level 2 accumulates 100 once; level 3 twice.
    assert_eq!(per_level[0].reborn, t0[0].reborn);
    assert_eq!(per_level[1].reborn[0], t0[1].reborn[0] + 100);
    assert_eq!(per_level[2].reborn[0], t0[2].reborn[0] + 200);
}

/// Reborn 3 is a real column (`k = 3.0`), not a clamp of reborn 2 (`k = 3.05`),
/// and it coincides with reborn 1 (same exponent).
#[test]
fn reborn_3_is_distinct_from_reborn_2() {
    let texps = compute_texps();
    let last = &texps[MAX_LEVEL as usize - 1];
    assert_ne!(last.reborn[2], last.reborn[3]);
    for row in &texps {
        assert_eq!(row.reborn[1], row.reborn[3]);
    }
}

/// `texp_get_lv_up` indexes reborn 0..3 directly. Reading column 3 must not be
/// clamped to column 2.
#[test]
fn texp_get_lv_up_indexes_reborn_0_through_3() {
    let texps = compute_texps();

    // No exp at level 1 -> no level-up for any reborn.
    for r in 0..4 {
        assert_eq!(texp_get_lv_up(&texps, 1, r, 0), 0);
    }

    // Reborn 3's own level-10 threshold is cheaper than reborn 2's, so at that
    // exact exp reborn 3 clears 9 level-ups while reborn 2 clears only 8. If
    // column 3 were clamped to column 2 the first call would return 8 too.
    let t = texps[9].reborn[3];
    assert_eq!(texp_get_lv_up(&texps, 1, 3, t), 9, "reborn 3 reads column 3");
    assert_eq!(texp_get_lv_up(&texps, 1, 2, t), 8, "reborn 2 is stricter");

    // An out-of-range reborn must not panic (clamps to the last column).
    let _ = texp_get_lv_up(&texps, 1, 9, 1_000_000_000);
}

/// The constants the client reads from `Data/Formula.Dat` are exactly the
/// coefficients `texps.rs` and `battle::engine` hard-code. Skipped if the data
/// file is absent (e.g. a stripped CI checkout).
#[test]
fn formula_dat_constants_match_client_coefficients() {
    let path = data_dir().join("Formula.Dat");
    if !path.exists() {
        eprintln!("Skipping test: Data/Formula.Dat not found");
        return;
    }

    let bytes = fs::read(&path).expect("read Formula.Dat");
    let f = FormulaDatLoader::load(&bytes).expect("parse Formula.Dat");

    // EXP table: reborn-0 exponent and the shared additive column.
    assert_eq!(f.w28, TEXP_K[0], "w28 is k[reborn 0]");
    assert_eq!(
        f.extra_exp as i64, TEXP_BASE_DEFAULT,
        "extra_exp is the shared base column"
    );

    // HP/SP coefficients used by `battle::engine::{get_hp_max,get_sp_max}`.
    assert_eq!(f.w37, 0.35, "HP level exponent");
    assert_eq!(f.w35, 1.0, "HP/SP level coefficient");
    assert_eq!(f.w40, 0.25, "SP level exponent");
    assert_eq!(f.w38, 1.0, "SP level coefficient");
    assert_eq!(f.w36, 2.0, "HP CX multiplier (reborn 0/1)");
    assert_eq!(f.w39, 2.0, "SP CX multiplier (reborn 0/1/2 jobs 1-2)");
    assert_eq!(f.base_hp, 80, "HP base (reborn 0)");
    assert_eq!(f.base_sp, 60, "SP base (reborn 0)");
}
