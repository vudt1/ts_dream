//! Golden scenario suite — the byte-level acceptance gate (ticket 23, Ch9 §9.4/§9.5).
//!
//! Loads every `golden/*.golden`, replays it against the Rust server code
//! (the opcode dispatcher for feature handlers, the seeded `BattleService`
//! for the async battle sample) and fails on **any** frame diff. Runs green
//! as a plain `cargo test` — no live server, DB, or socket needed.
//!
//! `cargo test --test golden_suite -- --ignored regenerate_goldens` re-locks
//! the synchronous scenarios from the current code (reproducible re-capture).

mod common;

/// Gate: every golden scenario replays byte-exact.
#[tokio::test]
async fn golden_scenarios_replay_byte_exact() {
    common::run_all_goldens().await;
}

/// Re-capture helper: rewrites the synchronous golden files from the current
/// handler output. Ignored by default AND gated behind `TS_REGENERATE_GOLDENS=1`
/// so it can never be run accidentally (Ch9 §9.4).
///
/// The golden files are the byte-level contract: they must come from analysis
/// of the C# `Logined1`/handlers (or a real C#↔client capture), never blindly
/// regenerated from the Rust output — otherwise the diff gate degrades into
/// "Rust diffs against itself" and stops guarding actual wire parity.
#[tokio::test]
#[ignore]
async fn regenerate_goldens() {
    assert!(
        std::env::var("TS_REGENERATE_GOLDENS").is_ok(),
        "regeneration is gated behind TS_REGENERATE_GOLDENS=1; re-run with \
         the env var set, and diff the result against the C# reference before \
         committing"
    );
    common::regenerate("golden", "Golden scenario (ticket 23, Ch9)").await;
}
