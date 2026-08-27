//! Eve Script Engine — 4-tier NPC event / dialogue / quest scripting runtime.
//!
//! Replaces the legacy quest `.txt`/`.ini` system with the `eve.emg` script
//! architecture ported from the Kotlin TS Mobile reference server:
//!
//! - Tier 1 — NpcEvent / Door dispatch inputs: [`auto_chain::EveAutoChainEngine`]
//!   walks an NPC or door event list and chains to the next matching event.
//! - Tier 2 — Condition evaluation: [`evaluator`] judges 15 condition classes
//!   against a pure [`state::PlayerEventState`] snapshot.
//! - Tier 3 — Result resolution & auto-chain: [`resolver`] groups conditions
//!   into AND chains, applies 4-tier prioritization and hands results to the
//!   auto-chain engine for seamless continuation.
//! - Tier 4 — Group random pick & state snapshots: [`group`] applies weighted
//!   `GroupData` probability selection; [`state::EveStateBuilder`] captures the
//!   player state consumed by every other tier.

pub mod auto_chain;
pub mod evaluator;
pub mod group;
pub mod resolver;
pub mod state;

pub use auto_chain::{
    AutoChainResult, EveAutoChainEngine, EventPhase, EventSession, MAX_CHAIN_DEPTH,
};
pub use evaluator::{compare_step, evaluate};
pub use group::apply_group_data;
pub use resolver::{build_chains, resolve, resolve_event, ChainResolveResult};
pub use state::{EveStateBuilder, PlayerEventState, PlayerStateInputs};
