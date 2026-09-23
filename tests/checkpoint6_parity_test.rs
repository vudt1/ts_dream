//! Comprehensive tests for Checkpoint 6 Remediation & Parity (Phases 2, 3, 4, 5):
//! - Task 2.1: Action Class 5 - Gold deduction (parameter_style == 2) with saturating_sub & gold_frame.
//! - Task 2.2: Action Class 7 - Stat & Skill point toasts (14 16 / 14 17) & EXP reward (style 4, stat 0x24).
//! - Task 3.1: MarkDatLoader - Decodes Data/Mark.Dat (516B/record, XOR 0x2774, offset - 7).
//! - Task 3.2: Action Class 2 - Auto quest_dont trigger when mark position > 0.
//! - Task 4.1: Party Door Warp - Leader walking through Eve Door warps all party members.
//! - Task 5.1: Real Trác Quận NPC interaction with Data/eve.emg and Data/Mark.Dat.

use std::path::PathBuf;
use ts_dream::battle::rng::DotNetRandom;
use ts_dream::data::loader::GameData;
use ts_dream::data::loaders::mark::MarkDatLoader;
use ts_dream::data::loaders::{EveResult, EveSceneInfo, SceneEveData};
use ts_dream::data::tables::Warp;
use ts_dream::eve::auto_chain::{EventPhase, EventSession};
use ts_dream::protocol::codecs::npc_talk::NpcTalkCodec;
use ts_dream::server::dispatcher::HandleOutcome;
use ts_dream::server::handlers::npc_event::{resolve_npc_event, set_eve_events_enabled, NpcTrigger};
use ts_dream::server::handlers::shops::gold_frame;
use ts_dream::server::handlers::stats::build_stat_update;
use ts_dream::server::handlers::talk::execute_event_step;
use ts_dream::server::session::{lock_online_sessions, Session};
use ts_dream::server::spawn;

fn data_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("Data")
}

const TEST_MAP: u16 = 7701;
const DEST_MAP: u16 = 7702;
const WARP_ID: i64 = 10;

fn test_synthetic_data() -> GameData {
    let mut data = GameData::default();
    data.warps.insert(
        (i64::from(TEST_MAP), WARP_ID),
        Warp {
            map1: i64::from(TEST_MAP),
            warpid: WARP_ID,
            map2: i64::from(DEST_MAP),
            x: 500,
            y: 600,
        },
    );
    let mut scene = SceneEveData::default();
    scene.scene_infos.insert(
        1,
        EveSceneInfo {
            eve_no: 1,
            background_no: 110,
            player_appear_x: 500,
            player_appear_y: 600,
            ..Default::default()
        },
    );
    data.scene_eve_data.insert(u32::from(TEST_MAP), scene);
    data
}

fn session_with_action(results: Vec<EveResult>) -> Session {
    let mut session = Session::new();
    session.id = 1001;
    session.name = b"tester".to_vec();
    session.map_id = TEST_MAP;
    session.map_x = 100;
    session.map_y = 100;
    session.in_world = true;
    session.current_event_session = Some(EventSession {
        map_id: i32::from(TEST_MAP),
        eve_no: 1,
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
// Phase 2: Action Class 5 (Gold) & Action Class 7 (Toasts, Sound, EXP)
// ============================================================================

#[test]
fn test_class5_gold_type1_and_type2_deduction() {
    let data = test_synthetic_data();

    // Type 1 (< 1000): convert to points (20:1)
    let action_pts = EveResult {
        result_type: 0,
        result_class: 5,
        parameter_style: 1,
        result_value: 50,
        ..Default::default()
    };
    let mut session = session_with_action(vec![action_pts]);
    session.point = 10;
    let mut out = HandleOutcome::default();
    execute_event_step(&mut session, &data, &mut out);
    assert_eq!(session.point, 1010);
    assert_eq!(out.outgoing[0].frame, build_stat_update(0x26, 1010));

    // Type 1 (>= 1000): add gold
    let action_add_gold = EveResult {
        result_type: 0,
        result_class: 5,
        parameter_style: 1,
        result_value: 2000,
        ..Default::default()
    };
    let mut session = session_with_action(vec![action_add_gold]);
    session.gold = 500;
    let mut out = HandleOutcome::default();
    execute_event_step(&mut session, &data, &mut out);
    assert_eq!(session.gold, 2500);
    assert_eq!(out.outgoing[0].frame, gold_frame(2500));

    // Type 2: deduct gold (saturating_sub)
    let action_sub_gold = EveResult {
        result_type: 0,
        result_class: 5,
        parameter_style: 2,
        result_value: 300,
        ..Default::default()
    };
    let mut session = session_with_action(vec![action_sub_gold]);
    session.gold = 1000;
    let mut out = HandleOutcome::default();
    execute_event_step(&mut session, &data, &mut out);
    assert_eq!(session.gold, 700);
    assert_eq!(out.outgoing[0].frame, gold_frame(700));

    // Type 2: deduct gold more than owned -> saturates at 0 without panic or overflow
    let action_sub_overflow = EveResult {
        result_type: 0,
        result_class: 5,
        parameter_style: 2,
        result_value: 9999,
        ..Default::default()
    };
    let mut session = session_with_action(vec![action_sub_overflow]);
    session.gold = 100;
    let mut out = HandleOutcome::default();
    execute_event_step(&mut session, &data, &mut out);
    assert_eq!(session.gold, 0);
    assert_eq!(out.outgoing[0].frame, gold_frame(0));
}

#[test]
fn test_class7_skill_point_and_stat_point_toasts() {
    let data = test_synthetic_data();

    // Skill point bonus (parameter 1, style 2) -> stat 0x25 + toast 14 17 [pts]
    let action_skill = EveResult {
        result_type: 0,
        result_class: 7,
        parameter: 1,
        parameter_style: 2,
        result_value: 5,
        ..Default::default()
    };
    let mut session = session_with_action(vec![action_skill]);
    session.skill_point = 10;
    let mut out = HandleOutcome::default();
    execute_event_step(&mut session, &data, &mut out);
    assert_eq!(session.skill_point, 15);
    assert_eq!(out.outgoing[0].frame, build_stat_update(0x25, 15));
    // Toast packet 14 17 with 5 points: F4 44 03 00 14 17 05
    assert_eq!(out.outgoing[1].frame, NpcTalkCodec::build_skill_point_toast_hex(5));
    assert_eq!(out.outgoing[1].frame, "F4440300141705");

    // Stat point bonus (parameter 1, style 3) -> stat 0x26 + toast 14 16 [pts]
    let action_stat = EveResult {
        result_type: 0,
        result_class: 7,
        parameter: 1,
        parameter_style: 3,
        result_value: 12,
        ..Default::default()
    };
    let mut session = session_with_action(vec![action_stat]);
    session.point = 50;
    let mut out = HandleOutcome::default();
    execute_event_step(&mut session, &data, &mut out);
    assert_eq!(session.point, 62);
    assert_eq!(out.outgoing[0].frame, build_stat_update(0x26, 62));
    // Toast packet 14 16 with 12 points: F4 44 03 00 14 16 0C
    assert_eq!(out.outgoing[1].frame, NpcTalkCodec::build_stat_point_toast_hex(12));
    assert_eq!(out.outgoing[1].frame, "F444030014160C");

    // EXP grant (parameter 1, style 4) -> session.texp += value + stat 0x24
    let action_exp = EveResult {
        result_type: 0,
        result_class: 7,
        parameter: 1,
        parameter_style: 4,
        result_value: 50_000,
        ..Default::default()
    };
    let mut session = session_with_action(vec![action_exp]);
    session.texp = 10_000;
    let mut out = HandleOutcome::default();
    execute_event_step(&mut session, &data, &mut out);
    assert_eq!(session.texp, 60_000);
    assert_eq!(out.outgoing[0].frame, build_stat_update(0x24, 60_000));
}

// ============================================================================
// Phase 3: MarkDatLoader & Quest Dont Integration
// ============================================================================

#[test]
fn test_mark_dat_loader_decodes_records() {
    let mark_path = data_dir().join("Mark.Dat");
    if !mark_path.exists() {
        eprintln!("Skipping test_mark_dat_loader_decodes_records: Mark.Dat not found");
        return;
    }
    let bytes = std::fs::read(&mark_path).expect("read Mark.Dat");
    let quest_marks = MarkDatLoader::load(&bytes).expect("MarkDatLoader::load");

    // Expect exactly 2,394 quest records loaded
    assert_eq!(quest_marks.len(), 2394, "Mark.Dat must contain 2,394 quests");

    // Verify known decrypted quest mark mappings
    assert_eq!(quest_marks.get(&10001), Some(&0));
    assert_eq!(quest_marks.get(&10002), Some(&0));
    assert_eq!(quest_marks.get(&10004), Some(&1));
    assert_eq!(quest_marks.get(&10005), Some(&32));
    assert_eq!(quest_marks.get(&10006), Some(&210));
    assert_eq!(quest_marks.get(&10007), Some(&2));
    assert_eq!(quest_marks.get(&10009), Some(&3));
}

#[test]
fn test_class2_quest_save_triggers_quest_dont_when_mark_present() {
    let mut data = test_synthetic_data();
    // Quest 10005 maps to mark position 32 in Mark.Dat
    data.quest_marks.insert(10005, 32);
    // Quest 10001 maps to mark position 0
    data.quest_marks.insert(10001, 0);

    // 1. Quest 10005: mark position > 0 -> should record quest_dont and emit Sub 0x05
    let quest_10005 = EveResult {
        result_type: 0,
        result_class: 2,
        parameter: 10005,
        parameter_style: 1,
        result_value: 1,
        ..Default::default()
    };
    let mut session = session_with_action(vec![quest_10005.clone()]);
    let mut out = HandleOutcome::default();
    execute_event_step(&mut session, &data, &mut out);

    assert!(session.quest_tasks.contains_key(&10005));
    assert!(
        session.quest_dont.contains(&32),
        "quest_dont must record mark 32 for quest 10005"
    );
    // Frame emitted for quest_dont: 18 05 [32: 2B LE = 20 00] [flag: 1B = 01]
    let dont_frames: Vec<_> = out
        .outgoing
        .iter()
        .filter(|f| f.frame.contains("1805"))
        .collect();
    assert_eq!(dont_frames.len(), 1);
    assert_eq!(dont_frames[0].frame, "F44405001805200001");

    // 2. Quest 10001: mark position == 0 -> should NOT record quest_dont
    let quest_10001 = EveResult {
        result_type: 0,
        result_class: 2,
        parameter: 10001,
        parameter_style: 1,
        result_value: 1,
        ..Default::default()
    };
    let mut session = session_with_action(vec![quest_10001]);
    let mut out = HandleOutcome::default();
    execute_event_step(&mut session, &data, &mut out);

    assert!(session.quest_tasks.contains_key(&10001));
    assert!(
        !session.quest_dont.contains(&0),
        "mark 0 must not be inserted into quest_dont"
    );
    let dont_frames: Vec<_> = out
        .outgoing
        .iter()
        .filter(|f| f.frame.contains("1805"))
        .collect();
    assert!(dont_frames.is_empty(), "mark 0 must not emit 18 05 frame");

    // 3. Quest 10005 with battle loss (battle_result == 2): should NOT record quest_dont even though mark > 0
    let mut session_loss = session_with_action(vec![quest_10005]);
    if let Some(ref mut ev) = session_loss.current_event_session {
        ev.battle_result = 2; // PlayerLose
    }
    let mut out_loss = HandleOutcome::default();
    execute_event_step(&mut session_loss, &data, &mut out_loss);
    assert!(
        !session_loss.quest_dont.contains(&32),
        "battle loss must not award quest_dont"
    );
    let dont_frames_loss: Vec<_> = out_loss
        .outgoing
        .iter()
        .filter(|f| f.frame.contains("1805"))
        .collect();
    assert!(
        dont_frames_loss.is_empty(),
        "battle loss must not emit 18 05 frame"
    );
}

// ============================================================================
// Phase 4: Party Door Warp
// ============================================================================

#[test]
fn test_party_door_warp_relocates_leader_and_all_members() {
    let data = test_synthetic_data();

    let leader_id = 9001;
    let member1_id = 9002;
    let member2_id = 9003;

    let mut leader_session = session_with_action(vec![EveResult {
        result_type: 2, // Door warp
        parameter: WARP_ID as u16,
        ..Default::default()
    }]);
    leader_session.id = leader_id;
    leader_session.id_leader = leader_id;
    leader_session.id_mem = [member1_id, member2_id, 0, 0];
    leader_session.map_id = TEST_MAP;
    leader_session.map_x = 100;
    leader_session.map_y = 100;

    let member1_session = Session {
        id: member1_id,
        id_leader: leader_id,
        map_id: TEST_MAP,
        map_x: 100,
        map_y: 100,
        in_world: true,
        ..Default::default()
    };
    let member2_session = Session {
        id: member2_id,
        id_leader: leader_id,
        map_id: TEST_MAP,
        map_x: 100,
        map_y: 100,
        in_world: true,
        ..Default::default()
    };

    lock_online_sessions().insert(leader_id, leader_session.clone());
    lock_online_sessions().insert(member1_id, member1_session.clone());
    lock_online_sessions().insert(member2_id, member2_session);

    let mut out = HandleOutcome::default();
    execute_event_step(&mut leader_session, &data, &mut out);

    // 1. Leader relocated in session and registry
    assert_eq!(leader_session.map_id, DEST_MAP);
    assert_eq!(leader_session.map_x, 500);
    assert_eq!(leader_session.map_y, 600);

    // 2. Members relocated in online_sessions() registry
    let reg = lock_online_sessions();
    let m1 = reg.get(&member1_id).expect("member 1 in registry");
    assert_eq!(m1.map_id, DEST_MAP);
    assert_eq!(m1.map_x, 500);
    assert_eq!(m1.map_y, 600);

    let m2 = reg.get(&member2_id).expect("member 2 in registry");
    assert_eq!(m2.map_id, DEST_MAP);
    assert_eq!(m2.map_x, 500);
    assert_eq!(m2.map_y, 600);
    drop(reg);

    // 3. Leader received fade (14 07) and relocate packet
    assert_eq!(out.outgoing[0].frame, "F44402001407");
    assert_eq!(
        out.outgoing[1].frame,
        spawn::build_relocate_packet(leader_id, DEST_MAP, 500, 600, WARP_ID as u8)
    );

    // 4. Direct messages queued for both party members (fade + relocate)
    let m1_msgs: Vec<_> = out
        .direct_messages
        .iter()
        .filter(|(target, _)| *target == member1_id)
        .map(|(_, f)| f.as_str())
        .collect();
    assert_eq!(m1_msgs.len(), 2);
    assert_eq!(m1_msgs[0], "F44402001407");
    assert_eq!(
        m1_msgs[1],
        spawn::build_relocate_packet(member1_id, DEST_MAP, 500, 600, WARP_ID as u8)
    );

    let m2_msgs: Vec<_> = out
        .direct_messages
        .iter()
        .filter(|(target, _)| *target == member2_id)
        .map(|(_, f)| f.as_str())
        .collect();
    assert_eq!(m2_msgs.len(), 2);
    assert_eq!(m2_msgs[0], "F44402001407");
    assert_eq!(
        m2_msgs[1],
        spawn::build_relocate_packet(member2_id, DEST_MAP, 500, 600, WARP_ID as u8)
    );

    // 5. Hide broadcast sent on old map for leader and both members
    assert_eq!(out.map_broadcast.len(), 3);
    assert_eq!(out.map_broadcast[0].subject, leader_id);
    assert_eq!(out.map_broadcast[0].map_id, Some(TEST_MAP));
    assert_eq!(out.map_broadcast[1].subject, member1_id);
    assert_eq!(out.map_broadcast[1].map_id, Some(TEST_MAP));
    assert_eq!(out.map_broadcast[2].subject, member2_id);
    assert_eq!(out.map_broadcast[2].map_id, Some(TEST_MAP));

    // 6. Member 1 completes client map loading and sends Teleport Confirm (0x0C Sub 1)
    let mut member1_conn = ts_dream::server::session::Conn {
        session: member1_session.clone(),
        ..Default::default()
    };
    let mut out_confirm = HandleOutcome::default();
    let service = ts_dream::battle::service::BattleService::default();
    let mut ctx = ts_dream::server::dispatcher::OpcodeCtx {
        conn: &mut member1_conn,
        data: &data,
        service: &service,
        out: &mut out_confirm,
        opcode: 0x0C,
        sub: 1,
        payload: &[],
        decoded: &[],
        env: ts_dream::server::dispatcher::ServerEnv::none(),
    };
    ts_dream::server::handlers::system::handle_teleport_confirm(&mut ctx);

    // Member 1 session and registry are synchronized at DEST_MAP
    assert_eq!(member1_conn.session.map_id, DEST_MAP);
    assert_eq!(member1_conn.session.map_x, 500);
    assert_eq!(member1_conn.session.map_y, 600);
    let reg = lock_online_sessions();
    let m1_after = reg.get(&member1_id).unwrap();
    assert_eq!(m1_after.map_id, DEST_MAP);
    assert_eq!(m1_after.map_x, 500);
    assert_eq!(m1_after.map_y, 600);
    drop(reg);

    // Cleanup
    lock_online_sessions().remove(&leader_id);
    lock_online_sessions().remove(&member1_id);
    lock_online_sessions().remove(&member2_id);
}

// ============================================================================
// Phase 5: Real Trác Quận data integration test
// ============================================================================

#[test]
fn test_real_data_trac_quan_npc_flow() {
    let data_path = data_dir();
    if !data_path.join("eve.emg").exists() || !data_path.join("Mark.Dat").exists() {
        eprintln!("Skipping test_real_data_trac_quan_npc_flow: static data missing");
        return;
    }
    let data = GameData::load(&data_path).expect("load real GameData");

    // Verify Mark.Dat loaded in production GameData
    assert_eq!(data.quest_marks.len(), 2394);

    // Verify Scene 10817 (Trác Quận newbie)
    let scene = data
        .scene_eve_data
        .get(&10817)
        .expect("scene 10817 must exist");

    assert!(scene.npcs.contains_key(&1), "NPC 1 must exist in scene 10817");

    set_eve_events_enabled(true);

    let mut session = Session::new();
    session.id = 12345;
    session.map_id = 10817;
    session.map_x = 590;
    session.map_y = 540;
    session.level = 1;

    let mut rng = DotNetRandom::new(999);

    // NPC 1 is Tân thủ hướng dẫn (template 33001)
    let ev = resolve_npc_event(&session, &data, NpcTrigger::ClickNpc(1), &mut rng);
    assert!(ev.is_some(), "NPC 1 on map 10817 must resolve");
    let ev = ev.unwrap();
    assert_eq!(ev.map_id, 10817);
    assert_eq!(ev.eve_no, 1);
    assert!(!ev.results.is_empty());
}
