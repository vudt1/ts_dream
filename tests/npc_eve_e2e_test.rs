//! Checkpoint 6 targeted suite: Eve Door/Battle dispatch, Action class 2/5/7
//! executors, quest/mission snapshot wiring, the post-battle resume, Eve/quest
//! persistence (merged into the `0001` baseline) and the auto-save fingerprint extension.
//!
//! Run (CP6 rule: targeted run only, never `--all-targets` for a touched file):
//!
//! ```bash
//! cargo test --test npc_eve_e2e_test
//! ```
//!
//! Design notes:
//! - Pure walker tests drive `execute_event_step` / `resume_eve_after_battle`
//!   against a **synthetic** `GameData` (no `Data/` load): the walker only
//!   consults `warps` and `scene_eve_data`, both injected below.
//! - `EventSession`s are built with `eve_no = 5`, `trigger_kind = 1` and a
//!   scene whose `npcs` map is empty, so `finish_event_session`'s auto-chain
//!   attempt deterministically yields `NoMatch` (no data-file dependence).
//! - Frame assertions compare against the same public codecs the server uses
//!   (`NpcTalkCodec` / `QuestSyncCodec`), except the two fixed legacy frames
//!   `F44402001407` (fade) and `F44402001408` (close dialog).
//!
//! **Persistence tests** bootstrap `sqlite::memory:?cache=shared` /
//! `tempfile` DBs running `0001_init.sql` — the *single* idempotent baseline
//! (since 2026-09-22 the three `character_quest_*` tables + `completioncount`
//! live there; there is no separate `0002` migration). The degradation test
//! re-creates a *pre-CP6* baseline by dropping the quest tables + column.

use ts_dream::battle::packets::hide_from_map;
use ts_dream::battle::runner::Outcome;
use ts_dream::battle::service::BattleService;
use ts_dream::data::loader::GameData;
use ts_dream::data::loaders::{EveFightData, EveResult, EveSceneInfo, SceneEveData};
use ts_dream::data::tables::Warp;
use ts_dream::db::modern::sqlite::SqliteRepositories;
use ts_dream::db::pool::{bootstrap, DbPool};
use ts_dream::eve::auto_chain::{EventPhase, EventSession};
use ts_dream::protocol::codecs::npc_talk::{NpcTalkCodec, TalkLockMode};
use ts_dream::protocol::codecs::quest_sync::QuestSyncCodec;
use ts_dream::server::auto_save;
use ts_dream::server::dispatcher::{dispatch, HandleOutcome, ServerEnv};
use ts_dream::server::handlers::npc_event::snapshot_state;
use ts_dream::server::handlers::shops::gold_frame;
use ts_dream::server::handlers::stats::build_stat_update;
use ts_dream::server::handlers::talk::{execute_event_step, resume_eve_after_battle};
use ts_dream::server::session::{lock_online_sessions, Conn, InventoryItem, Session};
use ts_dream::server::spawn;

const SOURCE_MAP: u16 = 9001; // scene + warp + fight 7 (scene_info background 55)))
const DEST_MAP: u16 = 9002; // Door destination
const FALLBACK_MAP: u16 = 9003; // scene with fight 8 but NO scene_infos -> diahinh 112
const NO_SCENE_MAP: u16 = 8888; // no scene_eve_data row at all
const WARP_ID: i64 = 3;
const FIGHT_ID: u16 = 7;

// ============================================================================
// Helpers
// ============================================================================

/// Synthetic `GameData`: one Door warp, one FightData with a background
/// (`scene_infos[1] = 55`) and one FightData without scene info (fallback).
fn synthetic_data() -> GameData {
    let mut data = GameData::default();
    data.warps.insert(
        (i64::from(SOURCE_MAP), WARP_ID),
        Warp {
            map1: i64::from(SOURCE_MAP),
            warpid: WARP_ID,
            map2: i64::from(DEST_MAP),
            x: 12,
            y: 34,
        },
    );
    let mut scene = SceneEveData::default();
    scene.fight_datas.insert(
        FIGHT_ID,
        EveFightData {
            eve_no: FIGHT_ID,
            ..Default::default()
        },
    );
    scene.scene_infos.insert(
        1,
        EveSceneInfo {
            eve_no: 1,
            background_no: 55,
            player_appear_x: 10,
            player_appear_y: 10,
            ..Default::default()
        },
    );
    data.scene_eve_data.insert(u32::from(SOURCE_MAP), scene);

    let mut fallback_scene = SceneEveData::default();
    fallback_scene.fight_datas.insert(
        8,
        EveFightData {
            eve_no: 8,
            ..Default::default()
        },
    );
    data.scene_eve_data
        .insert(u32::from(FALLBACK_MAP), fallback_scene);
    data
}

/// Bare session on `map_id` with the ids the frame assertions rely on.
fn test_session(map_id: u16) -> Session {
    let mut session = Session::new();
    session.id = 4242;
    session.map_id = map_id;
    session.map_x = 100;
    session.map_y = 100;
    session.level = 1;
    session
}

/// Session carrying an active event session queued at index 0.
///
/// `eve_no = 5` / `trigger_kind = 1` / empty `scene.npcs` keeps the auto-chain
/// attempt at finish time deterministic (`NoMatch`), so completion counts and
/// frame order are the only things under test.
fn session_with_event(map_id: u16, results: Vec<EveResult>) -> Session {
    let mut session = test_session(map_id);
    session.current_event_session = Some(EventSession {
        map_id: i32::from(map_id),
        eve_no: 5,
        npc_click_id: 1,
        trigger_kind: 1,
        chain_depth: 0,
        results,
        current_index: 0,
        phase: EventPhase::Executing,
        last_surface_id: -1,
        last_choice_code: -1,
        battle_result: 0,
        matched_condition_no: 0,
    });
    session
}

fn talk_result(mean_no: u16) -> EveResult {
    EveResult {
        result_type: 1,
        result_mean_no: mean_no,
        ..Default::default()
    }
}

fn door_result(warp_id: u16) -> EveResult {
    EveResult {
        result_type: 2,
        parameter: warp_id,
        ..Default::default()
    }
}

fn fight_result(fight_id: u16) -> EveResult {
    EveResult {
        result_type: 3,
        result_mean_no: fight_id,
        ..Default::default()
    }
}

/// Action / class 2 (quest log row): `parameter`=questId, `parameter_style`
/// =pStyle, `result_value`=step.
fn quest_action(quest_id: u16, p_style: u8, step: i32) -> EveResult {
    EveResult {
        result_type: 0,
        result_class: 2,
        parameter: quest_id,
        parameter_style: p_style,
        result_value: step,
        ..Default::default()
    }
}

/// Action / class 5 (gold): `parameter_style`=type, `result_value`=amount.
fn gold_action(gold_type: u8, amount: i32) -> EveResult {
    EveResult {
        result_type: 0,
        result_class: 5,
        parameter_style: gold_type,
        result_value: amount,
        ..Default::default()
    }
}

/// Action / class 7 (player attribute): `parameter`=effect, `parameter_style`
/// =type, `result_value`=value.
fn player_action(param: u16, p_style: u8, value: i32) -> EveResult {
    EveResult {
        result_type: 0,
        result_class: 7,
        parameter: param,
        parameter_style: p_style,
        result_value: value,
        ..Default::default()
    }
}

/// Frames whose main opcode (byte 4 of `F4 44 lenLE <op>`) is `0x18`.
fn op18_frames(frames: &[String]) -> Vec<&str> {
    frames
        .iter()
        .filter(|frame| {
            hex::decode(frame)
                .map(|bytes| bytes.len() >= 5 && bytes[4] == 0x18)
                .unwrap_or(false)
        })
        .map(String::as_str)
        .collect()
}

/// Plain `Vec<String>` copy of an outcome's frames (for `op18_frames`).
fn frame_list(out: &HandleOutcome) -> Vec<String> {
    out.outgoing.iter().map(|f| f.frame.clone()).collect()
}

fn unlock_frame(session_id: u32) -> String {
    NpcTalkCodec::build_talk_lock_hex(session_id, TalkLockMode::Unlock)
}

// ============================================================================
// Action class 2 — quest log executor (Bear QuestSaveHandler parity)
// ============================================================================

#[test]
fn action_class2_new_quest_writes_task_row_and_counts_completion() {
    let data = synthetic_data();
    // pStyle 4 is inside Bear's save set unconditionally; a brand-new quest
    // floors the written step at 1 (result_value 0 -> step 1).
    let mut session = session_with_event(SOURCE_MAP, vec![quest_action(700, 4, 0)]);
    let mut out = HandleOutcome::default();

    execute_event_step(&mut session, &data, &mut out);

    assert_eq!(session.quest_tasks.get(&700), Some(&(1, 1)));
    assert_eq!(
        out.outgoing[0].frame,
        QuestSyncCodec::build_task_entry_hex(1, 700, 1),
        "action must mirror exactly one 0x18 Sub 0x06 task frame"
    );
    assert_eq!(out.outgoing[1].frame, unlock_frame(session.id));
    assert_eq!(out.outgoing[2].frame, "F44402001408");
    assert_eq!(out.outgoing.len(), 3);
    assert!(out.eve_battle.is_none(), "actions never park a battle");
    assert!(
        session.current_event_session.is_none(),
        "exhausted queue finishes the session"
    );
    // has_state_changing_results() = true (carried an Action) -> counted.
    assert_eq!(session.completed_eve_counts.get(&5), Some(&1));
}

#[test]
fn action_class2_existing_quest_increments_or_backs_up() {
    let data = synthetic_data();

    // Increment: existing (slot 5, step 2) + result_value 3 -> step 5.
    let mut session = session_with_event(SOURCE_MAP, vec![quest_action(701, 1, 3)]);
    session.quest_tasks.insert(701, (5, 2));
    let mut out = HandleOutcome::default();
    execute_event_step(&mut session, &data, &mut out);
    assert_eq!(session.quest_tasks.get(&701), Some(&(5, 5)));
    assert_eq!(out.outgoing[0].frame, QuestSyncCodec::build_task_entry_hex(5, 701, 5));

    // Battle backup: resBattle 2 (lost) -> step backs up one (4 -> 3).
    let mut session = session_with_event(SOURCE_MAP, vec![quest_action(702, 1, 3)]);
    session.quest_tasks.insert(702, (9, 4));
    session
        .current_event_session
        .as_mut()
        .expect("event session")
        .battle_result = 2;
    let mut out = HandleOutcome::default();
    execute_event_step(&mut session, &data, &mut out);
    assert_eq!(session.quest_tasks.get(&702), Some(&(9, 3)));
    assert_eq!(out.outgoing[0].frame, QuestSyncCodec::build_task_entry_hex(9, 702, 3));
}

#[test]
fn action_class2_save_gate_skips_outside_bear_set() {
    let data = synthetic_data();
    // pStyle 1 with step 5 is NOT in Bear's save set {1,2,3,10,30,50,70}.
    let mut session = session_with_event(SOURCE_MAP, vec![quest_action(703, 1, 5)]);
    let mut out = HandleOutcome::default();

    execute_event_step(&mut session, &data, &mut out);

    assert!(
        session.quest_tasks.is_empty(),
        "step outside the save set must not write the log"
    );
    assert!(op18_frames(&frame_list(&out)).is_empty(), "no 0x18 frame on skip");
    // The walker still finishes: only the two closing frames.
    assert_eq!(out.outgoing.len(), 2);
    assert_eq!(out.outgoing[0].frame, unlock_frame(session.id));
    assert_eq!(out.outgoing[1].frame, "F44402001408");
}

#[test]
fn action_class2_remove_drops_quest_row() {
    let data = synthetic_data();

    // pStyle 3 + step 0 = drop the quest (Bear removerTask -> 0x18 Sub 0x04).
    let mut session = session_with_event(SOURCE_MAP, vec![quest_action(704, 3, 0)]);
    session.quest_tasks.insert(704, (3, 7));
    let mut out = HandleOutcome::default();
    execute_event_step(&mut session, &data, &mut out);
    assert!(
        !session.quest_tasks.contains_key(&704),
        "pStyle 3 + step 0 must drop the row"
    );
    assert_eq!(out.outgoing[0].frame, QuestSyncCodec::build_item_clear_hex(704));

    // Removing a quest that is not in the log emits nothing.
    let mut session = session_with_event(SOURCE_MAP, vec![quest_action(705, 3, 0)]);
    let mut out = HandleOutcome::default();
    execute_event_step(&mut session, &data, &mut out);
    assert!(op18_frames(&frame_list(&out)).is_empty());
}

// ============================================================================
// Action class 5 / class 7 — gold and player-attribute executors
// ============================================================================

#[test]
fn action_class5_gold_type1_points_then_gold() {
    let data = synthetic_data();

    // Type 1, amount < 1000: stat points at 20:1 (50 -> +1000 point).
    let mut session = session_with_event(SOURCE_MAP, vec![gold_action(1, 50)]);
    session.point = 100;
    let mut out = HandleOutcome::default();
    execute_event_step(&mut session, &data, &mut out);
    assert_eq!(session.point, 1100);
    assert_eq!(out.outgoing[0].frame, build_stat_update(0x26, 1100));
    assert_eq!(session.gold, 0);

    // Type 1, amount >= 1000: plain gold.
    let mut session = session_with_event(SOURCE_MAP, vec![gold_action(1, 1500)]);
    session.gold = 200;
    let mut out = HandleOutcome::default();
    execute_event_step(&mut session, &data, &mut out);
    assert_eq!(session.gold, 1700);
    assert_eq!(out.outgoing[0].frame, gold_frame(1700));
    assert_eq!(session.point, 0);

    // Type 2 has no proven executor: skipped (closing frames only).
    let mut session = session_with_event(SOURCE_MAP, vec![gold_action(2, 999)]);
    session.gold = 5;
    session.point = 7;
    let mut out = HandleOutcome::default();
    execute_event_step(&mut session, &data, &mut out);
    assert_eq!(session.gold, 5);
    assert_eq!(session.point, 7);
    assert_eq!(out.outgoing[0].frame, unlock_frame(session.id));
    assert_eq!(out.outgoing.len(), 2);
}

#[test]
fn action_class7_player_effects_and_deferred_types() {
    let data = synthetic_data();

    // parameter 1 + pStyle 2: skill points (0x25).
    let mut session = session_with_event(SOURCE_MAP, vec![player_action(1, 2, 5)]);
    session.skill_point = 10;
    let mut out = HandleOutcome::default();
    execute_event_step(&mut session, &data, &mut out);
    assert_eq!(session.skill_point, 15);
    assert_eq!(out.outgoing[0].frame, build_stat_update(0x25, 15));

    // parameter 1 + pStyle 3: stat points (0x26).
    let mut session = session_with_event(SOURCE_MAP, vec![player_action(1, 3, 7)]);
    session.point = 100;
    let mut out = HandleOutcome::default();
    execute_event_step(&mut session, &data, &mut out);
    assert_eq!(session.point, 107);
    assert_eq!(out.outgoing[0].frame, build_stat_update(0x26, 107));

    // pStyle 1 (Bear GetSaveMap): refresh the respawn point from the map.
    let mut session = session_with_event(SOURCE_MAP, vec![player_action(5, 1, 0)]);
    session.savemap = 0;
    let mut out = HandleOutcome::default();
    execute_event_step(&mut session, &data, &mut out);
    assert_eq!(session.savemap, SOURCE_MAP, "pStyle 1 must save the map");

    // parameter 2 (army) and pStyle 4 (EXP grant): deferred, no state change.
    let mut session = session_with_event(SOURCE_MAP, vec![player_action(2, 3, 99)]);
    session.point = 11;
    session.savemap = 0;
    let mut out = HandleOutcome::default();
    execute_event_step(&mut session, &data, &mut out);
    assert_eq!(session.point, 11, "army effect has no store yet");
    assert_eq!(session.savemap, 0, "pStyle 3 must not save the map");
    assert_eq!(out.outgoing.len(), 2, "deferred types emit only closing frames");

    let mut session = session_with_event(SOURCE_MAP, vec![player_action(1, 4, 50_000)]);
    session.point = 3;
    session.skill_point = 4;
    let mut out = HandleOutcome::default();
    execute_event_step(&mut session, &data, &mut out);
    assert_eq!(session.point, 3, "type 4 (setExp) stays deferred");
    assert_eq!(session.skill_point, 4);
}

// ============================================================================
// Door dispatch (result_type == 2)
// ============================================================================

#[test]
fn door_warps_then_finishes_session() {
    let data = synthetic_data();
    let mut session = session_with_event(SOURCE_MAP, vec![door_result(WARP_ID as u16)]);
    session.id = 5555;
    // Mirror the live registry entry so the Door warp can refresh it too.
    lock_online_sessions().insert(5555, session.clone());

    let mut out = HandleOutcome::default();
    execute_event_step(&mut session, &data, &mut out);

    assert_eq!(
        (session.map_id, session.map_x, session.map_y),
        (DEST_MAP, 12, 34),
        "Door must relocate through the Warps.dat row"
    );
    assert_eq!(
        out.outgoing[0].frame, "F44402001407",
        "fade screen first"
    );
    assert_eq!(
        out.outgoing[1].frame,
        spawn::build_relocate_packet(5555, DEST_MAP, 12, 34, WARP_ID as u8),
        "byte-identical relocate to the legacy warp-confirm path"
    );
    assert_eq!(out.outgoing[2].frame, unlock_frame(5555));
    assert_eq!(out.outgoing[3].frame, "F44402001408");
    assert_eq!(out.outgoing.len(), 4);
    assert!(out.eve_battle.is_none());
    assert!(
        session.current_event_session.is_none(),
        "Door ends its event session (CP6 #11)"
    );
    // Door carries neither Action nor Battle -> the completion watermark is
    // NOT bumped (CP6 #7).
    assert!(
        !session.completed_eve_counts.contains_key(&5),
        "pure Door sessions must not count as completed"
    );
    assert_eq!(out.map_broadcast.len(), 1);
    assert_eq!(out.map_broadcast[0].subject, 5555);
    assert_eq!(out.map_broadcast[0].map_id, Some(SOURCE_MAP));
    assert_eq!(out.map_broadcast[0].frame, hide_from_map(5555));

    let reg = lock_online_sessions();
    let online = reg.get(&5555).expect("registry entry");
    assert_eq!((online.map_id, online.map_x, online.map_y), (DEST_MAP, 12, 34));
    drop(reg);
    lock_online_sessions().remove(&5555);
}

#[test]
fn door_missing_warp_skips_then_finishes() {
    let data = synthetic_data();
    // No (9001, 99) row in Warps.dat -> skip, keep walking.
    let mut session = session_with_event(SOURCE_MAP, vec![door_result(99)]);
    let mut out = HandleOutcome::default();

    execute_event_step(&mut session, &data, &mut out);

    assert_eq!(session.map_id, SOURCE_MAP, "skip must not relocate");
    assert_eq!(out.outgoing.len(), 2);
    assert_eq!(out.outgoing[0].frame, unlock_frame(session.id));
    assert_eq!(out.outgoing[1].frame, "F44402001408");
    assert!(session.current_event_session.is_none());
}

#[test]
fn door_refuses_party_member_and_closes() {
    let data = synthetic_data();
    let mut session = session_with_event(SOURCE_MAP, vec![door_result(WARP_ID as u16)]);
    session.id = 5556;
    session.id_leader = 6000; // follower: warping would desync the party
    let mut out = HandleOutcome::default();

    execute_event_step(&mut session, &data, &mut out);

    assert_eq!(session.map_id, SOURCE_MAP, "a party follower must not warp");
    assert_eq!(out.outgoing.len(), 2, "unlock + close only, no relocate");
    assert_eq!(out.outgoing[1].frame, "F44402001408");
    assert!(session.current_event_session.is_none());
}

// ============================================================================
// Battle dispatch (result_type == 3) — park, flush order, guards
// ============================================================================

#[tokio::test]
async fn battle_parks_after_talk_and_stray_inputs_are_ignored() {
    let data = synthetic_data();
    let service = BattleService::default();
    let env = ServerEnv::none();

    let mut conn = Conn::default();
    conn.session.id = 4242;
    conn.session.map_id = SOURCE_MAP;
    let results = vec![talk_result(10364), fight_result(FIGHT_ID)];
    let talk_frame = NpcTalkCodec::build_talk_step_hex(&results[0]);
    conn.session.current_event_session = Some(EventSession {
        map_id: i32::from(SOURCE_MAP),
        eve_no: 5,
        npc_click_id: 1,
        trigger_kind: 1,
        chain_depth: 0,
        results,
        current_index: 0,
        phase: EventPhase::Executing,
        last_surface_id: -1,
        last_choice_code: -1,
        battle_result: 0,
        matched_condition_no: 0,
    });

    // 1) Talk-start delivery: the Talk frame goes out first (the connection
    //    loop flushes `out.outgoing` before it starts any eve battle).
    let mut out1 = HandleOutcome::default();
    execute_event_step(&mut conn.session, &data, &mut out1);
    assert_eq!(out1.outgoing.len(), 1);
    assert_eq!(out1.outgoing[0].frame, talk_frame);
    assert!(out1.eve_battle.is_none(), "Talk stops before the Battle result");
    {
        let ev = conn.session.current_event_session.as_ref().expect("session");
        assert_eq!(ev.current_index, 0, "Talk leaves the index on the delivered result");
        assert_eq!(ev.phase, EventPhase::Executing);
    }

    // 2) Sub 6 continue advances onto the Battle result and parks the session:
    //    the outcome carries the fight request but no extra frames, so the
    //    talk frame above is always ahead of the battle frames on the wire.
    let cont = [0xF4, 0x44, 0x02, 0x00, 0x14, 0x06];
    let out2 = dispatch(&mut conn, &cont, &data, &service, &env).await;
    assert!(out2.outgoing.is_empty(), "the Battle result itself emits no frames");
    assert_eq!(
        out2.eve_battle,
        Some((FIGHT_ID, 55)),
        "fight id + diahinh from scene_infos[1].background_no"
    );
    {
        let ev = conn.session.current_event_session.as_ref().expect("session");
        assert_eq!(ev.phase, EventPhase::AwaitingBattle);
        assert_eq!(ev.current_index, 1, "index parked on the battle result");
    }

    // 3) A stray continue while parked must not advance past the parked
    //    result (the post-battle resume owns that advance).
    let out3 = dispatch(&mut conn, &cont, &data, &service, &env).await;
    assert!(out3.outgoing.is_empty());
    assert!(out3.eve_battle.is_none());
    {
        let ev = conn.session.current_event_session.as_ref().expect("session");
        assert_eq!(ev.phase, EventPhase::AwaitingBattle);
        assert_eq!(ev.current_index, 1);
    }

    // 4) A stray menu select while parked is ignored too.
    let menu = [0xF4, 0x44, 0x03, 0x00, 0x14, 0x09, 0x01];
    let out4 = dispatch(&mut conn, &menu, &data, &service, &env).await;
    assert!(out4.outgoing.is_empty());
    {
        let ev = conn.session.current_event_session.as_ref().expect("session");
        assert_eq!(ev.phase, EventPhase::AwaitingBattle);
        assert_eq!(ev.current_index, 1);
    }
    assert!(conn.session.current_event_session.is_some(), "still parked");
}

#[test]
fn battle_missing_fight_row_skips() {
    let data = synthetic_data();

    // Scene exists but the fight id does not.
    let mut session = session_with_event(SOURCE_MAP, vec![fight_result(99)]);
    let mut out = HandleOutcome::default();
    execute_event_step(&mut session, &data, &mut out);
    assert!(out.eve_battle.is_none(), "unknown fight must never park");
    assert_eq!(out.outgoing.len(), 2, "walker skips and finishes");
    assert!(session.current_event_session.is_none());

    // Map has no Eve scene at all.
    let mut session = session_with_event(NO_SCENE_MAP, vec![fight_result(FIGHT_ID)]);
    let mut out = HandleOutcome::default();
    execute_event_step(&mut session, &data, &mut out);
    assert!(out.eve_battle.is_none());
    assert!(session.current_event_session.is_none());
}

#[test]
fn battle_diahinh_falls_back_to_112_without_scene_info() {
    let data = synthetic_data();
    let mut session = session_with_event(FALLBACK_MAP, vec![fight_result(8)]);
    let mut out = HandleOutcome::default();

    execute_event_step(&mut session, &data, &mut out);

    assert_eq!(out.eve_battle, Some((8, 112)), "missing scene_infos -> background 112");
    assert_eq!(
        session
            .current_event_session
            .as_ref()
            .expect("parked session")
            .phase,
        EventPhase::AwaitingBattle
    );
}

// ============================================================================
// Post-battle resume (CP6 #4)
// ============================================================================

#[test]
fn resume_win_continues_walk_and_records_quest() {
    let data = synthetic_data();
    let mut session = session_with_event(SOURCE_MAP, vec![fight_result(FIGHT_ID), quest_action(800, 1, 1)]);
    {
        let ev = session.current_event_session.as_mut().expect("session");
        ev.phase = EventPhase::AwaitingBattle;
        ev.current_index = 0; // parked on the battle result, as left by the walker
    }

    let (frames, next_battle) = resume_eve_after_battle(&mut session, Outcome::PlayerWin, &data);

    // Win advances past the battle result, executes the queued action, then
    // finishes the session.
    assert_eq!(session.quest_tasks.get(&800), Some(&(1, 1)), "post-battle quest write");
    assert_eq!(frames[0], QuestSyncCodec::build_task_entry_hex(1, 800, 1));
    assert_eq!(frames[1], unlock_frame(session.id));
    assert_eq!(frames[2], "F44402001408");
    assert_eq!(frames.len(), 3);
    assert!(next_battle.is_none(), "no chained battle in this queue");
    assert!(session.current_event_session.is_none(), "queue exhausted -> finished");
    assert_eq!(
        session.completed_eve_counts.get(&5),
        Some(&1),
        "Battle + Action session counts as completed (CP6 #7)"
    );
}

#[test]
fn resume_lose_or_flee_ends_talk_and_running_is_noop() {
    let data = synthetic_data();
    let mut session = session_with_event(SOURCE_MAP, vec![fight_result(FIGHT_ID), talk_result(10365)]);
    {
        let ev = session.current_event_session.as_mut().expect("session");
        ev.phase = EventPhase::AwaitingBattle;
        ev.current_index = 0;
    }

    // Defensive: Outcome::Running never reaches battle_ended -> untouched.
    let (frames, next) = resume_eve_after_battle(&mut session, Outcome::Running, &data);
    assert!(frames.is_empty() && next.is_none());
    {
        let ev = session.current_event_session.as_ref().expect("still parked");
        assert_eq!(ev.phase, EventPhase::AwaitingBattle);
        assert_eq!(ev.current_index, 0);
    }

    // Lose closes the talk outright (never a frozen dialog, no auto-chain).
    let (frames, next) = resume_eve_after_battle(&mut session, Outcome::PlayerLose, &data);
    assert_eq!(
        frames,
        vec![unlock_frame(session.id), "F44402001408".to_string()],
        "unlock then close — both fixed legacy frames"
    );
    assert!(next.is_none(), "a failed encounter never auto-chains");
    assert!(session.current_event_session.is_none());

    // Flee behaves like lose.
    let mut session = session_with_event(SOURCE_MAP, vec![fight_result(FIGHT_ID)]);
    {
        let ev = session.current_event_session.as_mut().expect("session");
        ev.phase = EventPhase::AwaitingBattle;
    }
    let (frames, next) = resume_eve_after_battle(&mut session, Outcome::PlayerFled, &data);
    assert_eq!(frames.len(), 2);
    assert_eq!(frames[1], "F44402001408");
    assert!(next.is_none());
    assert!(session.current_event_session.is_none());
}

#[test]
fn resume_leaves_unparked_sessions_untouched() {
    let data = synthetic_data();
    let mut session = session_with_event(SOURCE_MAP, vec![talk_result(10366)]);
    session.quest_tasks.insert(901, (4, 2));

    let (frames, next) = resume_eve_after_battle(&mut session, Outcome::PlayerWin, &data);

    assert!(frames.is_empty(), "not parked -> no output");
    assert!(next.is_none());
    assert!(session.current_event_session.is_some(), "session untouched");
    assert_eq!(session.quest_tasks.get(&901), Some(&(4, 2)), "state untouched");
}

// ============================================================================
// Snapshot wiring (npc_event::snapshot_state)
// ============================================================================

#[test]
fn snapshot_state_wires_quest_and_event_context() {
    let mut session = session_with_event(SOURCE_MAP, Vec::new());
    session.level = 7;
    session.quest_tasks.insert(900, (1, 4));
    session.quest_dont.insert(12);
    session.completed_eve_counts.insert(5, 2);

    let state = snapshot_state(&session);
    assert_eq!(state.mission_steps.get(&900), Some(&4), "quest log doubles as mission store");
    assert_eq!(state.mission_flags.get(&12), Some(&1), "quest-dont mark projected");
    assert_eq!(state.completed_eve_counts.get(&5), Some(&2));
    assert_eq!(state.level, 7);
    assert_eq!(
        (state.last_surface_id, state.last_choice_code, state.battle_result),
        (-1, -1, 0),
        "defaults without an answered surface / battle"
    );

    // The active event session feeds surface/choice/battle context.
    {
        let ev = session.current_event_session.as_mut().expect("session");
        ev.last_surface_id = 10364;
        ev.last_choice_code = 2;
        ev.battle_result = 1;
    }
    let state = snapshot_state(&session);
    assert_eq!(
        (state.last_surface_id, state.last_choice_code, state.battle_result),
        (10364, 2, 1)
    );
}

// ============================================================================
// Persistence (migration 0002) + auto-save fingerprint
// ============================================================================

/// Schema for the round-trip test: `0001` alone (the single idempotent
/// baseline — the Eve tables were merged into it on 2026-09-22).
async fn setup_test_db() -> DbPool {
    let pool = bootstrap("sqlite::memory:?cache=shared", None)
        .await
        .expect("bootstrap in-memory SQLite");
    sqlx::raw_sql(include_str!("../migrations/0001_init.sql"))
        .execute(&pool.write)
        .await
        .expect("execute 0001_init.sql migration");
    pool
}

/// Minimal `accounts` + `characters` rows so `save()`/`load()` have a target.
async fn seed_character(pool: &DbPool, account_id: i64) {
    sqlx::query(
        "INSERT INTO accounts (playerid, pass1, pass2, createdat) VALUES (?, '123456', '123456', 0)",
    )
    .bind(account_id)
    .execute(&pool.write)
    .await
    .expect("insert account");
    sqlx::query(
        "INSERT INTO characters (playerid, name, level, jobtype, gender, hair, element, rebornstage, curhp, maxhp, cursp, maxsp, freepoints, skillpoint, baseint, baseatk, basedef, basehpx, basespx, baseagi, mapid, mapx, mapy, pk, curexp, newbie)
         VALUES (?, X'746573746572', 1, 0, 1, 1, 1, 0, 100, 100, 50, 50, 0, 0, 10, 10, 10, 10, 10, 10, 10817, 100, 200, 0, 0, 0)",
    )
    .bind(account_id)
    .execute(&pool.write)
    .await
    .expect("insert character");
}

fn eve_state_session(account_id: i64) -> Session {
    let mut session = Session::new();
    session.id = account_id as u32;
    session.db_character_id = account_id;
    session.quest_tasks.insert(700, (1, 3));
    session.quest_tasks.insert(701, (2, 9));
    session.quest_dont.insert(5);
    session.quest_dont.insert(7);
    session.quest_items.push(InventoryItem {
        slot: 3,
        id: 46016,
        count: 2,
        ..Default::default()
    });
    session.completed_eve_counts.insert(5, 2);
    session
}

#[tokio::test]
async fn persistence_round_trip_eve_state() {
    let pool = setup_test_db().await;
    let account_id = 7001i64;
    seed_character(&pool, account_id).await;

    let repos = SqliteRepositories::new(pool.clone());
    repos
        .sessions()
        .save(&eve_state_session(account_id))
        .await
        .expect("save() must commit the Eve/quest state in its own transaction");

    // The `completioncount` column (merged into the 0001 baseline) is directly
    // readable.
    let count: i64 = sqlx::query_scalar(
        "SELECT completioncount FROM character_completed_events WHERE playerid = ? AND eventid = 5",
    )
    .bind(account_id)
    .fetch_one(&pool.read)
    .await
    .expect("completioncount column");
    assert_eq!(count, 2);

    let mut loaded = Session::new();
    let found = repos
        .sessions()
        .load(account_id, &mut loaded)
        .await
        .expect("load()");
    assert!(found, "character row must be found");
    assert_eq!(loaded.quest_tasks.len(), 2);
    assert_eq!(loaded.quest_tasks.get(&700), Some(&(1, 3)));
    assert_eq!(loaded.quest_tasks.get(&701), Some(&(2, 9)));
    assert_eq!(loaded.quest_dont.len(), 2);
    assert!(loaded.quest_dont.contains(&5) && loaded.quest_dont.contains(&7));
    assert_eq!(loaded.quest_items.len(), 1);
    assert_eq!(
        (
            loaded.quest_items[0].slot,
            loaded.quest_items[0].id,
            loaded.quest_items[0].count
        ),
        (3, 46016, 2)
    );
    assert_eq!(loaded.completed_eve_counts.get(&5), Some(&2));
}

#[tokio::test]
async fn persistence_degrades_when_quest_tables_are_missing() {
    // Isolated file DB: apply 0001 then *re-create a pre-CP6 baseline* —
    // drop the Eve/quest tables and the `completioncount` column. Models a
    // database created before Checkpoint 6 (ADR 0004: `pool::migrate` is a
    // no-op, so a live DB only gets new tables by applying the script) —
    // save/load must degrade, never fail.
    let dir = tempfile::tempdir().expect("tempdir");
    let url = format!("sqlite://{}", dir.path().join("no_eve.db").display());
    let pool = bootstrap(&url, None).await.expect("bootstrap file db");
    sqlx::raw_sql(include_str!("../migrations/0001_init.sql"))
        .execute(&pool.write)
        .await
        .expect("execute 0001_init.sql migration");
    sqlx::raw_sql(
        "DROP TABLE IF EXISTS character_quest_tasks;
         DROP TABLE IF EXISTS character_quest_dont;
         DROP TABLE IF EXISTS character_quest_items;
         CREATE TABLE character_completed_events_pre_cp6 AS
             SELECT playerid, eventid, completedat FROM character_completed_events;
         DROP TABLE character_completed_events;
         ALTER TABLE character_completed_events_pre_cp6
             RENAME TO character_completed_events;",
    )
    .execute(&pool.write)
    .await
    .expect("re-create pre-CP6 baseline schema");
    seed_character(&pool, 8001).await;

    let repos = SqliteRepositories::new(pool.clone());
    let mut session = Session::new();
    session.id = 8001;
    session.db_character_id = 8001;
    session.quest_tasks.insert(700, (1, 3));
    session.completed_eve_counts.insert(5, 2);

    repos
        .sessions()
        .save(&session)
        .await
        .expect("save() must degrade (debug log), not fail, without quest tables");

    let mut loaded = Session::new();
    let found = repos
        .sessions()
        .load(8001, &mut loaded)
        .await
        .expect("load() must degrade (debug log), not fail, without quest tables");
    assert!(found, "the 0001 character row still loads");
    assert!(
        loaded.quest_tasks.is_empty(),
        "missing quest tables -> empty quest state"
    );
    assert!(loaded.quest_dont.is_empty());
    assert!(loaded.quest_items.is_empty());
    assert!(loaded.completed_eve_counts.is_empty());
}

#[test]
fn fingerprint_tracks_eve_state() {
    let base = test_session(SOURCE_MAP);
    let fp_base = auto_save::fingerprint(&base);

    // Quest task row dirties the fingerprint; so does a step change on it.
    let mut with_task = base.clone();
    with_task.quest_tasks.insert(700, (1, 3));
    assert_ne!(fp_base, auto_save::fingerprint(&with_task), "quest task insert");
    let mut step_up = with_task.clone();
    step_up.quest_tasks.insert(700, (1, 4));
    assert_ne!(
        auto_save::fingerprint(&with_task),
        auto_save::fingerprint(&step_up),
        "quest step change"
    );

    // Quest-dont flag, quest item and completion count each dirty it too.
    let mut with_dont = with_task.clone();
    with_dont.quest_dont.insert(5);
    assert_ne!(
        auto_save::fingerprint(&with_task),
        auto_save::fingerprint(&with_dont),
        "quest-dont insert"
    );
    let mut with_item = with_dont.clone();
    with_item.quest_items.push(InventoryItem {
        slot: 1,
        id: 46016,
        count: 1,
        ..Default::default()
    });
    assert_ne!(
        auto_save::fingerprint(&with_dont),
        auto_save::fingerprint(&with_item),
        "quest item insert"
    );
    let mut with_count = with_item.clone();
    with_count.completed_eve_counts.insert(5, 1);
    assert_ne!(
        auto_save::fingerprint(&with_item),
        auto_save::fingerprint(&with_count),
        "completion count insert"
    );

    // Same content in different insertion order must fingerprint identically
    // (the mixers sort their keys — HashMap iteration order is randomised per
    // instance, so an unsorted mix would flap here).
    let mut first = Session::new();
    first.quest_tasks.insert(700, (1, 1));
    first.quest_tasks.insert(701, (2, 2));
    first.quest_tasks.insert(702, (3, 3));
    let mut second = Session::new();
    second.quest_tasks.insert(702, (3, 3));
    second.quest_tasks.insert(701, (2, 2));
    second.quest_tasks.insert(700, (1, 1));
    assert_eq!(
        auto_save::fingerprint(&first),
        auto_save::fingerprint(&second),
        "fingerprint must be insertion-order independent"
    );
}
