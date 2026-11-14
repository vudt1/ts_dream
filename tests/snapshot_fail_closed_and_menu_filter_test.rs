//! Integration tests for Ticket 04:
//! - Full `PlayerEventState` hydration (`homdo`, `follow_npc_ids`, `inn_npc_ids`,
//!   `cart_npc_ids`, `role_count_values`, `completed_eve_counts[(scene_id, eve_no)]`,
//!   `quest_tasks`, `Mark.dat` `quest_dont` projection, `bit_flags`).
//! - Fail-closed condition evaluator (`compare_step` unknown `ops` incl. 142 at
//!   `55100.eve eve=5 cond=14`, unknown `condition_class`, unknown `param`/`p_style`
//!   without `0 == 0 -> true` fallthrough).
//! - `0x14 09` `ChainFilter` (`chain_has_dialog_choice`) on real `Data/eve.emg` +
//!   `Data/Mark.Dat` (`12136` Giản Ung, `12001` Tống Nhân, `FollowPet`, `cc=2 ps=3`).

use std::path::PathBuf;
use ts_dream::battle::service::BattleService;
use ts_dream::data::loader::GameData;
use ts_dream::data::loaders::EveCondition;
use ts_dream::db::modern::model::PetStorageType;
use ts_dream::eve::auto_chain::EventPhase;
use ts_dream::eve::evaluator::{compare_step, evaluate};
use ts_dream::eve::state::PlayerEventState;
use ts_dream::server::dispatcher::{dispatch, HandleOutcome, ServerEnv};
use ts_dream::server::handlers::npc_event::{set_eve_events_enabled, snapshot_state_with_data};
use ts_dream::server::session::{Conn, InventoryItem, PetState};

fn data_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("Data")
}

fn conn_at(map_id: u16, x: u16, y: u16) -> Conn {
    let mut conn = Conn::default();
    conn.session.id = 1001;
    conn.session.name = b"hero".to_vec();
    conn.session.map_id = map_id;
    conn.session.map_x = x;
    conn.session.map_y = y;
    conn.session.level = 30;
    conn.session.in_world = true;
    conn
}

fn click_npc_frame(npc_click_id: u16) -> [u8; 8] {
    let [lo, hi] = npc_click_id.to_le_bytes();
    [0xF4, 0x44, 0x04, 0x00, 0x14, 0x01, lo, hi]
}

fn select_menu_frame(choice: u8) -> [u8; 7] {
    [0xF4, 0x44, 0x03, 0x00, 0x14, 0x09, choice]
}

const CONTINUE_FRAME: [u8; 6] = [0xF4, 0x44, 0x02, 0x00, 0x14, 0x06];

fn talk_step_frames(out: &HandleOutcome) -> Vec<&str> {
    out.outgoing
        .iter()
        .map(|f| f.frame.as_str())
        .filter(|h| h.len() >= 12 && h.starts_with("F444") && &h[8..10] == "14" && (&h[10..12] == "01" || &h[10..12] == "06"))
        .collect()
}

fn step_mean_no(hex: &str) -> u16 {
    // Frame: F4 44 [len:2B] 14 [sub:1B] [15B result] -> last 2B are result_mean_no (LE)
    let bytes = hex::decode(hex).expect("valid hex frame");
    assert!(bytes.len() >= 21, "talk step frame too short: {}", hex);
    u16::from_le_bytes([bytes[bytes.len() - 2], bytes[bytes.len() - 1]])
}

#[test]
fn test_snapshot_full_hydration_pets_mark_dat_bit_flags_and_role_counts() {
    let data = GameData::load(&data_dir()).expect("GameData::load");
    let mut conn = conn_at(12001, 500, 500);

    // 1. homdo (0x17)
    conn.session.homdo.push(InventoryItem {
        slot: 1,
        id: 32012,
        count: 3,
        ..Default::default()
    });
    // 2. Pets across Carried (follow), Hotel (inn), Cart (cart)
    conn.session.pets.push(PetState {
        stt: 1,
        storage_type: PetStorageType::Carried,
        id: 14131,
        ..Default::default()
    });
    conn.session.pets.push(PetState {
        stt: 5,
        storage_type: PetStorageType::Hotel,
        id: 14105,
        ..Default::default()
    });
    conn.session.pets.push(PetState {
        stt: 6,
        storage_type: PetStorageType::Cart,
        id: 14106,
        ..Default::default()
    });
    // 3. role_count_values & bit_flags & quest_tasks & scene-scoped completed_eve_counts
    conn.session.role_count_values.insert(108, 7);
    conn.session.bit_flags.insert(42);
    conn.session.quest_tasks.insert(10021, (1, 2));
    conn.session.completed_eve_counts.insert((12001, 15), 3);
    conn.session.completed_eve_counts.insert((12136, 15), 9);

    // 4. Mark.dat quest_dont projection: pick a real quest_id -> bit_id from Data/Mark.Dat
    let (&quest_id, &bit_id) = data
        .quest_marks
        .iter()
        .find(|(&qid, &bid)| qid > 0 && bid > 0 && qid != bid)
        .expect("Mark.Dat must have quest_id != bit_id entry");
    conn.session.quest_dont.insert(bit_id);

    let state = snapshot_state_with_data(&conn.session, Some(&data), None);

    assert_eq!(state.bag_items.get(&32012), Some(&3));
    assert!(state.follow_npc_ids.contains(&14131));
    assert!(!state.follow_npc_ids.contains(&14105));
    assert!(state.inn_npc_ids.contains(&14105));
    assert!(state.cart_npc_ids.contains(&14106));
    assert_eq!(state.role_count_values.get(&108), Some(&7));
    assert!(state.bit_flags.contains(&42));
    assert_eq!(state.mission_steps.get(&10021), Some(&2));
    assert_eq!(state.completed_eve_counts.get(&15), Some(&3));
    assert_eq!(
        state.mission_flags.get(&i32::from(quest_id)),
        Some(&1),
        "Mark.Dat must project bit_id {bit_id} to quest_id {quest_id}"
    );
    assert!(
        !state.mission_flags.contains_key(&i32::from(bit_id)),
        "raw bit_id must not pollute quest_id key space when GameData is present"
    );

    // Verify ConditionClass=2 pStyle=3 reads the projected quest_dont flag.
    let cond = EveCondition {
        condition_no: 1,
        condition_class: 2,
        condition_parameter: quest_id,
        condition_parameter_style: 3,
        condition_ops: 5, // >=
        condition_value: 1,
        and_num: 1,
        condition_sub_item: 0,
        to_result: 1,
        results: vec![],
    };
    assert!(
        evaluate(&cond, &state),
        "cc=2 pStyle=3 must read quest_dont via Mark.dat projection"
    );
}

#[test]
fn test_fail_closed_evaluator_rejects_unknown_ops_classes_and_zero_fallthrough() {
    let data = GameData::load(&data_dir()).expect("GameData::load");
    let mut state = PlayerEventState::default();
    state.follow_npc_ids.insert(15417);
    state.battle_result = 0;

    // 1. Real Data/eve.emg 55100.eve eve=5 cond=14 has condition_ops = 142 (cc=9, val=15417)
    let scene_55100 = data
        .scene_eve_data
        .get(&55100)
        .expect("55100.eve must be loaded");
    let eve5 = scene_55100.npc_events.get(&5).expect("eve=5 in 55100.eve");
    let cond14 = eve5
        .conditions
        .iter()
        .find(|c| c.condition_no == 14)
        .expect("cond_no=14 in 55100.eve eve=5");
    assert_eq!(cond14.condition_ops, 142);
    assert!(
        !evaluate(cond14, &state),
        "condition_ops=142 at 55100.eve eve=5 cond=14 must fail-closed even when NPC 15417 is carried"
    );
    assert!(!compare_step(0, 142, 0), "compare_step unknown ops must return false");

    // 2. Unimplemented condition_class in {3, 4, 5, 11, 13, 15, 16, 17}
    for cc in [3u8, 4, 5, 11, 13, 15, 16, 17] {
        let cond = EveCondition {
            condition_no: 1,
            condition_class: cc,
            condition_parameter: 0,
            condition_parameter_style: 0,
            condition_ops: 0,
            condition_value: 0,
            and_num: 1,
            condition_sub_item: 0,
        to_result: 1,
            results: vec![],
        };
        assert!(
            !evaluate(&cond, &state),
            "unknown condition_class={cc} must fail-closed"
        );
    }

    // 3. cc=7 unknown param (e.g. 99) with ops=0 val=0 must NOT fall through to 0 == 0 -> true
    let cc7_unknown_param = EveCondition {
        condition_no: 1,
        condition_class: 7,
        condition_parameter: 99,
        condition_parameter_style: 0,
        condition_ops: 0,
        condition_value: 0,
        and_num: 1,
        condition_sub_item: 0,
        to_result: 1,
        results: vec![],
    };
    assert!(!evaluate(&cc7_unknown_param, &state));

    // 4. cc=7 param=6 unknown pStyle (e.g. 9) with ops=0 val=0 must fail-closed
    let cc7_unknown_pstyle = EveCondition {
        condition_no: 1,
        condition_class: 7,
        condition_parameter: 6,
        condition_parameter_style: 9,
        condition_ops: 0,
        condition_value: 0,
        and_num: 1,
        condition_sub_item: 0,
        to_result: 1,
        results: vec![],
    };
    assert!(!evaluate(&cc7_unknown_pstyle, &state));

    // 5. cc=12 unknown param (e.g. 9) with ops=0 val=0 must fail-closed
    let cc12_unknown_param = EveCondition {
        condition_no: 1,
        condition_class: 12,
        condition_parameter: 9,
        condition_parameter_style: 0,
        condition_ops: 0,
        condition_value: 0,
        and_num: 1,
        condition_sub_item: 0,
        to_result: 1,
        results: vec![],
    };
    assert!(!evaluate(&cc12_unknown_param, &state));

    // 6. cc=8 unknown pStyle=0 when battle_result=0 must fail-closed
    let cc8_zero = EveCondition {
        condition_no: 1,
        condition_class: 8,
        condition_parameter: 0,
        condition_parameter_style: 0,
        condition_ops: 0,
        condition_value: 0,
        and_num: 1,
        condition_sub_item: 0,
        to_result: 1,
        results: vec![],
    };
    assert!(!evaluate(&cc8_zero, &state));

    // 7. Unmapped pStyle on cc=2 (e.g. 4), cc=9 (e.g. 2), cc=12 (e.g. 2) must fail-closed
    let cc2_unmapped_pstyle = EveCondition {
        condition_no: 1,
        condition_class: 2,
        condition_parameter: 10021,
        condition_parameter_style: 4,
        condition_ops: 0,
        condition_value: 0,
        and_num: 1,
        condition_sub_item: 0,
        to_result: 1,
        results: vec![],
    };
    assert!(!evaluate(&cc2_unmapped_pstyle, &state));

    let cc9_unmapped_pstyle = EveCondition {
        condition_no: 1,
        condition_class: 9,
        condition_parameter: 0,
        condition_parameter_style: 2,
        condition_ops: 0,
        condition_value: 15417,
        and_num: 1,
        condition_sub_item: 0,
        to_result: 1,
        results: vec![],
    };
    assert!(!evaluate(&cc9_unmapped_pstyle, &state));

    let cc12_unmapped_pstyle = EveCondition {
        condition_no: 1,
        condition_class: 12,
        condition_parameter: 0,
        condition_parameter_style: 2,
        condition_ops: 0,
        condition_value: 0,
        and_num: 1,
        condition_sub_item: 0,
        to_result: 1,
        results: vec![],
    };
    assert!(!evaluate(&cc12_unmapped_pstyle, &state));
}

#[tokio::test]
async fn test_dispatch_gian_ung_12136_menu_branching_and_cancel_filter() {
    let data = GameData::load(&data_dir()).expect("GameData::load");
    let service = BattleService::default();
    let env = ServerEnv::none();
    set_eve_events_enabled(true);

    let npc1 = data
        .scene_eve_data
        .get(&12136)
        .and_then(|s| s.npcs.get(&1))
        .expect("12136 NPC 1 (Giản Ung) must exist");

    // Step 0: Click Giản Ung -> may emit Talk steps before Surface 1
    let mut conn = conn_at(12136, npc1.x as u16, npc1.y as u16);
    let _out = dispatch(&mut conn, &click_npc_frame(1), &data, &service, &env).await;
    for _ in 0..10 {
        if conn
            .session
            .current_event_session
            .as_ref()
            .is_some_and(|e| e.phase == EventPhase::AwaitingChoice)
        {
            break;
        }
        let _ = dispatch(&mut conn, &CONTINUE_FRAME, &data, &service, &env).await;
    }
    let ev = conn
        .session
        .current_event_session
        .as_ref()
        .expect("Giản Ung session must reach AwaitingChoice");
    assert_eq!(ev.phase, EventPhase::AwaitingChoice);
    assert_eq!(ev.last_surface_id, 1);

    // Negative: Cancel (40) on Surface 1 must close safely, NOT re-trigger Surface 1
    let mut conn_cancel = conn_at(12136, npc1.x as u16, npc1.y as u16);
    conn_cancel.session = conn.session.clone();
    let out_cancel = dispatch(
        &mut conn_cancel,
        &select_menu_frame(40),
        &data,
        &service,
        &env,
    )
    .await;
    assert!(
        conn_cancel.session.current_event_session.is_none(),
        "40 cancel must close talk instead of falling back to normal chain"
    );
    assert!(
        out_cancel.outgoing.iter().any(|f| f.frame == "F44402001408"),
        "40 cancel must emit 14 08 EndTalk"
    );

    // Negative: Invalid TSurface choice code (e.g. 5) must close safely
    let mut conn_bad = conn_at(12136, npc1.x as u16, npc1.y as u16);
    conn_bad.session = conn.session.clone();
    let out_bad = dispatch(&mut conn_bad, &select_menu_frame(5), &data, &service, &env).await;
    assert!(conn_bad.session.current_event_session.is_none());
    assert!(out_bad.outgoing.iter().any(|f| f.frame == "F44402001408"));

    // Positive chain: 30 -> Surface 2 -> 32 -> Talk 10628 + Surface 3 -> 32 -> Talk 10628 + Surface 4 -> 31 -> Talk + Skill 14001 + Quest 10049
    let out1 = dispatch(&mut conn, &select_menu_frame(30), &data, &service, &env).await;
    let ev1 = conn.session.current_event_session.as_ref().expect("branch 30");
    assert_eq!(ev1.phase, EventPhase::AwaitingChoice);
    assert_eq!(ev1.last_surface_id, 2);
    assert!(!talk_step_frames(&out1).is_empty());

    let out2 = dispatch(&mut conn, &select_menu_frame(32), &data, &service, &env).await;
    assert_eq!(step_mean_no(talk_step_frames(&out2)[0]), 10628);
    let _ = dispatch(&mut conn, &CONTINUE_FRAME, &data, &service, &env).await;
    assert_eq!(
        conn.session.current_event_session.as_ref().unwrap().last_surface_id,
        3
    );

    let out3 = dispatch(&mut conn, &select_menu_frame(32), &data, &service, &env).await;
    assert_eq!(step_mean_no(talk_step_frames(&out3)[0]), 10628);
    let _ = dispatch(&mut conn, &CONTINUE_FRAME, &data, &service, &env).await;
    assert_eq!(
        conn.session.current_event_session.as_ref().unwrap().last_surface_id,
        4
    );

    let out4 = dispatch(&mut conn, &select_menu_frame(31), &data, &service, &env).await;
    assert_eq!(step_mean_no(talk_step_frames(&out4)[0]), 10600);
}

#[tokio::test]
async fn test_dispatch_tong_nhan_12001_menu_filter_and_si_triet_follow_pet() {
    let data = GameData::load(&data_dir()).expect("GameData::load");
    let service = BattleService::default();
    let env = ServerEnv::none();
    set_eve_events_enabled(true);

    let scene_12001 = data.scene_eve_data.get(&12001).expect("12001.eve loaded");

    // 1. Tống Nhân (12001 NPC 1): Click -> (optional Talk) -> Surface 1 -> 32 matches Talk 10177; 40 closes safely
    let npc1 = scene_12001.npcs.get(&1).expect("12001 NPC 1");
    let mut conn = conn_at(12001, npc1.x as u16, npc1.y as u16);
    let out0 = dispatch(&mut conn, &click_npc_frame(1), &data, &service, &env).await;
    if conn.session.current_event_session.is_none() {
        panic!(
            "12001 NPC 1 did not activate Eve session; npc1={:?}, out={:?}, eve15={:?}",
            npc1,
            out0.outgoing,
            scene_12001.npc_events.get(&15)
        );
    }
    for _ in 0..10 {
        if conn
            .session
            .current_event_session
            .as_ref()
            .is_some_and(|e| e.phase == EventPhase::AwaitingChoice)
        {
            break;
        }
        let _ = dispatch(&mut conn, &CONTINUE_FRAME, &data, &service, &env).await;
    }
    assert_eq!(
        conn.session.current_event_session.as_ref().unwrap().last_surface_id,
        1
    );

    let mut conn_cancel = conn_at(12001, npc1.x as u16, npc1.y as u16);
    conn_cancel.session = conn.session.clone();
    let out_cancel = dispatch(
        &mut conn_cancel,
        &select_menu_frame(40),
        &data,
        &service,
        &env,
    )
    .await;
    assert!(conn_cancel.session.current_event_session.is_none());
    assert!(out_cancel.outgoing.iter().any(|f| f.frame == "F44402001408"));

    let out_ok = dispatch(&mut conn, &select_menu_frame(32), &data, &service, &env).await;
    let steps = talk_step_frames(&out_ok);
    assert!(!steps.is_empty(), "Tống Nhân choice 32 must emit Talk step");

    // 1b. cc=7 param=5 p_style=1 (carried full >= 4) on 12001 Tống Nhân:
    // With 4 carried pets, clicking Tống Nhân immediately matches cond_no=4 (Talk 10563)
    // instead of opening Surface 1 (which requires p_style=2, < 4 carried pets).
    let mut conn_full = conn_at(12001, npc1.x as u16, npc1.y as u16);
    for stt in 1..=4 {
        conn_full.session.pets.push(PetState {
            stt,
            storage_type: PetStorageType::Carried,
            id: 14100 + u16::from(stt),
            ..Default::default()
        });
    }
    let out_full = dispatch(&mut conn_full, &click_npc_frame(1), &data, &service, &env).await;
    let full_steps = talk_step_frames(&out_full);
    assert!(
        !full_steps.is_empty(),
        "Tống Nhân with 4 carried pets must emit Talk 10563"
    );
    assert_eq!(
        step_mean_no(full_steps[0]),
        10563,
        "cc=7 param=5 p_style=1 (>= 4 carried pets) must select Talk 10563"
    );
    assert_ne!(
        conn_full
            .session
            .current_event_session
            .as_ref()
            .map(|e| e.phase),
        Some(EventPhase::AwaitingChoice),
        "full carried-pet branch must not open Surface 1"
    );

    // 2. FollowPet condition on 12001 NPC 24 (Sĩ Triết):
    // Verify FollowPet condition (cc=9) in real Data/eve.emg activates the FollowPet 14131 branch
    // only when pet 14131 is carried in follow slots.
    let npc24 = scene_12001.npcs.get(&24).expect("12001 NPC 24");
    let mut conn_no_pet = conn_at(12001, npc24.x as u16, npc24.y as u16);
    conn_no_pet.session.quest_tasks.insert(10544, (1, 1));
    let out_no_pet = dispatch(&mut conn_no_pet, &click_npc_frame(24), &data, &service, &env).await;
    let cond_no_pet = conn_no_pet
        .session
        .current_event_session
        .as_ref()
        .map(|ev| (ev.eve_no, ev.matched_condition_no));

    let mut conn_pet = conn_at(12001, npc24.x as u16, npc24.y as u16);
    conn_pet.session.quest_tasks.insert(10544, (1, 1));
    conn_pet.session.pets.push(PetState {
        stt: 1,
        storage_type: PetStorageType::Carried,
        id: 14131,
        ..Default::default()
    });
    let out_with_pet = dispatch(&mut conn_pet, &click_npc_frame(24), &data, &service, &env).await;
    let cond_with_pet = conn_pet
        .session
        .current_event_session
        .as_ref()
        .map(|ev| (ev.eve_no, ev.matched_condition_no));
    assert!(
        cond_with_pet.is_some() && cond_with_pet != cond_no_pet,
        "FollowPet 14131 must select a distinct branch (without={cond_no_pet:?}, with={cond_with_pet:?}, out_no={:?}, out_with={:?})",
        out_no_pet.outgoing,
        out_with_pet.outgoing
    );
    assert!(
        !talk_step_frames(&out_with_pet).is_empty()
            || out_with_pet.outgoing.iter().any(|f| f.frame == "F44402000602"),
        "with FollowPet 14131, NPC 24 FollowPet branch must match"
    );
}
