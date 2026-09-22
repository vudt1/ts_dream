//! Auto-chain engine: seamless continuation between NPC / door events.
//!
//! After an event completes, player state may have changed (quest steps,
//! flags, items). The engine re-evaluates the same NPC/door event list and
//! chains into a newly matching event. Same-eveNo chaining is allowed (needed
//! for tutorial / transition flows) but guarded by 4 anti-loop protections:
//!
//! 1. Same Condition Detection - identical matched chain = state unchanged.
//! 2. Re-Question Prevention - no Surface right after answering one.
//! 3. Re-Battle Prevention - no interactive result right after a battle.
//! 4. Duplicate Item Prevention - never hand out the same item twice.

use crate::battle::rng::DotNetRandom;
use crate::data::loaders::{NpcEventData, SceneEveData};
use crate::eve::resolver::resolve_event;
use crate::eve::state::PlayerEventState;
use tracing::{debug, info};

/// Lifecycle phase of an in-flight event session.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventPhase {
    /// Executing results in order.
    Executing,
    /// Waiting for the client Surface choice (C:020-009).
    AwaitingChoice,
    /// Waiting for the battle to finish.
    AwaitingBattle,
    /// Terminal state; completion recorded.
    Completed,
}

/// An in-flight event session (immutable by convention; callers replace it).
#[derive(Debug, Clone, PartialEq)]
pub struct EventSession {
    /// Scene that triggered the event.
    pub map_id: i32,
    /// Event number; 0 means fallback Talk.
    pub eve_no: i32,
    /// Click ID of the triggering NPC or door.
    pub npc_click_id: i32,
    /// Trigger kind (1=ClickNpc, 4=ClickDoor, 8=MeetDoor, ...).
    pub trigger_kind: i32,
    /// Auto-chain depth guard against infinite loops.
    pub chain_depth: i32,
    /// Result queue for this event.
    pub results: Vec<crate::data::loaders::EveResult>,
    /// Index of the next result to dispatch.
    pub current_index: usize,
    pub phase: EventPhase,
    /// Last displayed Surface ID (conditionClass=10).
    pub last_surface_id: i32,
    /// Last dialogue choice code (conditionClass=10, guard #2).
    pub last_choice_code: i32,
    /// Battle outcome 0=none 1=win 2=lose 3=flee (conditionClass=8, guard #3).
    pub battle_result: i32,
    /// Matched chain head conditionNo (guard #1 loop detection key).
    pub matched_condition_no: i32,
}

impl EventSession {
    /// Whether any result mutates server state (decides completion tracking).
    ///
    /// If a session included a battle and ended in defeat (`battle_result == 2`)
    /// or flight (`battle_result == 3`), the encounter was not completed successfully
    /// and must not be marked completed, allowing the player to retry the encounter.
    #[must_use]
    pub fn has_state_changing_results(&self) -> bool {
        if self.battle_result == 2 || self.battle_result == 3 {
            return false;
        }
        self.results
            .iter()
            .any(|r| r.result_type == 0 || (r.result_type == 3 && self.battle_result == 1))
    }
}

/// Outcome of an auto-chain attempt.
#[derive(Debug, Clone, PartialEq)]
pub enum AutoChainResult {
    /// A new event matched; the candidate session is ready for activation.
    Chained(EventSession),
    /// No event matched; the interaction ends here.
    NoMatch,
}

/// Auto-chain resolver walking scene event lists with loop protection.
///
/// Pure data engine: the caller supplies the rebuilt [`PlayerEventState`]
/// snapshot and owns session activation, keeping this unit-testable without
/// network or persistence layers.
pub struct EveAutoChainEngine;

impl EveAutoChainEngine {
    /// Whether a completed session should attempt an auto-chain at all.
    ///
    /// Requires: active trigger kind, non-zero eveNo (not fallback Talk) and
    /// depth below the cap.
    #[must_use]
    pub fn should_attempt_auto_chain(completed: &EventSession) -> bool {
        AUTO_CHAIN_TRIGGER_KINDS.contains(&completed.trigger_kind)
            && completed.eve_no != 0
            && completed.chain_depth < MAX_CHAIN_DEPTH
    }

    /// Re-evaluate the same NPC/door event list after state changed.
    ///
    /// For each candidate event (skipping finished or surpassed ones), the
    /// conditions are re-resolved. Same-eveNo candidates must pass all 4 loop
    /// guards before chaining. The returned [`EventSession`] carries depth+1
    /// and is ready for activation; no global state is touched here.
    ///
    /// Map consistency (player still on the triggering map) is enforced by
    /// the caller before invoking this method.
    #[must_use]
    pub fn try_auto_chain(
        scene: &SceneEveData,
        completed: &EventSession,
        state: &PlayerEventState,
        rng: &mut DotNetRandom,
    ) -> AutoChainResult {
        let Some(event_list) = event_list_for(scene, completed) else {
            return AutoChainResult::NoMatch;
        };

        let new_depth = completed.chain_depth + 1;

        for &eve_no in event_list {
            let Some(event_data) = scene.npc_events.get(&u16::from(eve_no)) else {
                continue;
            };
            if Self::should_skip_event(event_list, eve_no, event_data, state) {
                debug!(eve_no, "auto-chain skip: completed or progress surpassed");
                continue;
            }

            let Some(resolved) = resolve_event(event_data, state, &scene.group_datas, rng, None)
            else {
                continue;
            };

            let candidate = EventSession {
                map_id: completed.map_id,
                eve_no: i32::from(event_data.eve_no),
                npc_click_id: completed.npc_click_id,
                trigger_kind: completed.trigger_kind,
                chain_depth: new_depth,
                results: resolved.results,
                current_index: 0,
                phase: EventPhase::Executing,
                last_surface_id: -1,
                last_choice_code: -1,
                battle_result: 0,
                matched_condition_no: resolved.matched_condition_no,
            };

            if i32::from(eve_no) == completed.eve_no {
                // Guard #1: same condition chain means unchanged player state.
                if Self::detect_same_chain(&candidate, completed) {
                    debug!(
                        eve_no,
                        cond_no = candidate.matched_condition_no,
                        "auto-chain same chain detected; terminating"
                    );
                    return AutoChainResult::NoMatch;
                }
                // Guard #2: question asked again right after answering.
                if Self::detect_re_question(&candidate, completed) {
                    debug!(eve_no, "auto-chain surface re-question; skipping");
                    continue;
                }
                // Guard #3: battle/interaction repeated right after battle.
                if Self::detect_re_battle(&candidate, completed) {
                    debug!(eve_no, "auto-chain re-battle; skipping");
                    continue;
                }
                // Guard #4: same item handed out twice.
                if Self::detect_duplicate_items(&candidate, completed) {
                    info!(eve_no, "auto-chain duplicate item; skipping");
                    continue;
                }
            }

            return AutoChainResult::Chained(candidate);
        }

        debug!(depth = new_depth, "auto-chain: no match");
        AutoChainResult::NoMatch
    }

    /// Guard #1: new event matched the exact same AND chain as the completed
    /// one. Deterministic via `matchedConditionNo` (immune to GroupData
    /// randomness). `conditionNo=0` is legal and still counts as equal.
    #[must_use]
    pub fn detect_same_chain(new_session: &EventSession, completed: &EventSession) -> bool {
        new_session.matched_condition_no == completed.matched_condition_no
    }

    /// Guard #2: a Surface appears again although the player just answered.
    #[must_use]
    pub fn detect_re_question(new_session: &EventSession, completed: &EventSession) -> bool {
        completed.last_choice_code != -1 && new_session.results.iter().any(|r| r.result_type == 6)
    }

    /// Guard #3: an interactive result appears although the player just
    /// fought a battle.
    #[must_use]
    pub fn detect_re_battle(new_session: &EventSession, completed: &EventSession) -> bool {
        completed.battle_result != 0
            && new_session
                .results
                .iter()
                .any(|r| r.result_type == 6 || r.result_type == 3)
    }

    /// Guard #4: the new queue hands out an item the completed one already
    /// granted (`resultClass=1 pStyle=1`).
    #[must_use]
    pub fn detect_duplicate_items(new_session: &EventSession, completed: &EventSession) -> bool {
        let give_items =
            |results: &[crate::data::loaders::EveResult]| -> std::collections::HashSet<u16> {
                results
                    .iter()
                    .filter(|r| r.result_class == 1 && r.parameter_style == 1)
                    .map(|r| r.parameter)
                    .collect()
            };
        let completed_give = give_items(&completed.results);
        if completed_give.is_empty() {
            return false;
        }
        !give_items(&new_session.results).is_disjoint(&completed_give)
    }

    /// Progress watermark: whether an event should be skipped entirely.
    ///
    /// Rule 1: already completed and no cls=12 self-managed count condition.
    /// Rule 2: not yet completed but a later sibling event already is.
    #[must_use]
    pub fn should_skip_event(
        events: &[u8],
        eve_no: u8,
        event_data: &NpcEventData,
        state: &PlayerEventState,
    ) -> bool {
        let completed_count = state
            .completed_eve_counts
            .get(&(i32::from(eve_no)))
            .copied()
            .unwrap_or(0);
        let has_completion_condition = event_data
            .conditions
            .iter()
            .any(|c| c.condition_class == 12);
        if completed_count > 0 && !has_completion_condition {
            return true;
        }
        if completed_count == 0 && has_later_completed_event(events, eve_no, state) {
            return true;
        }
        false
    }
}

/// Maximum auto-chain depth before giving up.
pub const MAX_CHAIN_DEPTH: i32 = 10;

/// Trigger kinds eligible for auto-chain: ClickNpc=1, ClickDoor=4, MeetDoor=8.
const AUTO_CHAIN_TRIGGER_KINDS: [i32; 3] = [1, 4, 8];

fn has_later_completed_event(events: &[u8], current_eve_no: u8, state: &PlayerEventState) -> bool {
    let mut found_current = false;
    for &e in events {
        if e == current_eve_no {
            found_current = true;
            continue;
        }
        if found_current
            && state
                .completed_eve_counts
                .get(&(i32::from(e)))
                .copied()
                .unwrap_or(0)
                > 0
        {
            return true;
        }
    }
    false
}

fn event_list_for<'a>(scene: &'a SceneEveData, session: &EventSession) -> Option<&'a [u8]> {
    match session.trigger_kind {
        1 => Some(
            scene
                .npcs
                .get(&(session.npc_click_id as u16))?
                .events
                .as_slice(),
        ),
        4 | 8 => Some(
            scene
                .doors
                .get(&(session.npc_click_id as u16))?
                .events
                .as_slice(),
        ),
        _ => None,
    }
}
