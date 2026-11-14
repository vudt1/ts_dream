//! NPC event handler: Level-2 bridge between the talk flow and the Eve script
//! engine (tickets 04/05).
//!
//! The engine itself is pure data ([`EveAutoChainEngine`], `resolve_event`);
//! this module owns the session-facing half:
//! - snapshot a [`Session`] into a [`PlayerEventState`],
//! - resolve the event list of the clicked NPC / door on the player's map,
//! - activate the resulting [`EventSession`] (result dispatch stays with the
//!   legacy quest/talk result executors),
//! - and continue finished events via [`EveAutoChainEngine::try_auto_chain`]
//!   with the 4-layer loop protection.
//!
//! Activation is gated behind [`set_eve_events_enabled`]: off by default so
//! wire parity with the golden captures is preserved until the operator flips
//! the feature on (`TS_EVE_EVENTS=1` at boot).

use std::sync::atomic::{AtomicBool, Ordering};

use crate::battle::rng::DotNetRandom;
use crate::data::loader::GameData;
use crate::data::loaders::SceneEveData;
use crate::eve::auto_chain::{AutoChainResult, EveAutoChainEngine, EventSession};
use crate::eve::planner::plan_chain;
use crate::eve::resolver::ChainFilter;
use crate::eve::state::{EveStateBuilder, PlayerEventState, PlayerStateInputs};
use crate::server::session::Session;

static EVE_EVENTS_ENABLED: AtomicBool = AtomicBool::new(true);

/// Enable/disable live Eve-event resolution. Enabled by default in production.
pub fn set_eve_events_enabled(on: bool) {
    EVE_EVENTS_ENABLED.store(on, Ordering::Relaxed);
}

/// Whether live Eve-event resolution is active.
pub fn eve_events_enabled() -> bool {
    EVE_EVENTS_ENABLED.load(Ordering::Relaxed)
}

/// Trigger kinds handled here: ClickNpc=1, ClickDoor=4, MeetDoor=8, WalkStep=16.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NpcTrigger {
    ClickNpc(i32),
    ClickDoor(i32),
    MeetDoor(i32),
    WalkStep { x: u16, y: u16 },
    WalkStepEncounter(i32),
}

impl NpcTrigger {
    fn kind(self) -> crate::eve::auto_chain::EventTriggerKind {
        use crate::eve::auto_chain::EventTriggerKind;
        match self {
            NpcTrigger::ClickNpc(_) => EventTriggerKind::ClickNpc,
            NpcTrigger::ClickDoor(_) => EventTriggerKind::ClickDoor,
            NpcTrigger::MeetDoor(_) => EventTriggerKind::MeetDoor,
            NpcTrigger::WalkStep { .. } | NpcTrigger::WalkStepEncounter(_) => {
                EventTriggerKind::WalkStep
            }
        }
    }

    fn click_id(self) -> i32 {
        match self {
            NpcTrigger::ClickNpc(id)
            | NpcTrigger::ClickDoor(id)
            | NpcTrigger::MeetDoor(id)
            | NpcTrigger::WalkStepEncounter(id) => id,
            NpcTrigger::WalkStep { .. } => 0,
        }
    }
}

fn scene_for(data: &GameData, map_id: u16) -> Option<&SceneEveData> {
    data.scene_eve_data.get(&u32::from(map_id))
}

/// Snapshot a session into the pure engine state.
///
/// CP6 wiring:
/// - **Missions** come from the quest log (`quest_tasks`: quest id → stored
///   `mark_step`), so `conditionClass=2` reads the step the class-2 action
///   executor writes.
/// - **`battle_result`** is the active event session's outcome (0 when no
///   session is in flight), feeding `conditionClass=8`.
/// - **Completion counts** are the session's `completed_eve_counts`
///   (incremented by `finish_event_session`), feeding `conditionClass=12`.
/// - **Quest-dont marks** are projected into `mission_flags` keyed by the
///   mark id itself (`Mark.dat` bit-id space `1..=2400` vs mission-id keys). It stays inert until a condition
///   actually targets such a mission id, because nothing in the Eve flow
///   sets `quest_dont` today.
/// - **`mark_defs`** (missionId → bitId) stays empty (CP6 decision #6): no
///   consumer exists yet, and inventing a projection would make up a mapping
///   the data files do not define.
pub fn snapshot_state_with_data(
    session: &Session,
    data: Option<&GameData>,
    active: Option<&EventSession>,
) -> PlayerEventState {
    let bag_slots: Vec<(i32, i32)> = session
        .homdo
        .iter()
        .filter(|i| i.id > 0)
        .map(|i| (i32::from(i.id), i32::from(i.count)))
        .collect();
    let equips: Vec<i32> = session
        .trangbi
        .iter()
        .filter(|i| i.id > 0)
        .map(|i| i32::from(i.id))
        .collect();
    // Quest log rows double as the Eve mission store: quest id → step.
    let missions: Vec<(i32, i32)> = session
        .quest_tasks
        .iter()
        .map(|(&quest_id, &(_slot, step))| (i32::from(quest_id), i32::from(step)))
        .collect();
    let mut follow_npc_ids = Vec::new();
    let mut inn_npc_ids = Vec::new();
    let mut cart_npc_ids = Vec::new();
    for pet in &session.pets {
        if pet.id == 0 {
            continue;
        }
        let npc_id = i32::from(pet.id);
        match pet.effective_storage_type() {
            crate::db::modern::model::PetStorageType::Cart => cart_npc_ids.push(npc_id),
            crate::db::modern::model::PetStorageType::Hotel => inn_npc_ids.push(npc_id),
            crate::db::modern::model::PetStorageType::Carried => follow_npc_ids.push(npc_id),
            crate::db::modern::model::PetStorageType::Warehouse => {}
        }
    }
    let role_count_values: Vec<(i32, i32)> = session
        .role_count_values
        .iter()
        .map(|(&idx, &val)| (idx, val))
        .collect();
    let bit_flags: Vec<u32> = session.bit_flags.iter().copied().collect();
    let raw_flags: Vec<(i32, i32)> = session
        .quest_dont
        .iter()
        .map(|&bit_id| (i32::from(bit_id), 1))
        .collect();
    let mark_defs: std::collections::HashMap<i32, i32> = data
        .map(|d| {
            d.quest_marks
                .iter()
                .map(|(&qid, &bit_id)| (i32::from(qid), i32::from(bit_id)))
                .collect()
        })
        .unwrap_or_default();
    // Surface/choice/battle context of the active or completed event session feeds
    // conditionClass=10 (dialog choice) and =8 (battle result); a session
    // without one keeps the "no dialogue seen yet" -1 / "no battle" 0
    // defaults (Checkpoint 4).
    let (last_surface_id, last_choice_code, battle_result) = active
        .or(session.current_event_session.as_ref())
        .map(|ev| (ev.last_surface_id, ev.last_choice_code, ev.battle_result))
        .unwrap_or((-1, -1, 0));
    let target_scene = active
        .or(session.current_event_session.as_ref())
        .and_then(|ev| u16::try_from(ev.map_id).ok())
        .unwrap_or(session.map_id);
    let scene_counts: std::collections::HashMap<i32, i32> = session
        .completed_eve_counts
        .iter()
        .filter_map(|(&(scene_id, eve_no), &count)| {
            (scene_id == target_scene).then_some((eve_no, count))
        })
        .collect();
    let mut state = EveStateBuilder::build_player_state(&PlayerStateInputs {
        missions: &missions,
        raw_flags: &raw_flags,
        mark_defs: &mark_defs,
        bag_slots: &bag_slots,
        equips: &equips,
        level: i32::from(session.level),
        reborn_count: i32::from(session.reborn),
        last_surface_id,
        last_choice_code,
        battle_result,
        completed_eve_counts: scene_counts,
        follow_npc_ids: &follow_npc_ids,
        inn_npc_ids: &inn_npc_ids,
        cart_npc_ids: &cart_npc_ids,
        role_count_values: &role_count_values,
        bit_flags: &bit_flags,
    });
    if data.is_none() {
        for &mark in session.quest_dont.iter() {
            state.mission_flags.insert(i32::from(mark), 1);
        }
    }
    state
}

pub fn snapshot_state_with_context(
    session: &Session,
    active: Option<&EventSession>,
) -> PlayerEventState {
    snapshot_state_with_data(session, None, active)
}

pub fn snapshot_state(session: &Session) -> PlayerEventState {
    snapshot_state_with_data(session, None, None)
}

/// Resolve one NPC/door interaction against the scene's event list, optionally
/// restricting matching chains via `chain_filter` (e.g. `DialogChoice` on
/// `0x14 09`).
///
/// Returns an activated [`EventSession`] (results queued at index 0) when an
/// event matched; `None` when the feature is off, the map has no Eve scene, or
/// no chain matched (caller falls back to legacy talk tables).
#[must_use]
pub fn resolve_npc_event_with_filter(
    session: &Session,
    data: &GameData,
    trigger: NpcTrigger,
    rng: &mut DotNetRandom,
    chain_filter: Option<ChainFilter<'_>>,
) -> Option<EventSession> {
    let is_dungeon_event_map = matches!(session.map_id, 12911..=12928 | 49960..=49969);
    if !eve_events_enabled() && !is_dungeon_event_map {
        return None;
    }
    let scene = scene_for(data, session.map_id)?;
    let state = snapshot_state_with_data(session, Some(data), None);
    activate(scene, i32::from(session.map_id), trigger, &state, rng, chain_filter)
}

/// Resolve one NPC/door interaction against the scene's event list.
#[must_use]
pub fn resolve_npc_event(
    session: &Session,
    data: &GameData,
    trigger: NpcTrigger,
    rng: &mut DotNetRandom,
) -> Option<EventSession> {
    resolve_npc_event_with_filter(session, data, trigger, rng, None)
}

fn activate(
    scene: &SceneEveData,
    map_id: i32,
    trigger: NpcTrigger,
    state: &PlayerEventState,
    rng: &mut DotNetRandom,
    chain_filter: Option<ChainFilter<'_>>,
) -> Option<EventSession> {
    if let NpcTrigger::WalkStep { x, y } = trigger {
        for enc in scene.encounters.iter().filter(|enc| enc.contains(x, y)) {
            for &eve_no in &enc.events {
                let Some(event_data) = scene.npc_events.get(&u16::from(eve_no)) else {
                    continue;
                };
                if EveAutoChainEngine::should_skip_event(&enc.events, eve_no, event_data, state) {
                    continue;
                }
                let Some(planned) =
                    plan_chain(event_data, state, &scene.group_datas, rng, chain_filter)
                else {
                    continue;
                };
                // Pure random-encounter battles (all results rt=3) are governed by
                // the step-counter in `try_trigger_random_encounter`, not WalkStep.
                if planned.results.iter().all(|r| r.result_type == 3) {
                    continue;
                }
                return Some(EventSession {
                    map_id,
                    eve_no: i32::from(event_data.eve_no),
                    npc_click_id: i32::from(enc.id),
                    trigger_kind: trigger.kind(),
                    chain_depth: 0,
                    results: planned.results,
                    current_index: 0,
                    phase: crate::eve::auto_chain::EventPhase::Executing,
                    last_surface_id: -1,
                    last_choice_code: -1,
                    battle_result: 0,
                    matched_condition_no: planned.matched_condition_no,
                });
            }
        }
        return None;
    }

    let click = trigger.click_id() as u16;
    let indexed_events: Vec<u8> = match trigger {
        NpcTrigger::ClickNpc(_) => scene
            .npc_events_by_npc
            .get(&click)
            .map(|v| v.iter().filter_map(|&e| u8::try_from(e).ok()).collect())
            .unwrap_or_default(),
        NpcTrigger::ClickDoor(_) | NpcTrigger::MeetDoor(_) => scene
            .door_events_by_door
            .get(&click)
            .map(|v| v.iter().filter_map(|&e| u8::try_from(e).ok()).collect())
            .unwrap_or_default(),
        NpcTrigger::WalkStepEncounter(_) => scene
            .encounter_events_by_id
            .get(&click)
            .map(|v| v.iter().filter_map(|&e| u8::try_from(e).ok()).collect())
            .unwrap_or_default(),
        NpcTrigger::WalkStep { .. } => Vec::new(),
    };
    let fallback_events: &[u8] = if indexed_events.is_empty() {
        match trigger {
            NpcTrigger::ClickNpc(_) => &scene.npcs.get(&click)?.events,
            NpcTrigger::ClickDoor(_) | NpcTrigger::MeetDoor(_) => &scene.doors.get(&click)?.events,
            NpcTrigger::WalkStepEncounter(_) => scene
                .encounters
                .iter()
                .find(|enc| enc.id == click)
                .map(|enc| enc.events.as_slice())?,
            NpcTrigger::WalkStep { .. } => return None,
        }
    } else {
        indexed_events.as_slice()
    };
    for &eve_no in fallback_events {
        let Some(event_data) = scene.npc_events.get(&u16::from(eve_no)) else {
            continue;
        };
        if EveAutoChainEngine::should_skip_event(fallback_events, eve_no, event_data, state) {
            continue;
        }
        let Some(planned) =
            plan_chain(event_data, state, &scene.group_datas, rng, chain_filter)
        else {
            continue;
        };
        return Some(EventSession {
            map_id,
            eve_no: i32::from(event_data.eve_no),
            npc_click_id: trigger.click_id(),
            trigger_kind: trigger.kind(),
            chain_depth: 0,
            results: planned.results,
            current_index: 0,
            phase: crate::eve::auto_chain::EventPhase::Executing,
            last_surface_id: -1,
            last_choice_code: -1,
            battle_result: 0,
            matched_condition_no: planned.matched_condition_no,
        });
    }
    None
}

/// Auto-chain continuation after a completed event session. Map consistency
/// (the player is still on the triggering map) is checked here.
#[must_use]
pub fn auto_chain_after(
    completed: &EventSession,
    session: &Session,
    data: &GameData,
    rng: &mut DotNetRandom,
) -> AutoChainResult {
    if !EveAutoChainEngine::should_attempt_auto_chain(completed) {
        return AutoChainResult::NoMatch;
    }
    let Some(scene) = scene_for(data, session.map_id) else {
        return AutoChainResult::NoMatch;
    };
    if i32::from(session.map_id) != completed.map_id {
        return AutoChainResult::NoMatch;
    }
    let state = snapshot_state_with_data(session, Some(data), Some(completed));
    EveAutoChainEngine::try_auto_chain(scene, completed, &state, rng)
}
