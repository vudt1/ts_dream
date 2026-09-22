//! Unit & integration tests for Phase 1 P0 Blockers:
//! - Task 1.1: Context preservation in `snapshot_state_with_context` & `auto_chain_after` (Fix Vấn đề 1).
//! - Task 1.2: Post-battle resume for PlayerLose and PlayerFled with quest step back-up (Fix Vấn đề 2).
//! - Task 1.3: Eve Door warp without premature 14 08 packet (Fix Vấn đề 3).

use ts_dream::battle::rng::DotNetRandom;
use ts_dream::battle::runner::Outcome;
use ts_dream::data::loader::GameData;
use ts_dream::data::loaders::{
    EveCondition, EveFightData, EveNpcPlacement, EveResult, EveSceneInfo, NpcEventData,
    SceneEveData,
};
use ts_dream::data::tables::Warp;
use ts_dream::eve::auto_chain::{AutoChainResult, EventPhase, EventSession};
use ts_dream::protocol::codecs::npc_talk::{NpcTalkCodec, TalkLockMode};
use ts_dream::protocol::codecs::quest_sync::QuestSyncCodec;
use ts_dream::server::dispatcher::HandleOutcome;
use ts_dream::server::handlers::npc_event::{
    auto_chain_after, resolve_npc_event, set_eve_events_enabled, snapshot_state,
    snapshot_state_with_context, NpcTrigger,
};
use ts_dream::server::handlers::talk::{execute_event_step, resume_eve_after_battle};
use ts_dream::server::session::{lock_online_sessions, Session};
use ts_dream::server::spawn;

const TEST_MAP: u16 = 7701;
const DEST_MAP: u16 = 7702;
const WARP_ID: i64 = 5;
const FIGHT_ID: u16 = 42;

fn test_game_data() -> GameData {
    let mut data = GameData::default();
    data.warps.insert(
        (i64::from(TEST_MAP), WARP_ID),
        Warp {
            map1: i64::from(TEST_MAP),
            warpid: WARP_ID,
            map2: i64::from(DEST_MAP),
            x: 200,
            y: 300,
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
            background_no: 110,
            player_appear_x: 200,
            player_appear_y: 300,
            ..Default::default()
        },
    );
    data.scene_eve_data.insert(u32::from(TEST_MAP), scene);
    data
}

fn fight_result(fight_id: u16) -> EveResult {
    EveResult {
        result_type: 3,
        result_mean_no: fight_id,
        ..Default::default()
    }
}

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

fn talk_result(dialog_id: u16) -> EveResult {
    EveResult {
        result_type: 1,
        result_mean_no: dialog_id,
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

fn session_with_event(map_id: u16, results: Vec<EveResult>) -> Session {
    let mut session = Session::new();
    session.id = 9999;
    session.map_id = map_id;
    session.map_x = 100;
    session.map_y = 100;
    session.current_event_session = Some(EventSession {
        map_id: i32::from(map_id),
        eve_no: 10,
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

// ============================================================================
// Task 1.1 Tests: Context preservation in snapshot_state_with_context
// ============================================================================

#[test]
fn test_snapshot_state_with_context_preserves_completed_session_context() {
    let mut session = Session::new();
    session.id = 1234;
    session.level = 25;
    session.quest_tasks.insert(101, (1, 3));

    // Initially no active event session on the Session struct (e.g. after session.current_event_session.take())
    assert!(session.current_event_session.is_none());

    // Without context, default values are returned:
    let default_state = snapshot_state(&session);
    assert_eq!(default_state.last_surface_id, -1);
    assert_eq!(default_state.last_choice_code, -1);
    assert_eq!(default_state.battle_result, 0);

    // Create a completed event session that recorded battle win and a choice
    let completed = EventSession {
        map_id: 10817,
        eve_no: 10,
        npc_click_id: 1,
        trigger_kind: 1,
        chain_depth: 0,
        results: Vec::new(),
        current_index: 0,
        phase: EventPhase::Executing,
        last_surface_id: 2001,
        last_choice_code: 3,
        battle_result: 1, // Won battle
        matched_condition_no: 1,
    };

    // When snapshot_state_with_context is called with Some(&completed), the context is preserved
    let state = snapshot_state_with_context(&session, Some(&completed));
    assert_eq!(state.last_surface_id, 2001, "last_surface_id must be preserved from completed session");
    assert_eq!(state.last_choice_code, 3, "last_choice_code must be preserved from completed session");
    assert_eq!(state.battle_result, 1, "battle_result must be preserved from completed session");
    assert_eq!(state.level, 25);
    assert_eq!(state.mission_steps.get(&101), Some(&3));
}

#[test]
fn test_snapshot_state_with_context_fallback_to_current_event_session() {
    let mut session = Session::new();
    session.id = 1234;
    session.current_event_session = Some(EventSession {
        map_id: 10817,
        eve_no: 10,
        npc_click_id: 1,
        trigger_kind: 1,
        chain_depth: 0,
        results: Vec::new(),
        current_index: 0,
        phase: EventPhase::Executing,
        last_surface_id: 1005,
        last_choice_code: 2,
        battle_result: 2, // Lost battle
        matched_condition_no: 0,
    });

    // When active is None, it should fall back to session.current_event_session
    let state = snapshot_state_with_context(&session, None);
    assert_eq!(state.last_surface_id, 1005);
    assert_eq!(state.last_choice_code, 2);
    assert_eq!(state.battle_result, 2);
}

#[test]
fn test_auto_chain_after_preserves_battle_result_for_next_event() {
    let mut data = test_game_data();
    let scene = data.scene_eve_data.get_mut(&u32::from(TEST_MAP)).unwrap();

    // NPC 1 has events [1, 2]
    scene.npcs.insert(
        1,
        EveNpcPlacement {
            id: 1,
            npc_id: 33001,
            events: vec![1, 2],
            ..Default::default()
        },
    );

    // Event 1: battle
    scene.npc_events.insert(
        1,
        NpcEventData {
            eve_no: 1,
            when_happen: [true, false, false, false],
            conditions: vec![EveCondition {
                condition_no: 0,
                condition_class: 0, // Unconditional
                and_num: 1,
                to_result: 1,
                results: vec![fight_result(FIGHT_ID)],
                ..Default::default()
            }],
        },
    );

    // Event 2: requires BattleResult == 1 (Win)
    scene.npc_events.insert(
        2,
        NpcEventData {
            eve_no: 2,
            when_happen: [true, false, false, false],
            conditions: vec![EveCondition {
                condition_no: 1,
                condition_class: 8, // BattleResult
                condition_ops: 1,   // ==
                condition_parameter_style: 1, // Win
                and_num: 1,
                to_result: 1,
                results: vec![talk_result(20001)],
                ..Default::default()
            }],
        },
    );

    let mut session = Session::new();
    session.id = 5555;
    session.map_id = TEST_MAP;
    // finish_event_session records completion count before calling auto_chain_after
    session.completed_eve_counts.insert(1, 1);
    // session.current_event_session is None (simulating post-take() in finish_event_session)
    assert!(session.current_event_session.is_none());

    let completed_win = EventSession {
        map_id: i32::from(TEST_MAP),
        eve_no: 1,
        npc_click_id: 1,
        trigger_kind: 1,
        chain_depth: 0,
        results: vec![fight_result(FIGHT_ID)],
        current_index: 1,
        phase: EventPhase::Executing,
        last_surface_id: -1,
        last_choice_code: -1,
        battle_result: 1, // Win!
        matched_condition_no: 0,
    };

    let mut rng = DotNetRandom::new(42);

    // auto_chain_after uses snapshot_state_with_context(session, Some(completed))
    let result = auto_chain_after(&completed_win, &session, &data, &mut rng);

    match result {
        AutoChainResult::Chained(chained) => {
            assert_eq!(chained.eve_no, 2, "Must chain into Event 2 matching BattleResult == 1");
            assert_eq!(chained.chain_depth, 1);
            assert_eq!(chained.results.len(), 1);
            assert_eq!(chained.results[0].result_mean_no, 20001);
        }
        AutoChainResult::NoMatch => {
            panic!("Expected auto-chain to match Event 2 on battle win, but got NoMatch!");
        }
    }

    // Now test with completed_loss: should NOT match Event 2 (which requires Win)
    let completed_loss = EventSession {
        map_id: i32::from(TEST_MAP),
        eve_no: 1,
        npc_click_id: 1,
        trigger_kind: 1,
        chain_depth: 0,
        results: vec![fight_result(FIGHT_ID)],
        current_index: 1,
        phase: EventPhase::Executing,
        last_surface_id: -1,
        last_choice_code: -1,
        battle_result: 2, // Lost
        matched_condition_no: 0,
    };
    let mut rng = DotNetRandom::new(42);
    let result_loss = auto_chain_after(&completed_loss, &session, &data, &mut rng);
    assert_eq!(result_loss, AutoChainResult::NoMatch, "Loss should not match Win condition");
}

// ============================================================================
// Task 1.2 Tests: Post-battle resume for PlayerLose and PlayerFled
// ============================================================================

#[test]
fn test_resume_after_battle_loss_executes_quest_backup() {
    let data = test_game_data();
    // Script: battle followed by quest step save (class 2, style 1, step 3)
    let mut session = session_with_event(
        TEST_MAP,
        vec![fight_result(FIGHT_ID), quest_action(702, 1, 3)],
    );
    // Initial quest state: slot 2, step 5
    session.quest_tasks.insert(702, (2, 5));
    {
        let ev = session.current_event_session.as_mut().expect("session");
        ev.phase = EventPhase::AwaitingBattle;
        ev.current_index = 0; // Parked on battle result
    }

    // Battle outcome: PlayerLose
    let (frames, next_battle) = resume_eve_after_battle(&mut session, Outcome::PlayerLose, &data);

    // The walker must advance to index 1, execute quest_action,
    // which detects battle_result == 2 and backs up current step: 5 - 1 = 4!
    assert_eq!(
        session.quest_tasks.get(&702),
        Some(&(2, 4)),
        "Quest step must back up by 1 on lost battle (5 -> 4)"
    );
    assert_eq!(
        frames[0],
        QuestSyncCodec::build_task_entry_hex(2, 702, 4),
        "Must emit 0x18 Sub 0x06 update frame for decremented step"
    );
    assert_eq!(
        frames[1],
        NpcTalkCodec::build_talk_lock_hex(session.id, TalkLockMode::Unlock)
    );
    assert_eq!(frames[2], "F44402001408");
    assert!(next_battle.is_none());
    assert!(session.current_event_session.is_none(), "Queue exhausted -> finished");
    assert_eq!(
        session.completed_eve_counts.get(&10),
        None,
        "Failed encounter must NOT be marked as completed (Fix Vấn đề 2)"
    );
}

#[test]
fn test_resume_after_battle_fled_executes_quest_backup() {
    let data = test_game_data();
    let mut session = session_with_event(
        TEST_MAP,
        vec![fight_result(FIGHT_ID), quest_action(703, 1, 2)],
    );
    // Initial quest state: slot 3, step 3
    session.quest_tasks.insert(703, (3, 3));
    {
        let ev = session.current_event_session.as_mut().expect("session");
        ev.phase = EventPhase::AwaitingBattle;
        ev.current_index = 0;
    }

    // Battle outcome: PlayerFled
    let (frames, next_battle) = resume_eve_after_battle(&mut session, Outcome::PlayerFled, &data);

    // PlayerFled -> battle_result == 3 -> current.saturating_sub(1) = 2
    assert_eq!(
        session.quest_tasks.get(&703),
        Some(&(3, 2)),
        "Quest step must back up by 1 on fled battle (3 -> 2)"
    );
    assert_eq!(
        frames[0],
        QuestSyncCodec::build_task_entry_hex(3, 703, 2)
    );
    assert!(next_battle.is_none());
    assert!(session.current_event_session.is_none());
    assert_eq!(
        session.completed_eve_counts.get(&10),
        None,
        "Fled encounter must NOT be marked as completed (Fix Vấn đề 2)"
    );
}

#[test]
fn test_resume_after_battle_loss_delivers_subsequent_dialogue() {
    let data = test_game_data();
    let dialog_id = 10888;
    let mut session = session_with_event(
        TEST_MAP,
        vec![fight_result(FIGHT_ID), talk_result(dialog_id)],
    );
    {
        let ev = session.current_event_session.as_mut().expect("session");
        ev.phase = EventPhase::AwaitingBattle;
        ev.current_index = 0;
    }

    let (frames, next_battle) = resume_eve_after_battle(&mut session, Outcome::PlayerLose, &data);

    assert_eq!(frames.len(), 1, "Must deliver subsequent talk frame");
    assert_eq!(
        frames[0],
        NpcTalkCodec::build_talk_step_hex(&talk_result(dialog_id))
    );
    assert!(next_battle.is_none());
    assert!(session.current_event_session.is_some(), "Session remains active awaiting client continue");
    let ev = session.current_event_session.as_ref().unwrap();
    assert_eq!(ev.battle_result, 2, "battle_result must be recorded as 2 (Lose)");
    assert_eq!(ev.current_index, 1);
}

// ============================================================================
// Task 1.3 Tests: Eve Door warp without premature 14 08
// ============================================================================

#[test]
fn test_eve_door_warp_does_not_send_early_14_08_or_unlock() {
    let data = test_game_data();
    let mut session = session_with_event(TEST_MAP, vec![door_result(WARP_ID as u16)]);
    session.id = 6677;
    session.idtalking = 1;

    lock_online_sessions().insert(6677, session.clone());

    let mut out = HandleOutcome::default();
    execute_event_step(&mut session, &data, &mut out);

    // Verify coordinates and map relocated:
    assert_eq!(session.map_id, DEST_MAP);
    assert_eq!(session.map_x, 200);
    assert_eq!(session.map_y, 300);

    // Exactly 2 frames: 14 07 (Fade screen) and Relocate packet (0x0C)
    assert_eq!(out.outgoing.len(), 2, "Must emit exactly 2 frames: fade and relocate");
    assert_eq!(out.outgoing[0].frame, "F44402001407", "Frame 0 must be screen fade (14 07)");
    assert_eq!(
        out.outgoing[1].frame,
        spawn::build_relocate_packet(6677, DEST_MAP, 200, 300, WARP_ID as u8),
        "Frame 1 must be relocate packet (Opcode 0x0C)"
    );

    // CRITICAL (Fix Vấn đề 3): absolutely no 14 08 (EndTalk) and no 14 2C (Unlock)
    for frame in &out.outgoing {
        assert_ne!(frame.frame, "F44402001408", "Must NOT send 14 08 during Door warp!");
        assert!(
            !frame.frame.contains("142C"),
            "Must NOT send 14 2C unlock during Door warp!"
        );
    }

    // Talk context and event session must be cleaned up in memory:
    assert!(session.current_event_session.is_none(), "current_event_session must be cleared");
    assert_eq!(session.idtalking, 0, "idtalking must be reset");

    lock_online_sessions().remove(&6677);
}

#[test]
fn test_battle_loss_does_not_mark_completed_and_permits_retry() {
    set_eve_events_enabled(true);
    let mut data = test_game_data();
    let scene = data.scene_eve_data.get_mut(&u32::from(TEST_MAP)).unwrap();

    // NPC 1 on TEST_MAP with event 1
    scene.npcs.insert(
        1,
        EveNpcPlacement {
            id: 1,
            npc_id: 33001,
            events: vec![1],
            ..Default::default()
        },
    );

    // Event 1: Unconditional battle followed by quest step change
    scene.npc_events.insert(
        1,
        NpcEventData {
            eve_no: 1,
            when_happen: [true, false, false, false],
            conditions: vec![EveCondition {
                condition_no: 0,
                condition_class: 0,
                and_num: 1,
                to_result: 1,
                results: vec![fight_result(FIGHT_ID), quest_action(702, 1, 3)],
                ..Default::default()
            }],
        },
    );

    let mut session = Session::new();
    session.id = 8888;
    session.map_id = TEST_MAP;
    session.quest_tasks.insert(702, (1, 5));

    let mut rng = DotNetRandom::new(12345);

    // 1. First interaction: talk to NPC 1 -> trigger battle
    let ev = resolve_npc_event(&session, &data, NpcTrigger::ClickNpc(1), &mut rng)
        .expect("First interaction must resolve event 1");
    assert_eq!(ev.eve_no, 1);
    session.current_event_session = Some(ev);

    let mut out = HandleOutcome::default();
    execute_event_step(&mut session, &data, &mut out);
    assert_eq!(out.eve_battle, Some((FIGHT_ID, 110)));
    assert_eq!(
        session
            .current_event_session
            .as_ref()
            .map(|e| e.phase),
        Some(EventPhase::AwaitingBattle)
    );

    // 2. Battle result: Outcome::PlayerLose
    let (_frames, next_battle) = resume_eve_after_battle(&mut session, Outcome::PlayerLose, &data);
    assert!(next_battle.is_none());
    assert!(session.current_event_session.is_none());
    // Quest step backed up (5 -> 4)
    assert_eq!(session.quest_tasks.get(&702), Some(&(1, 4)));
    // CRITICAL: Completed count must NOT be incremented for event 1!
    assert_eq!(
        session.completed_eve_counts.get(&1),
        None,
        "Event 1 must NOT be marked completed on battle loss"
    );

    // 3. Second interaction: Player retries by clicking NPC 1 again!
    let ev_retry = resolve_npc_event(&session, &data, NpcTrigger::ClickNpc(1), &mut rng)
        .expect("Player must be able to retry event 1 after losing the battle!");
    assert_eq!(ev_retry.eve_no, 1);
    session.current_event_session = Some(ev_retry);

    let mut out2 = HandleOutcome::default();
    execute_event_step(&mut session, &data, &mut out2);
    assert_eq!(out2.eve_battle, Some((FIGHT_ID, 110)));

    // 4. Battle result: Outcome::PlayerWin
    let (_frames_win, next_battle_win) =
        resume_eve_after_battle(&mut session, Outcome::PlayerWin, &data);
    assert!(next_battle_win.is_none());
    assert!(session.current_event_session.is_none());
    // Quest step advanced on win (4 + 3 = 7)
    assert_eq!(session.quest_tasks.get(&702), Some(&(1, 7)));
    // CRITICAL: Completed count MUST be incremented on battle win!
    assert_eq!(
        session.completed_eve_counts.get(&1),
        Some(&1),
        "Event 1 MUST be marked completed on battle win"
    );

    // 5. Third interaction: Player clicks NPC 1 again -> Event 1 is now skipped!
    let ev_third = resolve_npc_event(&session, &data, NpcTrigger::ClickNpc(1), &mut rng);
    assert!(
        ev_third.is_none(),
        "Event 1 must be skipped after successful completion"
    );
}

#[test]
fn test_battle_flee_does_not_mark_completed_and_permits_retry() {
    set_eve_events_enabled(true);
    let mut data = test_game_data();
    let scene = data.scene_eve_data.get_mut(&u32::from(TEST_MAP)).unwrap();

    scene.npcs.insert(
        2,
        EveNpcPlacement {
            id: 2,
            npc_id: 33002,
            events: vec![3],
            ..Default::default()
        },
    );

    scene.npc_events.insert(
        3,
        NpcEventData {
            eve_no: 3,
            when_happen: [true, false, false, false],
            conditions: vec![EveCondition {
                condition_no: 0,
                condition_class: 0,
                and_num: 1,
                to_result: 1,
                results: vec![fight_result(FIGHT_ID)],
                ..Default::default()
            }],
        },
    );

    let mut session = Session::new();
    session.id = 8889;
    session.map_id = TEST_MAP;

    let mut rng = DotNetRandom::new(999);

    // 1. First interaction: start battle
    let ev = resolve_npc_event(&session, &data, NpcTrigger::ClickNpc(2), &mut rng)
        .expect("First interaction must resolve event 3");
    session.current_event_session = Some(ev);

    let mut out = HandleOutcome::default();
    execute_event_step(&mut session, &data, &mut out);
    assert_eq!(out.eve_battle, Some((FIGHT_ID, 110)));

    // 2. Flee battle
    let (_frames, next) = resume_eve_after_battle(&mut session, Outcome::PlayerFled, &data);
    assert!(next.is_none());
    assert!(session.current_event_session.is_none());
    assert_eq!(
        session.completed_eve_counts.get(&3),
        None,
        "Event 3 must NOT be marked completed on flee"
    );

    // 3. Retry interaction after fleeing
    let ev_retry = resolve_npc_event(&session, &data, NpcTrigger::ClickNpc(2), &mut rng)
        .expect("Player must be able to retry event 3 after fleeing!");
    assert_eq!(ev_retry.eve_no, 3);
}
