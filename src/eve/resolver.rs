//! Tier 3 - AND chain grouping, 3-tier prioritized matching (plus declaration-order tie-break) and event
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
    /// Result queue collected across all conditions in the matched chain.
    pub results: Vec<EveResult>,
    /// Matched chain head `conditionNo` (auto-chain loop detection key).
    pub matched_condition_no: i32,
    /// Highest quest-step threshold among cls=2 conditions (0 when absent).
    pub step_score: i32,
}

/// Predicate restricting which AND chains may match, used on re-evaluation
/// passes (e.g. "chain must contain conditionClass=8" after a battle).
pub type ChainFilter<'a> = &'a dyn Fn(&[EveCondition]) -> bool;

/// Returns `true` if `chain` contains a `conditionClass=10` (DialogChoice)
/// condition matching `(surface_id, choice_code)`.
#[must_use]
pub fn chain_has_dialog_choice(
    chain: &[EveCondition],
    surface_id: i32,
    choice_code: i32,
) -> bool {
    chain.iter().any(|c| {
        c.condition_class == 10
            && i32::from(c.condition_parameter) == surface_id
            && i32::from(c.condition_parameter_style) == choice_code
    })
}

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

/// Find the best fully-matching AND chain whose combined results are non-empty.
///
/// Matching strategy (3-tier sort + declaration-order tie-break):
/// 1. Primary: highest cls=2 (quest step) `conditionValue` in the chain
///    (higher threshold = more precise quest stage).
/// 2. Secondary: chain length (longer = more constraints = more precise).
/// 3. Tertiary: total result count across all conditions in the chain
///    (more = main handler, fewer = fallback greeting).
/// 4. Tie-break: declaration order (data order = design priority).
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
        // The combined chain must carry at least one result.
        let result_count: usize = chain.iter().map(|c| c.results.len()).sum();
        if result_count == 0 {
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
        let candidate = ChainCandidate {
            chain,
            step_score,
            result_count,
        };

        let better = match &best {
            None => true,
            Some(current) => candidate.is_better_than(current),
        };
        if better {
            best = Some(candidate);
        }
    }

    let winner = best?;
    let results: Vec<EveResult> = winner
        .chain
        .iter()
        .flat_map(|c| c.results.iter().cloned())
        .collect();
    debug!(
        cond_nos = ?winner.chain.iter().map(|c| c.condition_no).collect::<Vec<_>>(),
        result_count = results.len(),
        step_score = winner.step_score,
        "matched AND chain"
    );
    Some(ChainResolveResult {
        results,
        matched_condition_no: i32::from(winner.chain[0].condition_no),
        step_score: winner.step_score,
    })
}

struct ChainCandidate<'a> {
    chain: &'a [EveCondition],
    step_score: i32,
    result_count: usize,
}

impl ChainCandidate<'_> {
    /// 3-tier comparison + declaration-order tie-break (earlier equal candidate
    /// is never displaced).
    fn is_better_than(&self, current: &ChainCandidate<'_>) -> bool {
        if self.step_score != current.step_score {
            return self.step_score > current.step_score;
        }
        if self.chain.len() != current.chain.len() {
            return self.chain.len() > current.chain.len();
        }
        self.result_count > current.result_count
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
