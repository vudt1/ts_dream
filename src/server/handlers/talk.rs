//! NpcEvent handler (Opcode 0x14): H1 start talk, H6 menus, H4 end talk, H8 warp talk, H9 select menu.
//!
//! Identity rules (ticket 18 review):
//! - **Map Object ID** (`map_object_id` = `idtalking`): the on-map instance
//!   from the H1 request (LE16). It keys `Data_Talks`/quests and is embedded
//!   hex-encoded in the special NPC packets.
//! - **NPC Template ID** (`npc_id`): resolved `(map, map_object_id) →
//!   NpcOnMap.NpcId`. The H1/H6 **special** branches (banker/inn/`16012`) are
//!   selected on this id, never on the object id.
//! - A **Talk Context** tracks `{talk_type, map_object_id, talk_count, select_menu}`.

use crate::battle::rng::DotNetRandom;
use crate::data::loader::GameData;
use crate::data::loaders::EveResult;
use crate::eve::auto_chain::{AutoChainResult, EventPhase};
use crate::protocol::codecs::npc_talk::{NpcTalkCodec, TalkLockMode};
use crate::protocol::encoder;
use crate::server::dispatcher::{HandleOutcome, OpcodeCtx};
use crate::server::handlers::npc_event::{self, NpcTrigger};
use crate::server::handlers::stats::build_stat_update;
use crate::db::pool::DbPool;
use crate::server::session::Conn;

/// Reset the legacy talk-context fields (`typetalk`, menu, warp confirm)
/// plus the active event session — the packet-less half of [`end_talk`],
/// shared with the CP4 event finish path (whose frames were already sent).
fn reset_talk_context(conn: &mut Conn) {
    conn.session.idtalking = 0;
    conn.session.select_menu = 0;
    conn.session.talk_count = 0;
    conn.session.warp_finish = false;
    conn.session.current_event_session = None;
}

/// EndTalk packet + reset the whole talk context.
pub fn end_talk(conn: &mut Conn, out: &mut HandleOutcome) {
    // When an Eve event session is active the client expects the actor-unlock
    // frame (`0x14 Sub 0x2C`, mode 0x02) before the close-dialog frame
    // (Bear `processStep` end-of-chain order). Legacy talk paths keep the
    // bare `F44402001408` for golden parity.
    if conn.session.current_event_session.is_some() {
        let char_id = conn.session.id as u32;
        out.send(NpcTalkCodec::build_talk_lock_hex(char_id, TalkLockMode::Unlock));
    }
    out.send("F44402001408");
    reset_talk_context(conn);
}

/// Wall-clock millisecond seed for the Eve RNG (shared by the talk-start
/// bridge and the Checkpoint 4 continue / menu continuations).
fn eve_rng_seed() -> i32 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i32)
        .unwrap_or(12345)
}

/// Split a dialog hex string on `F444` and emit each fragment 500 ms apart.
/// Frame order is preserved.
pub fn talk_messages(conn: &mut Conn, talk_string: &str, out: &mut HandleOutcome) {
    for part in talk_string.split("F444") {
        if !part.is_empty() {
            let frame = format!("F444{part}");
            if frame == "F44402001408" {
                conn.session.select_menu = 40;
            }
            out.send_delayed(frame, 500);
        }
    }
}

/// Dispatch Opcode 0x14 — NpcEvent (NPC talk / gate / scene-script).
pub async fn handle_talk(ctx: &mut OpcodeCtx<'_>) {
    let conn = &mut ctx.conn;
    let out = &mut ctx.out;
    let data = ctx.data;
    let pool = ctx.env.pool;
    let (sub, payload) = (ctx.sub, ctx.payload);
    match sub {
        1 => handle_talk_start(conn, payload, data, out),
        4 => end_talk(conn, out),
        6 => handle_talk_continue(conn, payload, data, pool, out).await,
        8 => handle_talk_warp(conn, payload, data, &ctx.env, out).await,
        9 => handle_talk_select_menu(conn, payload, data, out),
        _ => end_talk(conn, out),
    }
}

/// H1 identity + distance gate.
///
/// The `(map, object)` row MUST exist (its NPC template id drives the special
/// branches and its position the ±150 distance test); a talk to a missing
/// on-map instance is rejected with EndTalk (ticket 18 review: "reject
/// missing/out-of-range before any packet").
fn resolve_npc(data: &GameData, conn: &Conn, map_object_id: i32) -> Option<(i32, bool)> {
    if let Some(npc) = data
        .npc_on_map
        .iter()
        .find(|n| n.map_id == i64::from(conn.session.map_id) && n.id == i64::from(map_object_id))
    {
        let dx = i64::from(conn.session.map_x) - npc.x;
        let dy = i64::from(conn.session.map_y) - npc.y;
        let in_range = (-150..=150).contains(&dx) && (-150..=150).contains(&dy);
        return Some((npc.npc_id as i32, in_range));
    }
    if let Some(scene) = data.scene_eve_data.get(&u32::from(conn.session.map_id)) {
        if let Some(npc) = scene.npcs.get(&(map_object_id as u16)) {
            let dx = i64::from(conn.session.map_x) - i64::from(npc.x);
            let dy = i64::from(conn.session.map_y) - i64::from(npc.y);
            let in_range = (-150..=150).contains(&dx) && (-150..=150).contains(&dy);
            return Some((i32::from(npc.npc_id), in_range));
        }
    }
    None
}

/// Central Checkpoint 4 walker: dispatch the active event session's result
/// queue from `current_index` until the session needs client input or runs
/// out of results.
///
/// Queue-index convention: `current_index` points at the **last dispatched**
/// result (talk-start and talk-continue leave it sitting on the delivered
/// Talk/Surface); Action results advance it inline while walking. Callers
/// re-entering after a delivered result (Sub 6 / Sub 9) advance it first.
///
/// - Talk (`result_type == 1`): emit the step frame and stop — the next
///   `0x14 Sub 0x06` continues.
/// - Action (`0`): apply the state change and keep walking (non-blocking).
/// - Surface (`6`): record `last_surface_id`, switch to
///   [`EventPhase::AwaitingChoice`], emit the frame and stop — the
///   `0x14 Sub 0x09` choice answers it.
/// - Door (`2`) / Battle (`3`): close the talk safely.
///   TODO(CP6): dispatch door warp / battle trigger instead of ending here.
/// - Exhausted queue: unlock + close-dialog, then auto-chain into the next
///   matching event or reset the talk context.
fn execute_event_step(conn: &mut Conn, data: &GameData, out: &mut HandleOutcome) {
    loop {
        // Clone the current result so the action executors below can borrow
        // `conn` mutably.
        let next = conn
            .session
            .current_event_session
            .as_ref()
            .and_then(|ev| ev.results.get(ev.current_index).cloned());
        let Some(result) = next else {
            if conn.session.current_event_session.is_some() {
                finish_event_session(conn, data, out);
            }
            return;
        };

        match result.result_type {
            // Talk: deliver the step and wait for the client's Sub 6.
            1 => {
                out.send(NpcTalkCodec::build_talk_step_hex(&result));
                return;
            }
            // Surface/menu: remember the surface id (conditionClass=10 key),
            // wait for the client's Sub 9 choice.
            6 => {
                if let Some(ev) = conn.session.current_event_session.as_mut() {
                    ev.last_surface_id = i32::from(result.result_mean_no);
                    ev.phase = EventPhase::AwaitingChoice;
                }
                out.send(NpcTalkCodec::build_talk_step_hex(&result));
                return;
            }
            // Action: apply non-blocking state changes, keep walking.
            0 => {
                execute_action_result(conn, data, out, &result);
                match conn.session.current_event_session.as_mut() {
                    Some(ev) => ev.current_index = ev.current_index.saturating_add(1),
                    // Defensive: action executors never clear the session.
                    None => return,
                }
            }
            // Door / Battle: end safely for now.
            // TODO(CP6): dispatch door warp / battle trigger here instead.
            2 | 3 => {
                tracing::debug!(
                    result_type = result.result_type,
                    result_no = result.result_no,
                    "Eve Door/Battle result reached; closing the talk safely"
                );
                end_talk(conn, out);
                return;
            }
            other => {
                tracing::debug!(other, "unhandled Eve result type; skipping");
                match conn.session.current_event_session.as_mut() {
                    Some(ev) => ev.current_index = ev.current_index.saturating_add(1),
                    None => return,
                }
            }
        }
    }
}

/// Close an exhausted event session: unlock the actor, emit the close-dialog
/// frame, then either auto-chain into the next matching event (re-opening the
/// dialog) or reset the whole talk context.
fn finish_event_session(conn: &mut Conn, data: &GameData, out: &mut HandleOutcome) {
    let char_id = conn.session.id as u32;
    out.send(NpcTalkCodec::build_talk_lock_hex(char_id, TalkLockMode::Unlock));
    out.send(NpcTalkCodec::build_end_talk_hex());

    let Some(completed) = conn.session.current_event_session.take() else {
        return;
    };
    let mut rng = DotNetRandom::new(eve_rng_seed());
    match npc_event::auto_chain_after(&completed, &conn.session, data, &mut rng) {
        AutoChainResult::Chained(new_ev) => {
            conn.session.current_event_session = Some(new_ev);
            out.send("F44402000602");
            out.send(NpcTalkCodec::build_talk_lock_hex(char_id, TalkLockMode::Lock));
            // Bounded recursion: EveAutoChainEngine::MAX_CHAIN_DEPTH caps the
            // chain depth, so a chained session cannot recurse forever.
            execute_event_step(conn, data, out);
        }
        AutoChainResult::NoMatch => {
            // Frames already sent above — reset only the context (the same
            // field set `end_talk` clears).
            reset_talk_context(conn);
        }
    }
}

/// Apply one non-blocking Action result (`result_type == 0`).
///
/// `parameter` carries the target id and `result_value` the signed delta
/// (both live item operations in `eve.emg` use `parameter_style = 0`, so the
/// delta's sign is the give/take discriminator). Classes without a proven
/// executor are skipped with a debug log — the walker keeps advancing, so an
/// unsupported action can never wedge the dialog.
fn execute_action_result(
    conn: &mut Conn,
    data: &GameData,
    out: &mut HandleOutcome,
    result: &EveResult,
) {
    match result.result_class {
        // Item: parameter=itemId, result_value=signed quantity.
        1 => {
            let qty = result.result_value;
            if qty < 0 {
                conn.session
                    .remove_homdo_item(result.parameter, qty.unsigned_abs());
            } else if qty > 0 {
                let count = u8::try_from(qty).unwrap_or(u8::MAX);
                let item = crate::server::inventory::from_template(data, result.parameter, count);
                conn.session.add_homdo_item(item);
            }
            if qty != 0 {
                // Sync the mutated bag to the client (legacy H6 gift flow).
                out.send(conn.session.dump_homdo());
            }
        }
        // Quest/mission flag (parameter=missionId, result_value=target step).
        2 => {
            // TODO(ticket 08): no Eve mission-flag store is wired yet —
            // Session.quest_steps is keyed by NPC object id (legacy H6) and
            // snapshot_state feeds missions as empty, so writing would land
            // in the wrong store. Skip honestly rather than invent semantics.
            tracing::debug!(
                mission = result.parameter,
                value = result.result_value,
                "Eve action quest-flag result skipped: no mission store wired"
            );
        }
        // Player attribute deltas (exp/gold selectors).
        7 => {
            // TODO(ticket 08/CP6): the parameter->exp/gold selector mapping
            // for PC result payloads is not proven and Session carries no
            // EXP field; skipping is safer than inventing a gold/EXP write.
            tracing::debug!(
                param = result.parameter,
                value = result.result_value,
                "Eve action player-attribute result skipped: selector unverified"
            );
        }
        other => {
            tracing::debug!(
                class = other,
                param = result.parameter,
                "Eve action result class has no executor; skipped"
            );
        }
    }
}

fn handle_talk_start(conn: &mut Conn, payload: &[u8], data: &GameData, out: &mut HandleOutcome) {
    if payload.len() < 2 {
        end_talk(conn, out);
        return;
    }
    let map_object_id = encoder::u16_le(payload[0], payload[1]) as i32;
    conn.session.idtalking = map_object_id;
    conn.session.talk_type = "NPC".to_string();
    conn.session.talk_count = 0;

    let Some((template_id, in_range)) = resolve_npc(data, conn, map_object_id) else {
        end_talk(conn, out); // missing on-map instance -> reject
        return;
    };
    conn.session.idnpctalking = template_id;

    // Special template ids embed the map object id (hex-encoded).
    // Distance gate applies before the special payload.
    if matches!(template_id, 16080 | 16004 | 16011 | 16015) {
        if !in_range {
            end_talk(conn, out);
            return;
        }
        out.send("F44402000602");
        out.send(format!(
            "F44411001401000000010603{:02X}0000000000000100",
            map_object_id
        ));
        return;
    }
    if matches!(template_id, 15002 | 16001 | 16016) {
        if !in_range {
            end_talk(conn, out);
            return;
        }
        out.send("F44402000602");
        out.send(format!(
            "F44411001401000000010603{:02X}0000000000000200",
            map_object_id
        ));
        return;
    }
    if template_id == 16012 {
        return;
    }

    // Generic dialog: the distance gate still applies.
    if !in_range {
        end_talk(conn, out);
        return;
    }
    // Eve Engine event bridge (ticket 04/05 / Checkpoint 2).
    let mut rng = DotNetRandom::new(eve_rng_seed());
    if let Some(event_session) = npc_event::resolve_npc_event(
        &conn.session,
        data,
        NpcTrigger::ClickNpc(map_object_id),
        &mut rng,
    ) {
        // Wire order (Bear `ClickkNpc`/`processStep`): open dialog frame,
        // then actor lock, then the first talk step when it is a Talk result.
        let char_id = conn.session.id as u32;
        let first_talk = event_session
            .results
            .first()
            .filter(|r| r.result_type == 1)
            .map(NpcTalkCodec::build_talk_step_hex);
        conn.session.current_event_session = Some(event_session);
        out.send("F44402000602");
        out.send(NpcTalkCodec::build_talk_lock_hex(char_id, TalkLockMode::Lock));
        match first_talk {
            Some(step) => out.send(step),
            // CP4: a non-Talk head result goes through the central walker —
            // Action results run to completion inline, a Surface awaits the
            // Sub 9 choice, and Door/Battle close the talk safely
            // (TODO(CP6): dispatch door warp / battle trigger).
            None => execute_event_step(conn, data, out),
        }
        return;
    }

    let key = crate::server::handlers::quest::quest_key(
        i64::from(conn.session.map_id),
        "NPC",
        i64::from(map_object_id),
        crate::server::handlers::quest::current_step(conn),
    );
    if let Some(talk) = data.talks.get(&key) {
        out.send("F44402000602");
        let sum: i64 = talk.teamdef.iter().sum();
        if talk.dialogs.is_empty() && sum > 0 {
            crate::server::handlers::quest::trigger_teamdef(conn, &talk.teamdef, out);
            return;
        }
        talk_messages(conn, &talk.dialogs, out);
    } else {
        out.send("F44402000602");
        out.send(format!(
            "F44411001401000000010103{:02X}000000000000C830",
            map_object_id
        ));
    }
}

async fn handle_talk_continue(
    conn: &mut Conn,
    _payload: &[u8],
    data: &GameData,
    _pool: Option<&DbPool>,
    out: &mut HandleOutcome,
) {
    // Eve engine path (Checkpoint 4): an active event session owns the
    // continue — the legacy H6 guards below must not run for it.
    if conn.session.current_event_session.is_some() {
        if let Some(ev) = conn.session.current_event_session.as_mut() {
            if ev.phase == EventPhase::AwaitingChoice {
                // The pending Surface must be answered with Sub 9; a stray
                // continue must not skip the menu result.
                tracing::debug!("ignoring talk-continue while awaiting Surface choice");
                return;
            }
            // The queue index sits on the result already delivered; advance
            // past it before walking the queue again (research CP4 §1.3).
            ev.current_index = ev.current_index.saturating_add(1);
        }
        execute_event_step(conn, data, out);
        return;
    }

    // H6 pre-dispatch guards.
    if conn.session.warp_finish {
        out.send("F44402000504");
        out.send("F44402001408");
        conn.session.warp_finish = false;
        conn.session.talk_count = 0;
        conn.session.idtalking = 0;
        return;
    }
    if conn.session.idtalking == 0 && conn.session.select_menu == 40 {
        end_talk(conn, out);
        return;
    }
    if conn.session.idtalking <= 0 {
        return;
    }

    let template = conn.session.idnpctalking;

    // Banker / Store NPCs (branch on template id).
    if matches!(template, 16080 | 16004 | 16011 | 16023) {
        match conn.session.select_menu {
            30 => {
                out.send("F44403001D0900");
                out.send(format!(
                    "F44406001D04{}",
                    encoder::le32(conn.session.bank_gold)
                ));
                out.send("F44402001D05");
                out.send("F44402001409");
            }
            31 => {
                out.send("F44402001D06");
                out.send("F44402001409");
            }
            40 => end_talk(conn, out),
            _ => end_talk(conn, out),
        }
        return;
    }

    // Inn / hotel NPCs.
    if matches!(template, 15002 | 16001 | 16016 | 15118) {
        match conn.session.select_menu {
            30 => out.send("F44411001401000000010603010000000000000100"),
            31 => {
                conn.session.hp = conn.session.hp_max;
                conn.session.sp = conn.session.sp_max;
                out.send(build_stat_update(0x19, conn.session.hp as i32));
                out.send(build_stat_update(0x1A, conn.session.sp as i32));
                end_talk(conn, out);
            }
            32 => out.send("F44411001401000000010603010000000000000100"),
            33 => {
                crate::server::handlers::quest::save_map(conn);
                let item = crate::server::inventory::from_template(data, 46016, 2);
                let _ = conn.session.add_homdo_item(item);
                out.send(conn.session.dump_homdo());
                end_talk(conn, out);
            }
            40 => end_talk(conn, out),
            _ => end_talk(conn, out),
        }
        return;
    }

    // NPC 16015 — inn + gift item x2.
    if template == 16015 {
        match conn.session.select_menu {
            30 => out.send("F44411001401000000010603010000000000000200"),
            31 => {
                conn.session.hp = conn.session.hp_max;
                conn.session.sp = conn.session.sp_max;
                out.send(build_stat_update(0x19, conn.session.hp as i32));
                out.send(build_stat_update(0x1A, conn.session.sp as i32));
                end_talk(conn, out);
            }
            32 => out.send("F44411001401000000010603010000000000000200"),
            33 => {
                crate::server::handlers::quest::save_map(conn);
                let item = crate::server::inventory::from_template(data, 46016, 2);
                let _ = conn.session.add_homdo_item(item);
                out.send(conn.session.dump_homdo());
                end_talk(conn, out);
            }
            40 => end_talk(conn, out),
            _ => end_talk(conn, out),
        }
        return;
    }

    if template == 16012 {
        return;
    }

    // Daily quest map 12711 owns the whole context (21 RNG draws).
    if conn.session.map_id == 12711 {
        crate::server::handlers::quest::generate_daily_quest(conn, data, out);
        return;
    }

    // Generic data-driven quest path.
    if !crate::server::handlers::quest::try_quest_h6(conn, data, out) {
        end_talk(conn, out);
    }
}

async fn handle_talk_warp(
    conn: &mut Conn,
    payload: &[u8],
    data: &GameData,
    env: &crate::server::dispatcher::ServerEnv<'_>,
    out: &mut HandleOutcome,
) {
    if payload.len() >= 2 {
        conn.session.idtalking = encoder::u16_le(payload[0], payload[1]) as i32;
    }
    crate::server::handlers::quest::handle_warp_confirm(conn, data, env, out).await;
}

fn handle_talk_select_menu(
    conn: &mut Conn,
    payload: &[u8],
    data: &GameData,
    out: &mut HandleOutcome,
) {
    let Some(&choice) = payload.first() else {
        return;
    };

    // Eve menu path (Checkpoint 4): answer a pending Surface choice.
    let awaiting_choice = conn
        .session
        .current_event_session
        .as_ref()
        .is_some_and(|ev| ev.phase == EventPhase::AwaitingChoice);
    if awaiting_choice {
        // ChoiceCode 0 = "nothing selected yet" (wire default) — ignore it.
        if choice == 0 {
            tracing::debug!("ignoring menu select with default ChoiceCode 0");
            return;
        }
        // Record the choice on the active session: `snapshot_state` reads it
        // back for the conditionClass=10 branch re-evaluation, and the queue
        // index skips the already-delivered Surface result.
        let trigger = conn
            .session
            .current_event_session
            .as_mut()
            .and_then(|ev| {
                ev.last_choice_code = i32::from(choice);
                ev.phase = EventPhase::Executing;
                ev.current_index = ev.current_index.saturating_add(1);
                match ev.trigger_kind {
                    1 => Some(NpcTrigger::ClickNpc(ev.npc_click_id)),
                    4 => Some(NpcTrigger::ClickDoor(ev.npc_click_id)),
                    8 => Some(NpcTrigger::MeetDoor(ev.npc_click_id)),
                    _ => None,
                }
            });
        conn.session.select_menu = i32::from(choice);

        let Some(trigger) = trigger else {
            tracing::debug!("unsupported Eve trigger kind for menu select; ending talk");
            end_talk(conn, out);
            return;
        };
        // Re-resolve the NPC's event list against the updated snapshot: the
        // conditionClass=10 chain matching (surfaceId, choiceCode) wins and
        // becomes the branch session (research CP4 §2.3).
        let mut rng = DotNetRandom::new(eve_rng_seed());
        match npc_event::resolve_npc_event(&conn.session, data, trigger, &mut rng) {
            Some(mut branch) => {
                // Carry the answered Surface context into the branch session:
                // later snapshot reads (and auto-chain guard #2, "no Surface
                // right after answering") must still see the choice.
                if let Some(prev) = conn.session.current_event_session.as_ref() {
                    branch.last_surface_id = prev.last_surface_id;
                    branch.last_choice_code = prev.last_choice_code;
                }
                // The dialog frame and actor lock stay in place — deliver the
                // first step of the matched branch directly.
                conn.session.current_event_session = Some(branch);
                execute_event_step(conn, data, out);
            }
            // No branch matches this choice: close safely (unlock + 1408).
            None => end_talk(conn, out),
        }
        return;
    }

    // Legacy H6 path: store the raw choice byte for the talk-continue routing.
    conn.session.select_menu = i32::from(choice);
}
