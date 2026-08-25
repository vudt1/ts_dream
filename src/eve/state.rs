//! Player event state snapshot and builders.
//!
//! [`PlayerEventState`] is an immutable value object consumed by the pure
//! condition evaluator — it never queries external systems. [`EveStateBuilder`]
//! aggregates raw inputs (missions, flags, bag, pets) into that snapshot.

use crate::data::loaders::{EveResult, SceneEveData};
use std::collections::{HashMap, HashSet};

/// Snapshot of every player fact the Eve condition evaluator may consult.
///
/// Mirrors Kotlin `PlayerEventState`. Field defaults follow the wire
/// semantics: no dialogue seen yet (`last_surface_id = -1`,
/// `last_choice_code = -1`) and no battle in flight (`battle_result = 0`,
/// `1=win 2=lose 3=flee`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlayerEventState {
    /// Mission ID → current step (`conditionClass=2` pStyle=1/2).
    pub mission_steps: HashMap<i32, i32>,
    /// Mission ID → completion flag value (`conditionClass=2` pStyle=3).
    pub mission_flags: HashMap<i32, i32>,
    /// Item ID → total quantity across bag slots (`conditionClass=1`).
    pub bag_items: HashMap<i32, i32>,
    /// Equipped item IDs (`conditionClass=7` param=3).
    pub equip_items: HashSet<i32>,
    /// Character level (`conditionClass=7` param=0).
    pub level: i32,
    /// Reborn count (`conditionClass=7` param=2).
    pub reborn_count: i32,
    /// Last displayed Surface ID (`conditionClass=10`).
    pub last_surface_id: i32,
    /// Last dialogue choice code (`conditionClass=10`).
    pub last_choice_code: i32,
    /// Battle result: 0=none 1=win 2=lose 3=flee (`conditionClass=8`).
    pub battle_result: i32,
    /// Completed event counts per eveNo for the current scene
    /// (`conditionClass=12` param=0 reads `current_eve_no`).
    pub completed_eve_counts: HashMap<i32, i32>,
    /// The eveNo currently being evaluated (`conditionClass=12` param=0).
    pub current_eve_no: i32,
    /// Follow-slot general NPC IDs (`conditionClass=9`, class=7 param=6 pStyle=1).
    pub follow_npc_ids: HashSet<i32>,
    /// Inn general NPC IDs (class=7 param=6 pStyle=3).
    pub inn_npc_ids: HashSet<i32>,
    /// Cart general NPC IDs (class=7 param=6 pStyle=2).
    pub cart_npc_ids: HashSet<i32>,
    /// RoleCount statistic counters (`conditionClass=14`; e.g. 100=capture,
    /// 101=death, 108=consecutive login).
    pub role_count_values: HashMap<i32, i32>,
}

impl Default for PlayerEventState {
    fn default() -> Self {
        Self {
            mission_steps: HashMap::new(),
            mission_flags: HashMap::new(),
            bag_items: HashMap::new(),
            equip_items: HashSet::new(),
            level: 0,
            reborn_count: 0,
            last_surface_id: -1,
            last_choice_code: -1,
            battle_result: 0,
            completed_eve_counts: HashMap::new(),
            current_eve_no: 0,
            follow_npc_ids: HashSet::new(),
            inn_npc_ids: HashSet::new(),
            cart_npc_ids: HashSet::new(),
            role_count_values: HashMap::new(),
        }
    }
}

/// Raw inputs aggregated by [`EveStateBuilder::build_player_state`].
///
/// Callers hand over references from Quest / Inventory / Pet systems; the
/// builder performs all merging so downstream tiers stay pure.
#[derive(Debug, Clone)]
pub struct PlayerStateInputs<'a> {
    /// `(missionId, step)` pairs of missions in progress.
    pub missions: &'a [(i32, i32)],
    /// `(bitIndex, bitValue)` completion flag records.
    pub raw_flags: &'a [(i32, i32)],
    /// Mission definition map `missionId -> bitId` used to project raw flags
    /// onto mission IDs.
    pub mark_defs: &'a HashMap<i32, i32>,
    /// `(itemId, quantity)` per occupied bag slot.
    pub bag_slots: &'a [(i32, i32)],
    /// Equipped item IDs.
    pub equips: &'a [i32],
    pub level: i32,
    pub reborn_count: i32,
    /// Session context carried over from an active event, if any.
    pub last_surface_id: i32,
    pub last_choice_code: i32,
    pub battle_result: i32,
    /// Completed event counts for this scene (DB load merged with memory).
    pub completed_eve_counts: HashMap<i32, i32>,
    /// Follow-slot general NPC IDs.
    pub follow_npc_ids: &'a [i32],
    /// Inn general NPC IDs.
    pub inn_npc_ids: &'a [i32],
    /// Cart general NPC IDs.
    pub cart_npc_ids: &'a [i32],
}

/// Builds player state snapshots and resolves fallback dialogue.
pub struct EveStateBuilder;

impl EveStateBuilder {
    /// Aggregate raw system inputs into a pure [`PlayerEventState`] snapshot.
    ///
    /// Completion flags are projected through `mark_defs`
    /// (`missionId -> bitId`); only positive flag values are recorded so a
    /// finished mission (step=0, flag=1) still satisfies `actual >= 1`.
    pub fn build_player_state(inputs: &PlayerStateInputs<'_>) -> PlayerEventState {
        let mut state = PlayerEventState {
            level: inputs.level,
            reborn_count: inputs.reborn_count,
            last_surface_id: inputs.last_surface_id,
            last_choice_code: inputs.last_choice_code,
            battle_result: inputs.battle_result,
            completed_eve_counts: inputs.completed_eve_counts.clone(),
            ..PlayerEventState::default()
        };

        for &(mission_id, step) in inputs.missions {
            state.mission_steps.insert(mission_id, step);
        }

        let mut flag_by_bit_index: HashMap<i32, i32> = HashMap::new();
        for &(bit_index, bit_value) in inputs.raw_flags {
            flag_by_bit_index.insert(bit_index, bit_value);
        }
        for (&mission_id, &bit_id) in inputs.mark_defs {
            if let Some(&flag_value) = flag_by_bit_index.get(&bit_id) {
                if flag_value > 0 {
                    state.mission_flags.insert(mission_id, flag_value);
                }
            }
        }

        for &(item_id, quantity) in inputs.bag_slots {
            *state.bag_items.entry(item_id).or_insert(0) += quantity;
        }

        state.equip_items = inputs.equips.iter().copied().collect();
        state.follow_npc_ids = inputs.follow_npc_ids.iter().copied().collect();
        state.inn_npc_ids = inputs.inn_npc_ids.iter().copied().collect();
        state.cart_npc_ids = inputs.cart_npc_ids.iter().copied().collect();

        state
    }

    /// Find the first Talk result among the given events as fallback dialogue.
    pub fn find_fallback_talk(scene: &SceneEveData, eve_nos: &[u16]) -> Option<EveResult> {
        for &eve_no in eve_nos {
            let event_data = scene.npc_events.get(&eve_no)?;
            for cond in &event_data.conditions {
                for result in &cond.results {
                    if result.result_type == 1 && result.result_mean_no > 0 {
                        return Some(result.clone());
                    }
                }
            }
        }
        None
    }

    /// Distance gate between player and target positions (200 px squared,
    /// looser than the client's own 130 px check). A player at origin is
    /// still initializing and bypasses validation.
    pub fn is_within_range(px: i32, py: i32, tx: i32, ty: i32) -> bool {
        if px == 0 && py == 0 {
            return true;
        }
        let dx = (px - tx) as i64;
        let dy = (py - ty) as i64;
        dx * dx + dy * dy <= MAX_INTERACT_DISTANCE_SQ
    }
}

const MAX_INTERACT_DISTANCE_SQ: i64 = 200 * 200;
