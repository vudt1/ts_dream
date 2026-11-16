//! Parity tests for `battle::engine::{get_hp_max, get_sp_max}` against the
//! client `FUN_0065a554` (HP) and `FUN_0065ad10` (SP) — ticket 11.
//!
//! Goldens are decoded from the ASM branch tables plus the `Data/Formula.Dat`
//! coefficients (`w35 = 1.0`, `w37 = 0.35`, `w40 = 0.25`, `base_hp = 80`,
//! `base_sp = 60`), not from the Rust functions, so they lock the behaviour.
//! Pure logic — no DB, no network.

use ts_dream::battle::engine::{get_hp_max, get_sp_max};

/// `(reborn, job, level, hpx) -> max HP`, one row per distinct ASM branch.
#[test]
fn hp_matches_client_sample_set() {
    let cases: &[(i64, i64, i64, i64, i64)] = &[
        (0, 0, 1, 0, 81),      // 0x65a583
        (0, 0, 2, 1, 87),      // rounding discriminator (see below)
        (0, 0, 10, 10, 155),   // 0x65a583
        (0, 0, 50, 10, 229),
        (0, 0, 100, 25, 481),
        (1, 0, 50, 10, 349),   // 0x65a66d
        (2, 1, 50, 10, 678),   // 0x65a778 (mult 2.5)
        (2, 2, 50, 10, 848),   // 0x65a841 (mult 3.0)
        (2, 3, 50, 10, 539),   // 0x65a90e (add 23)
        (2, 4, 50, 10, 519),   // 0x65a9db (add 21)
        (3, 2, 50, 10, 519),   // reborn 3 -> server safe fallback
    ];
    for &(rb, job, lv, hpx, want) in cases {
        assert_eq!(
            get_hp_max(rb, job, lv, hpx),
            want,
            "hp rb={rb} job={job} lv={lv} hpx={hpx}"
        );
    }
}

/// `(reborn, job, level, spx) -> max SP`, one row per distinct ASM branch.
#[test]
fn sp_matches_client_sample_set() {
    let cases: &[(i64, i64, i64, i64, i64)] = &[
        (0, 0, 1, 0, 61),      // 0x65ad3f
        (0, 0, 10, 5, 88),     // rounding discriminator (see below)
        (0, 0, 10, 10, 106),
        (0, 0, 50, 10, 163),
        (0, 0, 100, 25, 318),
        (1, 0, 50, 10, 213),   // 0x65ae06
        (2, 1, 50, 10, 263),   // 0x65aee3 (jobs 1|2)
        (2, 2, 50, 10, 263),
        (2, 3, 50, 10, 440),   // 0x65af8a (mult 3.0)
        (2, 4, 50, 10, 553),   // 0x65b031 (mult 3.5)
        (3, 0, 50, 10, 553),   // reborn 3 -> server safe fallback
    ];
    for &(rb, job, lv, spx, want) in cases {
        assert_eq!(
            get_sp_max(rb, job, lv, spx),
            want,
            "sp rb={rb} job={job} lv={lv} spx={spx}"
        );
    }
}

/// The client uses `@ROUND` (half away from zero); the old `.floor()` was off
/// by one whenever the fractional part reached 0.5.
#[test]
fn rounds_half_away_instead_of_floord() {
    // 2^0.35 = 1.2746: (1.2746 + 1) * 1 * 2 + 80 + 2 = 86.549 -> 87 (floor 86).
    assert_eq!(get_hp_max(0, 0, 2, 1), 87);
    // 10^0.25 = 1.7783: 1.7783 * 5 * 2 + 60 + 10 = 87.783 -> 88 (floor 87).
    assert_eq!(get_sp_max(0, 0, 10, 5), 88);
}

/// Reborn 0 and 1 dispatch only on reborn; `job` is ignored.
#[test]
fn reborn_0_and_1_ignore_job() {
    for job in 0..5 {
        assert_eq!(get_hp_max(0, job, 50, 10), 229, "hp rb0 job {job}");
        assert_eq!(get_hp_max(1, job, 50, 10), 349, "hp rb1 job {job}");
        assert_eq!(get_sp_max(0, job, 50, 10), 163, "sp rb0 job {job}");
        assert_eq!(get_sp_max(1, job, 50, 10), 213, "sp rb1 job {job}");
    }
}

/// Reborn 2 dispatches on `job` 1..4; anything else falls back to the job-4
/// column (the client instead returns 0, which the server must not mirror).
#[test]
fn reborn_2_dispatches_on_job() {
    assert_eq!(get_hp_max(2, 1, 50, 10), 678);
    assert_eq!(get_hp_max(2, 2, 50, 10), 848);
    assert_eq!(get_hp_max(2, 3, 50, 10), 539);
    assert_eq!(get_hp_max(2, 4, 50, 10), 519);
    assert_eq!(get_hp_max(2, 0, 50, 10), 519);
    assert_eq!(get_hp_max(2, 9, 50, 10), 519);

    assert_eq!(get_sp_max(2, 1, 50, 10), 263);
    assert_eq!(get_sp_max(2, 2, 50, 10), 263);
    assert_eq!(get_sp_max(2, 3, 50, 10), 440);
    assert_eq!(get_sp_max(2, 4, 50, 10), 553);
    assert_eq!(get_sp_max(2, 0, 50, 10), 553);
    assert_eq!(get_sp_max(2, 9, 50, 10), 553);
}

/// Reborn 3 falls through to the client epilogue (returns 0); the server keeps
/// the safe job-4 column regardless of `job` so stats never drop to 0.
#[test]
fn reborn_3_uses_safe_fallback_for_any_job() {
    for job in 0..5 {
        assert_eq!(get_hp_max(3, job, 50, 10), 519, "hp rb3 job {job}");
        assert_eq!(get_sp_max(3, job, 50, 10), 553, "sp rb3 job {job}");
    }
}
