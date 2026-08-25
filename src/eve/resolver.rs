//! Tier 3 - AND chain grouping, 4-tier prioritized matching and event
//! resolution (chain match + GroupData probability filter).
//!
//! Source of truth: `Eve_NpcEventData.lua` where the `andNum` field defines
//! the AND chain structure; competing chains are ranked by quest-step score,
//! chain length, result count and declaration order.

use crate::battle::rng::DotNetRandom;
use crate::data::loaders::{EveCondition, EveGroupData, EveResult, NpcEventData};
use crate::eve::evaluator::evaluate;
use crate::eve::group::apply_group_data;
use crate::eve::state::PlayerEventState;
use std::collections::HashMap;
use tracing::debug;

/// Result of a successful AND chain match.
#[derive(Debug, Clone, PartialEq)]
pub struct ChainResolveResult {
    /// Result queue attached to the matched chain head.
    pub results: Vec<EveResult>,
    /// Matched chain head `conditionNo` (auto-chain loop detection key).
    pub matched_condition_no: i32,
    /// Highest quest-step threshold among cls=2 conditions (0 when absent).
    pub step_score: i32,
}

/// Predicate restricting which AND chains may match, used on re-evaluation
/// passes (e.g. "chain must contain conditionClass=8" after a battle).
pub type ChainFilter<'a> = &'a dyn Fn(&[EveCondition]) -> bool;

/// Group a condition list into AND chains.
///
/// `andNum` is the total length of the chain including the leading condition:
/// - 0: belongs to the previous chain (invalid at a start position; treated as 1)
/// - 1: standalone condition
/// - N: this condition plus the next N-1 conditions
///
/// Example: `[andNum=2, 0, 2, 0, 1, 1]` produces `[[0,1], [2,3], [4], [5]]`.
#[must_use]
pub fn build_chains(conditions: &[EveCondition]) -> Vec<&[EveCondition]> {
    let mut chains = Vec::new();
    let mut i = 0usize;
    while i < conditions.len() {
        let chain_len = usize::from(conditions[i].and_num).max(1);
        let chain_end = (i + chain_len).min(conditions.len());
        chains.push(&conditions[i..chain_end]);
        i = chain_end;
    }
    chains
}

/// Find the best fully-matching AND chain whose head carries results.
///
/// Matching strategy (4-tier sort):
/// 1. Primary: highest cls=2 (quest step) `conditionValue` in the chain
///    (higher threshold = more precise quest stage).
/// 2. Secondary: chain length (longer = more constraints = more precise).
/// 3. Tertiary: head result count (more = main handler, fewer = fallback
///    greeting).
/// 4. Final: declaration order (data order = design priority).
///
/// `chain_filter` optionally restricts candidates, e.g. "chain must contain
/// conditionClass=8" after a battle or "=10" after a dialogue choice.
pub fn resolve(
    conditions: &[EveCondition],
    evaluator: &dyn Fn(&EveCondition) -> bool,
    chain_filter: Option<ChainFilter<'_>>,
) -> Option<ChainResolveResult> {
    let chains = build_chains(conditions);
    let mut best: Option<ChainCandidate<'_>> = None;

    for chain in chains {
        let head = &chain[0];
        // The chain head must carry results.
        if head.results.is_empty() {
            continue;
        }
        // Optional extra filter for re-evaluation passes.
        if let Some(filter) = chain_filter {
            if !filter(chain) {
                continue;
            }
        }
        // Every condition in the chain must hold simultaneously.
        if !chain.iter().all(evaluator) {
            continue;
        }

        let step_score = chain
            .iter()
            .filter(|c| c.condition_class == 2)
            .map(|c| c.condition_value)
            .max()
            .unwrap_or(0);
        let candidate = ChainCandidate { chain, step_score };

        let better = match &best {
            None => true,
            Some(current) => candidate.is_better_than(current),
        };
        if better {
            best = Some(candidate);
        }
    }

    let winner = best?;
    debug!(
        cond_nos = ?winner.chain.iter().map(|c| c.condition_no).collect::<Vec<_>>(),
        result_count = winner.chain[0].results.len(),
        step_score = winner.step_score,
        "matched AND chain"
    );
    Some(ChainResolveResult {
        results: winner.chain[0].results.clone(),
        matched_condition_no: i32::from(winner.chain[0].condition_no),
        step_score: winner.step_score,
    })
}

struct ChainCandidate<'a> {
    chain: &'a [EveCondition],
    step_score: i32,
}

impl ChainCandidate<'_> {
    /// 4-tier comparison; declaration order wins ties by never displacing
    /// an earlier equal candidate.
    fn is_better_than(&self, current: &ChainCandidate<'_>) -> bool {
        if self.step_score != current.step_score {
            return self.step_score > current.step_score;
        }
        if self.chain.len() != current.chain.len() {
            return self.chain.len() > current.chain.len();
        }
        self.chain[0].results.len() > current.chain[0].results.len()
    }
}

/// Evaluate one event's condition chains and apply GroupData probability
/// filtering to the winning result queue.
///
/// Pure resolution semantics: no session is created and no state is mutated.
/// The state snapshot is copied with `current_eve_no` set so cls=12
/// conditions read the correct completion counter.
pub fn resolve_event(
    event_data: &NpcEventData,
    state: &PlayerEventState,
    group_datas: &HashMap<u16, EveGroupData>,
    rng: &mut DotNetRandom,
    chain_filter: Option<ChainFilter<'_>>,
) -> Option<ChainResolveResult> {
    let mut state_with_eve = state.clone();
    state_with_eve.current_eve_no = i32::from(event_data.eve_no);

    let mut chain_result = resolve(
        &event_data.conditions,
        &|cond| evaluate(cond, &state_with_eve),
        chain_filter,
    )?;

    chain_result.results = apply_group_data(chain_result.results, group_datas, rng);
    Some(chain_result)
}
