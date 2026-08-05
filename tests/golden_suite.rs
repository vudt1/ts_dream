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
/// handler output. Ignored by default; run manually when behaviour changes.
#[tokio::test]
#[ignore]
async fn regenerate_goldens() {
    common::regenerate("golden", "Golden scenario (ticket 23, Ch9)").await;
}
