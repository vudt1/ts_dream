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
use crate::eve::resolver::resolve_event;
use crate::eve::state::{EveStateBuilder, PlayerEventState, PlayerStateInputs};
use crate::server::dispatcher::OpcodeCtx;
use crate::server::session::Session;

/// PC/aLogin 0x1A (OP_MONEY_SYNC per mobile-table: S->C money sync) request
/// shapes proven by static payload analysis. The PC dialect carries
/// talk-selector payloads instead; these are deliberately not named with
/// mobile `mainKind` semantics: the selector meaning is still not proven
/// by a live PC capture.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PcTalkRequest {
    SelectorOnly(u8),
    SelectorU8 { selector: u8, value: u8 },
    SelectorU16 { selector: u8, value: u16 },
    SelectorU32 { selector: u8, value: u32 },
}

/// Parse the PC 0x1A body (`ctx.payload`, after opcode and sub byte) using only
/// the schemas present in `opcode_1A_full_payloads.md`.
#[must_use]
pub fn parse_pc_talk_request(payload: &[u8]) -> Option<PcTalkRequest> {
    let selector = *payload.first()?;
    match (selector, payload.len()) {
        (0x00..=0x0A, 1) => Some(PcTalkRequest::SelectorOnly(selector)),
        (0x09, 2) => Some(PcTalkRequest::SelectorU8 {
            selector,
            value: payload[1],
        }),
        (0x08 | 0x0A, 3) => Some(PcTalkRequest::SelectorU16 {
            selector,
            value: u16::from_le_bytes([payload[1], payload[2]]),
        }),
        (0x01 | 0x02 | 0x05 | 0x06, 5) => Some(PcTalkRequest::SelectorU32 {
            selector,
            value: u32::from_le_bytes([payload[1], payload[2], payload[3], payload[4]]),
        }),
        _ => None,
    }
}

/// PC 0x1A (OP_MONEY_SYNC) / Talk boundary. The selector/value parser is live and strict, while
/// Eve execution remains opt-in until a PC selector-to-trigger mapping,
/// result serializer, and event-session persistence contract are verified.
pub async fn handle_pc_talk(ctx: &mut OpcodeCtx<'_>) {
    let Some(request) = parse_pc_talk_request(ctx.payload) else {
        tracing::debug!(
            "reject malformed PC 0x1A payload: {} bytes",
            ctx.payload.len()
        );
        return;
    };
    if !eve_events_enabled() {
        return;
    }
    tracing::warn!(
        ?request,
        "PC 0x1A selector accepted at typed boundary; Eve execution is disabled until selector semantics are verified"
    );
}

static EVE_EVENTS_ENABLED: AtomicBool = AtomicBool::new(false);

/// Enable/disable live Eve-event resolution. Off by default (golden parity).
pub fn set_eve_events_enabled(on: bool) {
    EVE_EVENTS_ENABLED.store(on, Ordering::Relaxed);
}

/// Whether live Eve-event resolution is active.
pub fn eve_events_enabled() -> bool {
    EVE_EVENTS_ENABLED.load(Ordering::Relaxed)
}

/// Trigger kinds handled here: ClickNpc=1, ClickDoor=4, MeetDoor=8.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NpcTrigger {
    ClickNpc(i32),
    ClickDoor(i32),
    MeetDoor(i32),
}

impl NpcTrigger {
    fn kind(self) -> i32 {
        match self {
            NpcTrigger::ClickNpc(_) => 1,
            NpcTrigger::ClickDoor(_) => 4,
            NpcTrigger::MeetDoor(_) => 8,
        }
    }

    fn click_id(self) -> i32 {
        match self {
            NpcTrigger::ClickNpc(id) | NpcTrigger::ClickDoor(id) | NpcTrigger::MeetDoor(id) => id,
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
///   mark id itself — an explicit key-space assumption (wire mark 1..=300 vs
///   Bear's mission-id-keyed `QuestDont`). It stays inert until a condition
///   actually targets such a mission id, because nothing in the Eve flow
///   sets `quest_dont` today.
/// - **`mark_defs`** (missionId → bitId) stays empty (CP6 decision #6): no
///   consumer exists yet, and inventing a projection would make up a mapping
///   the data files do not define.
pub fn snapshot_state_with_context(
    session: &Session,
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
    // Surface/choice/battle context of the active or completed event session feeds
    // conditionClass=10 (dialog choice) and =8 (battle result); a session
    // without one keeps the "no dialogue seen yet" -1 / "no battle" 0
    // defaults (Checkpoint 4).
    let (last_surface_id, last_choice_code, battle_result) = active
        .or(session.current_event_session.as_ref())
        .map(|ev| (ev.last_surface_id, ev.last_choice_code, ev.battle_result))
        .unwrap_or((-1, -1, 0));
    let mut state = EveStateBuilder::build_player_state(&PlayerStateInputs {
        missions: &missions,
        raw_flags: &[],
        mark_defs: &std::collections::HashMap::new(),
        bag_slots: &bag_slots,
        equips: &equips,
        level: i32::from(session.level),
        reborn_count: i32::from(session.reborn),
        last_surface_id,
        last_choice_code,
        battle_result,
        completed_eve_counts: session.completed_eve_counts.clone(),
        follow_npc_ids: &[],
        inn_npc_ids: &[],
        cart_npc_ids: &[],
    });
    for &mark in session.quest_dont.iter() {
        state.mission_flags.insert(i32::from(mark), 1);
    }
    state
}

pub fn snapshot_state(session: &Session) -> PlayerEventState {
    snapshot_state_with_context(session, None)
}

/// Resolve one NPC/door interaction against the scene's event list.
///
/// Returns an activated [`EventSession`] (results queued at index 0) when an
/// event matched; `None` when the feature is off, the map has no Eve scene, or
/// no chain matched (caller falls back to legacy talk tables).
#[must_use]
pub fn resolve_npc_event(
    session: &Session,
    data: &GameData,
    trigger: NpcTrigger,
    rng: &mut DotNetRandom,
) -> Option<EventSession> {
    if !eve_events_enabled() {
        return None;
    }
    let scene = scene_for(data, session.map_id)?;
    let state = snapshot_state(session);
    activate(scene, i32::from(session.map_id), trigger, &state, rng)
}

fn activate(
    scene: &SceneEveData,
    map_id: i32,
    trigger: NpcTrigger,
    state: &PlayerEventState,
    rng: &mut DotNetRandom,
) -> Option<EventSession> {
    let click = trigger.click_id() as u16;
    let events: &[u8] = match trigger.kind() {
        1 => &scene.npcs.get(&click)?.events,
        4 | 8 => &scene.doors.get(&click)?.events,
        _ => return None,
    };
    for &eve_no in events {
        let Some(event_data) = scene.npc_events.get(&u16::from(eve_no)) else {
            continue;
        };
        if EveAutoChainEngine::should_skip_event(events, eve_no, event_data, state) {
            continue;
        }
        let Some(resolved) = resolve_event(event_data, state, &scene.group_datas, rng, None) else {
            continue;
        };
        return Some(EventSession {
            map_id,
            eve_no: i32::from(event_data.eve_no),
            npc_click_id: trigger.click_id(),
            trigger_kind: trigger.kind(),
            chain_depth: 0,
            results: resolved.results,
            current_index: 0,
            phase: crate::eve::auto_chain::EventPhase::Executing,
            last_surface_id: -1,
            last_choice_code: -1,
            battle_result: 0,
            matched_condition_no: resolved.matched_condition_no,
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
    let state = snapshot_state_with_context(session, Some(completed));
    EveAutoChainEngine::try_auto_chain(scene, completed, &state, rng)
}
