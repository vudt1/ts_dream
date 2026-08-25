//! GroupData weighted probability selection for event results.
//!
//! A result with `resultGroupNo > 0` references a GroupData table: one member
//! is picked per group (weighted random or fixed) and only the matching
//! `resultNo` survives filtering. `resultGroupNo == 0` results always pass.

use crate::battle::rng::DotNetRandom;
use crate::data::loaders::{EveGroupData, EveResult};
use std::collections::HashMap;
use tracing::{debug, warn};

/// Sentinel meaning "keep every member of this group".
const KEEP_ALL: i32 = -1;

/// Apply GroupData probability filtering to a result queue.
///
/// - `resultGroupNo == 0`: initial group, always kept.
/// - `resultGroupNo > 0`: look up the GroupData table, pick one member per
///   group and keep only the result whose `resultNo` matches the pick. A
///   missing table keeps all of that group's results.
#[must_use]
pub fn apply_group_data(
    results: Vec<EveResult>,
    group_datas: &HashMap<u16, EveGroupData>,
    rng: &mut DotNetRandom,
) -> Vec<EveResult> {
    if results.iter().all(|r| r.result_group_no == 0) {
        return results;
    }

    let mut selections: HashMap<u16, i32> = HashMap::new();

    for result in &results {
        let group_no = result.result_group_no;
        if group_no == 0 || selections.contains_key(&group_no) {
            continue;
        }

        match group_datas.get(&group_no) {
            None => {
                warn!(group_no, "GroupData missing; keeping all group members");
                selections.insert(group_no, KEEP_ALL);
            }
            Some(group_data) => {
                let selected_no = select_member(group_data, rng);
                debug!(
                    group_no,
                    selected_no,
                    use_mode = group_data.use_mode,
                    "group selection"
                );
                selections.insert(group_no, selected_no);
            }
        }
    }

    results
        .into_iter()
        .filter(|result| match selections.get(&result.result_group_no) {
            None => true,
            Some(&KEEP_ALL) => true,
            Some(&selected_no) => result.result_no as i32 == selected_no,
        })
        .collect()
}

/// Pick one member from a GroupData table by probability.
///
/// Returns the chosen `resultNo`, or [`KEEP_ALL`] when the whole group should
/// be preserved (no members, zero weights).
fn select_member(group_data: &EveGroupData, rng: &mut DotNetRandom) -> i32 {
    // Designer-pinned selection wins outright.
    if group_data.pick_member > 0 {
        return i32::from(group_data.pick_member);
    }
    // Single member needs no dice roll.
    if group_data.member_no_ay.len() == 1 {
        return i32::from(group_data.member_no_ay[0]);
    }
    if group_data.member_no_ay.is_empty() {
        return KEEP_ALL;
    }

    let total_weight: i32 = group_data.probability_rate_ay.iter().map(|&w| i32::from(w)).sum();
    if total_weight <= 0 {
        // All-zero weights mean "designed as one unit" - keep everything.
        return KEEP_ALL;
    }

    let mut roll = rng.next_max(total_weight);
    for (i, &member_no) in group_data.member_no_ay.iter().enumerate() {
        roll -= i32::from(group_data.probability_rate_ay[i]);
        if roll < 0 {
            return i32::from(member_no);
        }
    }
    i32::from(group_data.member_no_ay[group_data.member_no_ay.len() - 1])
}
