//! Tier 2 — pure condition evaluation for Eve scripts.
//!
//! The client performs no condition judging; every branch decision is made
//! here against a [`PlayerEventState`] snapshot. Semantics derive from
//! `Eve_NpcEventData.lua` field naming plus observed game behavior.
//!
//! Verified condition classes: 0, 1, 2, 7, 8, 9, 10, 12, 14.
//! Unimplemented classes (3, 4, 5, 6, 11, 13, 15) fail conservatively.

use crate::data::loaders::EveCondition;
use crate::eve::state::PlayerEventState;
use tracing::debug;

/// Comparison operator table (`conditionOps`).
///
/// `ops=0` means *equal* in the Eve system — this differs from
/// `DungeonCriteriaData.CompareResult`, where 0 is always false.
#[must_use]
pub fn compare_step(actual: i32, ops: u8, expected: i32) -> bool {
    match ops {
        0 => actual == expected,
        1 => actual == expected,
        2 => actual < expected,
        3 => actual <= expected,
        4 => actual > expected,
        5 => actual >= expected,
        6 => actual != expected,
        _ => true,
    }
}

/// Evaluate a single condition against the player state snapshot.
#[must_use]
pub fn evaluate(cond: &EveCondition, state: &PlayerEventState) -> bool {
    let param = i32::from(cond.condition_parameter);
    let p_style = cond.condition_parameter_style;

    match cond.condition_class {
        // Unconditional — always matches.
        0 => true,

        // Bag item possession, reversed comparison:
        //   val=0 ops=2 → 0 < qty → has the item
        //   val=0 ops=5 → 0 >= qty → does not have the item
        1 => {
            let qty = state.bag_items.get(&param).copied().unwrap_or(0);
            let result = compare_step(cond.condition_value, cond.condition_ops, qty);
            debug!(
                class = 1, item_id = param, qty, ops = cond.condition_ops,
                val = cond.condition_value, "bag item condition -> {}", result
            );
            result
        }

        // Quest step. pStyle=3 inspects max(step, flag) so completed missions
        // (step=0, flag=1) still satisfy `actual >= 1`.
        2 => {
            let step = state.mission_steps.get(&param).copied().unwrap_or(0);
            let actual = if p_style == 3 {
                let flag = state.mission_flags.get(&param).copied().unwrap_or(0);
                step.max(flag)
            } else {
                step
            };
            let result = compare_step(actual, cond.condition_ops, cond.condition_value);
            debug!(
                class = 2, mission_id = param, actual, ops = cond.condition_ops,
                val = cond.condition_value, "quest step condition -> {}", result
            );
            result
        }

        // Generic attribute; `param` selects the sub-type.
        7 => {
            let actual = match cond.condition_parameter {
                0 => state.level,
                2 => state.reborn_count,
                // Equipment possession: value is the itemId when worn, else 0.
                3 => {
                    if state.equip_items.contains(&cond.condition_value) {
                        cond.condition_value
                    } else {
                        0
                    }
                }
                // General storage slot: pStyle=1 follow / 2 cart / 3 inn,
                // value is the npcId when stored there, else 0. Unknown
                // pStyles never own (conservative).
                6 => {
                    let owned = match p_style {
                        1 => state.follow_npc_ids.contains(&cond.condition_value),
                        2 => state.cart_npc_ids.contains(&cond.condition_value),
                        3 => state.inn_npc_ids.contains(&cond.condition_value),
                        _ => false,
                    };
                    if owned {
                        cond.condition_value
                    } else {
                        0
                    }
                }
                // Unknown sub-type: ops=0/1 with val=0 degrades to `true`,
                // mirroring the reference behavior.
                _ => 0,
            };
            // Numeric attributes (level, reborn) use reversed comparison like
            // class=1 — Eve data format is `conditionValue [ops] actual`, so
            // ops=2 val=24 reads "24 < level" i.e. level above 24. Possession
            // sub-types stay forward (`actual ops value`).
            let result = if cond.condition_parameter == 0 || cond.condition_parameter == 2 {
                compare_step(cond.condition_value, cond.condition_ops, actual)
            } else {
                compare_step(actual, cond.condition_ops, cond.condition_value)
            };
            debug!(
                class = 7, param = cond.condition_parameter, actual,
                ops = cond.condition_ops, val = cond.condition_value,
                "attribute condition -> {}", result
            );
            result
        }

        // Battle outcome: pStyle=1 win / 2 lose / 3 flee.
        8 => {
            let result = i32::from(p_style) == state.battle_result;
            debug!(
                class = 8, p_style, battle_result = state.battle_result,
                "battle result condition -> {}", result
            );
            result
        }

        // General possession: value=npcId; owned → npcId, missing → 0.
        9 => {
            let npc_id = cond.condition_value;
            let owned = state.follow_npc_ids.contains(&npc_id);
            let actual = if owned { npc_id } else { 0 };
            let result = compare_step(actual, cond.condition_ops, npc_id);
            debug!(
                class = 9, npc_id, owned, ops = cond.condition_ops,
                "general condition -> {}", result
            );
            result
        }

        // Dialogue choice: param=surfaceId, pStyle=choiceCode.
        10 => {
            let result =
                state.last_surface_id == param && state.last_choice_code == i32::from(p_style);
            debug!(
                class = 10, surface_id = param, code = p_style,
                last_surface = state.last_surface_id,
                last_code = state.last_choice_code,
                "dialog choice condition -> {}", result
            );
            result
        }

        // Scene event completion count; param=0 reads the current eveNo.
        12 => {
            let value = match cond.condition_parameter {
                0 => state.completed_eve_counts.get(&state.current_eve_no).copied().unwrap_or(0),
                _ => 0,
            };
            let result = compare_step(value, cond.condition_ops, cond.condition_value);
            debug!(
                class = 12, param = cond.condition_parameter,
                eve_no = state.current_eve_no, count = value,
                ops = cond.condition_ops, val = cond.condition_value,
                "event count condition -> {}", result
            );
            result
        }

        // RoleCount statistic counter; param=counter index.
        14 => {
            let actual = state.role_count_values.get(&param).copied().unwrap_or(0);
            let result = compare_step(actual, cond.condition_ops, cond.condition_value);
            debug!(
                class = 14, count_index = param, actual,
                ops = cond.condition_ops, val = cond.condition_value,
                "role count condition -> {}", result
            );
            result
        }

        other => {
            tracing::warn!(
                class = other, param, p_style,
                ops = cond.condition_ops, val = cond.condition_value,
                "unimplemented conditionClass -> false"
            );
            false
        }
    }
}
