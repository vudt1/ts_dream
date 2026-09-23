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
use crate::battle::runner::Outcome;
use crate::data::loader::GameData;
use crate::data::loaders::EveResult;
use crate::eve::auto_chain::{AutoChainResult, EventPhase};
use crate::protocol::codecs::npc_talk::{NpcTalkCodec, TalkLockMode};
use crate::protocol::encoder;
use crate::server::dispatcher::{HandleOutcome, OpcodeCtx};
use crate::server::handlers::npc_event::{self, NpcTrigger};
use crate::server::handlers::quest_sync::{remove_quest_task, set_quest_dont, set_quest_task};
use crate::server::handlers::shops::gold_frame;
use crate::server::handlers::stats::build_stat_update;
use crate::db::pool::DbPool;
use crate::server::session::{Conn, Session};

/// Reset the legacy talk-context fields (`typetalk`, menu, warp confirm)
/// plus the active event session — the packet-less half of [`end_talk_session`],
/// shared with the CP4 event finish path (whose frames were already sent).
///
/// Session-shaped (not `Conn`-shaped) so the post-battle resume in
/// `battle::service::battle_ended` can call it with only a session guard.
fn reset_talk_context(session: &mut Session) {
    session.idtalking = 0;
    session.select_menu = 0;
    session.talk_count = 0;
    session.warp_finish = false;
    session.current_event_session = None;
}

/// EndTalk packet + reset the whole talk context, over a bare [`Session`].
///
/// When an Eve event session is active the client expects the actor-unlock
/// frame (`0x14 Sub 0x2C`, mode 0x02) before the close-dialog frame
/// (Bear `processStep` end-of-chain order). Legacy talk paths keep the
/// bare `F44402001408` for golden parity.
pub fn end_talk_session(session: &mut Session, out: &mut HandleOutcome) {
    if session.current_event_session.is_some() {
        let char_id = session.id as u32;
        out.send(NpcTalkCodec::build_talk_lock_hex(char_id, TalkLockMode::Unlock));
    }
    out.send("F44402001408");
    reset_talk_context(session);
}

/// [`end_talk_session`] over the connection wrapper (legacy call sites).
pub fn end_talk(conn: &mut Conn, out: &mut HandleOutcome) {
    end_talk_session(&mut conn.session, out);
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

/// Central Checkpoint 4/6 walker: dispatch the active event session's result
/// queue from `current_index` until the session needs client input or runs
/// out of results.
///
/// Queue-index convention: `current_index` points at the **last dispatched**
/// result (talk-start and talk-continue leave it sitting on the delivered
/// Talk/Surface); Action results advance it inline while walking. Callers
/// re-entering after a delivered result (Sub 6 / Sub 9 / post-battle resume)
/// advance it first.
///
/// - Talk (`result_type == 1`): emit the step frame and stop — the next
///   `0x14 Sub 0x06` continues.
/// - Action (`0`): apply the state change and keep walking (non-blocking).
/// - Surface (`6`): record `last_surface_id`, switch to
///   [`EventPhase::AwaitingChoice`], emit the frame and stop — the
///   `0x14 Sub 0x09` choice answers it.
/// - Door (`2`): warp to `data.warps[(source map, parameter)]` then
///   [`finish_event_session`]. A missing warp id skips (advance + continue).
/// - Battle (`3`): pre-check `scene.fight_datas[result_mean_no]`, park the
///   session in [`EventPhase::AwaitingBattle`] and hand `out.eve_battle` to the
///   connection loop, which starts the battle **after** this outcome's frames
///   flush (so talk frames are never reordered behind battle frames).
/// - Exhausted queue: unlock + close-dialog, then auto-chain into the next
///   matching event or reset the talk context.
///
/// Takes `&mut Session` (not `&mut Conn`) because the post-battle resume in
/// `battle::service::battle_ended` reaches it through a session guard only.
pub fn execute_event_step(session: &mut Session, data: &GameData, out: &mut HandleOutcome) {
    loop {
        // Clone the current result so the action executors below can borrow
        // `session` mutably.
        let next = session
            .current_event_session
            .as_ref()
            .and_then(|ev| ev.results.get(ev.current_index).cloned());
        let Some(result) = next else {
            if session.current_event_session.is_some() {
                finish_event_session(session, data, out);
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
                if let Some(ev) = session.current_event_session.as_mut() {
                    ev.last_surface_id = i32::from(result.result_mean_no);
                    ev.phase = EventPhase::AwaitingChoice;
                }
                out.send(NpcTalkCodec::build_talk_step_hex(&result));
                return;
            }
            // Action: apply non-blocking state changes, keep walking.
            0 => {
                execute_action_result(session, data, out, &result);
                match session.current_event_session.as_mut() {
                    Some(ev) => ev.current_index = ev.current_index.saturating_add(1),
                    // Defensive: action executors never clear the session.
                    None => return,
                }
            }
            // Door: relocate to the scripted destination, then finish.
            2 => {
                let warp_key = (i64::from(session.map_id), i64::from(result.parameter));
                let Some(warp) = data.warps.get(&warp_key) else {
                    tracing::debug!(
                        map_id = session.map_id,
                        warp_id = result.parameter,
                        "Eve Door result has no Warps.dat row; skipping"
                    );
                    match session.current_event_session.as_mut() {
                        Some(ev) => ev.current_index = ev.current_index.saturating_add(1),
                        None => return,
                    }
                    continue;
                };
                // Party followers cannot relocate on their own (legacy parity
                // with `handle_warp_confirm`): close the talk instead.
                if session.id_leader > 0 && session.id_leader != session.id {
                    tracing::debug!("Eve Door warp refused: member follows a party leader");
                    end_talk_session(session, out);
                    return;
                }
                crate::server::handlers::quest::perform_warp(session, warp, out);
                // Scoping decision (CP6 #11 & Fix Vấn đề 3): Door ends its event session —
                // the warp already moved the player off `completed.map_id`.
                // Do NOT call finish_event_session here: sending 14 08 prematurely clears
                // the screen fade (14 07) before the client finishes loading the map.
                // 14 08 will be sent upon map load confirmation (0x0C Sub 1) in system.rs.
                reset_talk_context(session);
                session.current_event_session = None;
                return;
            }
            // Battle: park the session; the connection loop starts the fight
            // once this outcome's frames have been flushed.
            3 => {
                let fight_id = result.result_mean_no;
                let scene = data.scene_eve_data.get(&u32::from(session.map_id));
                let has_fight = scene
                    .map(|s| s.fight_datas.contains_key(&fight_id))
                    .unwrap_or(false);
                if !has_fight {
                    tracing::debug!(
                        map_id = session.map_id,
                        fight_id,
                        "Eve Battle result has no FightData row; skipping"
                    );
                    match session.current_event_session.as_mut() {
                        Some(ev) => ev.current_index = ev.current_index.saturating_add(1),
                        None => return,
                    }
                    continue;
                }
                let diahinh = scene
                    .and_then(|s| s.scene_infos.get(&1))
                    .map(|info| i32::from(info.background_no))
                    .unwrap_or(112);
                if let Some(ev) = session.current_event_session.as_mut() {
                    ev.phase = EventPhase::AwaitingBattle;
                    // Index stays parked on the battle result; the resume path
                    // advances it by one before walking again (Talk parity).
                }
                out.eve_battle = Some((fight_id, diahinh));
                return;
            }
            other => {
                tracing::debug!(other, "unhandled Eve result type; skipping");
                match session.current_event_session.as_mut() {
                    Some(ev) => ev.current_index = ev.current_index.saturating_add(1),
                    None => return,
                }
            }
        }
    }
}

/// Close an exhausted event session: unlock the actor, emit the close-dialog
/// frame, record the completion count when the session actually mutated state,
/// then either auto-chain into the next matching event (re-opening the
/// dialog) or reset the whole talk context.
fn finish_event_session(session: &mut Session, data: &GameData, out: &mut HandleOutcome) {
    let char_id = session.id as u32;
    out.send(NpcTalkCodec::build_talk_lock_hex(char_id, TalkLockMode::Unlock));
    out.send(NpcTalkCodec::build_end_talk_hex());

    let Some(completed) = session.current_event_session.take() else {
        return;
    };
    // CP6 #7: a session counts as completed only when it carried an Action or
    // a Battle result — pure Talk/Surface chains must not bump the counter
    // (the auto-chain watermark `should_skip_event` keys off it).
    if completed.has_state_changing_results() {
        *session
            .completed_eve_counts
            .entry(completed.eve_no)
            .or_insert(0) += 1;
    }
    let mut rng = DotNetRandom::new(eve_rng_seed());
    match npc_event::auto_chain_after(&completed, session, data, &mut rng) {
        AutoChainResult::Chained(new_ev) => {
            session.current_event_session = Some(new_ev);
            out.send("F44402000602");
            out.send(NpcTalkCodec::build_talk_lock_hex(char_id, TalkLockMode::Lock));
            // Bounded recursion: EveAutoChainEngine::MAX_CHAIN_DEPTH caps the
            // chain depth, so a chained session cannot recurse forever.
            execute_event_step(session, data, out);
        }
        AutoChainResult::NoMatch => {
            // Frames already sent above — reset only the context (the same
            // field set `end_talk` clears).
            reset_talk_context(session);
        }
    }
}

/// CP6 decision #4 & Phase 1 Fix Vấn đề 2 — advance a parked Eve event session
/// (`phase == AwaitingBattle`) after its scripted battle ended.
///
/// Returns `(frames, next_battle)`:
/// - **Win / Lose / Flee** records `battle_result` (1=Win, 2=Lose, 3=Flee),
///   advances `current_index` past the delivered battle, sets
///   `phase = EventPhase::Executing`, and walks `execute_event_step`.
///   Subsequent actions (e.g. quest step back-up on loss/flee, loss dialogue,
///   or post-battle auto-chain) will execute properly.
/// - `next_battle` carries an `eve_battle` requested *by the resumed walk*
///   (a second scripted battle in the same session); the caller — the battle
///   service, once its `members` lock is free — starts it.
///
/// A session that is not parked returns empty output untouched. Kept here
/// rather than in `battle::service` because it drives this module's dialog
/// state machine (and so the battle layer only forwards the outcome).
pub fn resume_eve_after_battle(
    session: &mut Session,
    outcome: Outcome,
    data: &GameData,
) -> (Vec<String>, Option<(u16, i32)>) {
    let parked = matches!(
        session.current_event_session.as_ref(),
        Some(ev) if ev.phase == EventPhase::AwaitingBattle
    );
    if !parked {
        return (Vec::new(), None);
    }
    let battle_result: i32 = match outcome {
        Outcome::PlayerWin => 1,
        Outcome::PlayerLose => 2,
        Outcome::PlayerFled => 3,
        // Defensive: a battle still marked Running never reaches `battle_ended`.
        Outcome::Running => return (Vec::new(), None),
    };
    {
        let Some(ev) = session.current_event_session.as_mut() else {
            return (Vec::new(), None);
        };
        ev.battle_result = battle_result;
        ev.phase = EventPhase::Executing;
        // Index sits on the delivered battle result — advance first.
        ev.current_index = ev.current_index.saturating_add(1);
    }
    let mut out = HandleOutcome::default();
    execute_event_step(session, data, &mut out);
    let next_battle = out.eve_battle;
    let frames = out.outgoing.into_iter().map(|f| f.frame).collect();
    (frames, next_battle)
}

/// Apply one non-blocking Action result (`result_type == 0`).
///
/// `parameter` carries the target id and `result_value` the signed delta
/// (both live item operations in `eve.emg` use `parameter_style = 0`, so the
/// delta's sign is the give/take discriminator). Classes without a proven
/// executor are skipped with a debug log — the walker keeps advancing, so an
/// unsupported action can never wedge the dialog.
///
/// Wire semantics are decoded exactly like Bear's `PackagePayloadDecoder`
/// (`result_type` at `pos+3`, `result_class` at `pos+4`, `parameter` at
/// `pos+5`, `parameter_style` at `pos+7`, `result_value` at `pos+8`).
///
/// Takes `&mut Session` (not `&mut Conn`) so the post-battle resume in
/// `battle::service::battle_ended` can drive it through a session guard only.
fn execute_action_result(
    session: &mut Session,
    data: &GameData,
    out: &mut HandleOutcome,
    result: &EveResult,
) {
    match result.result_class {
        // Item: parameter=itemId, result_value=signed quantity.
        1 => {
            let qty = result.result_value;
            if qty < 0 {
                session.remove_homdo_item(result.parameter, qty.unsigned_abs());
            } else if qty > 0 {
                let count = u8::try_from(qty).unwrap_or(u8::MAX);
                let item = crate::server::inventory::from_template(data, result.parameter, count);
                session.add_homdo_item(item);
            }
            if qty != 0 {
                // Sync the mutated bag to the client (legacy H6 gift flow).
                out.send(session.dump_homdo());
            }
        }
        // Quest-log row (Bear `QuestSaveHandler`, CP6 #5): parameter=questId,
        // parameter_style=pStyle, result_value's low byte the step written at
        // `packageToSend[8]`.
        2 => {
            let quest_id = result.parameter;
            let p_style = result.parameter_style;
            let step = (result.result_value & 0xFF) as u8;

            if p_style == 3 && step == 0 {
                // pStyle 3 + step 0 = drop the quest from the log. The client
                // clears the shared quest row by id (`0x18 Sub 0x04`) — the
                // frame Bear's `removerTask` sends.
                remove_quest_task(session, out, quest_id);
                return;
            }
            // Bear only persists when the (pStyle, step) pair is in its save
            // set; anything else is a no-op with a debug trace.
            let save = (p_style == 1 && matches!(step, 1 | 2 | 3 | 10 | 30 | 50 | 70))
                || p_style == 4
                || (p_style == 2 && matches!(step, 1 | 9));
            if !save {
                tracing::debug!(
                    quest_id,
                    p_style,
                    step,
                    "Eve quest-save result outside Bear's save set; skipped"
                );
                return;
            }
            // `resBattle` equivalent: the active event session's battle
            // outcome (0=none 1=win 2=lose 3=flee).
            let battle_result = session
                .current_event_session
                .as_ref()
                .map(|ev| ev.battle_result)
                .unwrap_or(0);
            let next_step = match session.quest_tasks.get(&quest_id) {
                // New quest: the payload's step, floored at 1.
                None => step.max(1),
                // Existing quest: current + delta — except after a lost/fled
                // battle (Bear `resBattle` 2/3), where the step backs up one.
                Some((_slot, current)) => {
                    if matches!(battle_result, 2 | 3) {
                        current.saturating_sub(1)
                    } else {
                        current.saturating_add(step)
                    }
                }
            };
            set_quest_task(session, out, quest_id, next_step);
            // Task 3.2: If quest has a mark position > 0 in Mark.Dat, record quest_dont and emit frame 18 05.
            // Only award completion when the battle was not lost or fled.
            if !matches!(battle_result, 2 | 3) {
                if let Some(&mark) = data.quest_marks.get(&quest_id) {
                    if mark > 0 {
                        set_quest_dont(session, out, mark, 1);
                    }
                }
            }
        }
        // Gold / stat-point reward (Bear `GoldEffectHandler`, CP6 #10):
        // parameter_style=type, result_value=amount.
        5 => {
            let amount = result.result_value;
            match result.parameter_style {
                // Type 1: small amounts convert to stat points at 20:1,
                // anything >= 1000 is plain gold.
                1 if amount > 0 => {
                    if amount < 1000 {
                        let rewarded = u16::try_from(amount * 20).unwrap_or(u16::MAX);
                        // Bear caps its point pool at 1_000_000_000;
                        // `Session.point` is a `u16`, so it saturates there.
                        session.point = session.point.saturating_add(rewarded);
                        out.send(build_stat_update(0x26, i32::from(session.point)));
                    } else {
                        session.gold = session.gold.saturating_add(amount as u32);
                        out.send(gold_frame(session.gold));
                    }
                }
                // Type 2 (Bear `GoldEffectHandler`): deduct gold from player
                // (e.g. gate entry fees or penalties). Saturating to prevent underflow.
                2 => {
                    if amount > 0 {
                        session.gold = session.gold.saturating_sub(amount as u32);
                    }
                    out.send(gold_frame(session.gold));
                }
                other => {
                    tracing::debug!(
                        gold_type = other,
                        amount,
                        "Eve gold result type has no proven executor; skipped"
                    );
                }
            }
        }
        // Player-attribute effects (Bear `StatBonusAndBallEffectHandler` +
        // `GetSaveMap`, CP6 #10).
        7 => {
            // parameter_style 1 (Bear `GetSaveMap`): refresh the respawn point
            // from the current map.
            if result.parameter_style == 1 {
                crate::server::handlers::quest::save_map(session);
            }
            match result.parameter {
                // parameter 1 (Bear `GetSttBonus`): type = parameter_style,
                // points = result_value's low 16 bits.
                1 => match result.parameter_style {
                    2 => {
                        let gained = (result.result_value & 0xFFFF) as u16;
                        session.skill_point = session.skill_point.saturating_add(gained);
                        out.send(build_stat_update(0x25, i32::from(session.skill_point)));
                        out.send(NpcTalkCodec::build_skill_point_toast_hex(gained.min(255) as u8));
                    }
                    3 => {
                        let gained = (result.result_value & 0xFFFF) as u16;
                        session.point = session.point.saturating_add(gained);
                        out.send(build_stat_update(0x26, i32::from(session.point)));
                        out.send(NpcTalkCodec::build_stat_point_toast_hex(gained.min(255) as u8));
                    }
                    // Type 4: EXP reward into session.texp and broadcast stat update 0x24.
                    4 => {
                        let exp = result.result_value.max(0) as u32;
                        session.texp = session.texp.saturating_add(exp);
                        out.send(build_stat_update(0x24, session.texp as i32));
                    }
                    other => tracing::debug!(
                        stt_bonus_type = other,
                        value = result.result_value,
                        "Eve stat-bonus result type has no executor; skipped"
                    ),
                },
                // parameter 2 (Bear `GetArmy`): army effects have no session
                // store yet — deferred (CP6 #10).
                2 => tracing::debug!(
                    army_type = result.parameter_style,
                    value = result.result_value,
                    "Eve army result skipped: no army store wired"
                ),
                other => tracing::debug!(
                    param = other,
                    style = result.parameter_style,
                    "Eve player-attribute class result has no executor; skipped"
                ),
            }
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
            // CP4/CP6: a non-Talk head result goes through the central walker —
            // Action results run to completion inline, a Surface awaits the
            // Sub 9 choice, and Door/Battle warp or park the session.
            None => execute_event_step(&mut conn.session, data, out),
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
            if ev.phase == EventPhase::AwaitingBattle {
                // The eve battle is in flight and the queue index is parked on
                // the battle result — a stray continue must not advance past
                // it (the post-battle resume owns that advance).
                tracing::debug!("ignoring talk-continue while awaiting eve battle");
                return;
            }
            // The queue index sits on the result already delivered; advance
            // past it before walking the queue again (research CP4 §1.3).
            ev.current_index = ev.current_index.saturating_add(1);
        }
        execute_event_step(&mut conn.session, data, out);
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
                crate::server::handlers::quest::save_map(&mut conn.session);
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
                crate::server::handlers::quest::save_map(&mut conn.session);
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

    // Stray menu select while the eve battle is in flight: ignore it rather
    // than fall into the legacy routing below (the parked battle result is
    // resumed by `battle::service::battle_ended`, not by Sub 9).
    let awaiting_battle = conn
        .session
        .current_event_session
        .as_ref()
        .is_some_and(|ev| ev.phase == EventPhase::AwaitingBattle);
    if awaiting_battle {
        tracing::debug!("ignoring menu select while awaiting eve battle");
        return;
    }

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
                execute_event_step(&mut conn.session, data, out);
            }
            // No branch matches this choice: close safely (unlock + 1408).
            None => end_talk(conn, out),
        }
        return;
    }

    // Legacy H6 path: store the raw choice byte for the talk-continue routing.
    conn.session.select_menu = i32::from(choice);
}
