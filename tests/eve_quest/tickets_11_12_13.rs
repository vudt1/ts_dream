//! Integration & parity tests for Tickets 11, 12, and 13:
//! - Ticket 11: Removal of `should_skip_event` (multi-step same-`eve_no` without `cc=12`,
//!   multi-quest NPCs, `AutoChain` with `battle_result == 0`, negative `cc=12`/`cc=2`/guard checks)
//! - Ticket 12: `EveSceneInfo.dest_map_id` decoding on `Data/eve.emg` (`10,336` `rt=2` warps),
//!   `EveSceneInfo` priority over `Warps.txt`, post-battle warp, party DB persistence, negative cases
//! - Ticket 13: `ground_server.mmg` wired into `GameData`, `DiaHinh` on `0x0B 0xFA` for Eve &
//!   random encounters (low byte = `geolBaseAtt` or fallback `112`, high byte = `0`).

use crate::common::test_db::setup_test_db;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};
use ts_dream::battle::rng::DotNetRandom;
use ts_dream::battle::service::BattleService;
use ts_dream::data::loader::GameData;
use ts_dream::data::loaders::{
    EveCondition, EveDataLoader, EveDoorPlacement, EveEncounterPlacement, EveFightData,
    EveFightEnemy, EveNpcPlacement, EveResult, EveSceneInfo, NpcEventData, SceneEveData,
};
use ts_dream::data::tables::Warp;
use ts_dream::db::modern::sqlite::SqliteRepositories;
use ts_dream::db::modern::traits::CharacterSeed;
use ts_dream::db::pool::DbPool;
use ts_dream::eve::auto_chain::{
    AutoChainResult, EveAutoChainEngine, EventPhase, EventSession, EventTriggerKind,
};
use ts_dream::eve::state::PlayerEventState;
use ts_dream::server::dispatcher::{dispatch, HandleOutcome, ServerEnv};
use ts_dream::server::handlers::npc_event::set_eve_events_enabled;
use ts_dream::server::handlers::quest::{resolve_door_warp, resolve_scene_info_warp};
use ts_dream::server::session::{
    lock_online_sessions, registry_get, registry_register_for_login, Conn, Session,
};

fn data_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("Data")
}

fn click_npc_frame(place_id: u16) -> Vec<u8> {
    let [lo, hi] = place_id.to_le_bytes();
    vec![0xF4, 0x44, 0x04, 0x00, 0x14, 0x01, lo, hi]
}

fn click_door_frame(door_id: u16) -> Vec<u8> {
    let [lo, hi] = door_id.to_le_bytes();
    vec![0xF4, 0x44, 0x04, 0x00, 0x14, 0x04, lo, hi]
}

fn move_frame(dir: u8, x: u16, y: u16) -> Vec<u8> {
    let [x0, x1] = x.to_le_bytes();
    let [y0, y1] = y.to_le_bytes();
    vec![0xF4, 0x44, 0x07, 0x00, 0x06, 0x01, dir, x0, x1, y0, y1]
}

fn has_relocate_frame(frames: &[String], player_id: u32, dest_map: u16) -> bool {
    let id_hex = ts_dream::protocol::encoder::le32(player_id);
    let map_hex = ts_dream::protocol::encoder::le16(dest_map);
    let expected_prefix = format!("F4440D000C{id_hex}{map_hex}");
    frames.iter().any(|f| f.starts_with(&expected_prefix))
}

fn outgoing_hexes(out: &HandleOutcome) -> Vec<String> {
    out.outgoing.iter().map(|f| f.frame.clone()).collect()
}

fn direct_hexes(out: &HandleOutcome, target_id: u32) -> Vec<String> {
    out.direct_messages
        .iter()
        .filter(|(id, _)| *id == target_id)
        .map(|(_, f)| f.clone())
        .collect()
}

fn decode_hex(hex: &str) -> Vec<u8> {
    (0..hex.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&hex[i..i + 2], 16).unwrap())
        .collect()
}

/// Extract the 16-bit LE `DiaHinh` from a `0x0B 0xFA` battle-start frame.
/// Wire format: `F4 44 <len_lo> <len_hi> 0B FA <diahinh_lo> <diahinh_hi> ...`
fn extract_0b_fa_diahinh(frame_hex: &str) -> Option<u16> {
    let bytes = decode_hex(frame_hex);
    if bytes.len() >= 8
        && bytes[0] == 0xF4
        && bytes[1] == 0x44
        && bytes[4] == 0x0B
        && bytes[5] == 0xFA
    {
        Some(u16::from_le_bytes([bytes[6], bytes[7]]))
    } else {
        None
    }
}

fn quest_step(session: &Session, quest_id: u16) -> Option<u8> {
    session.quest_tasks.get(&quest_id).map(|&(_, s)| s)
}

async fn seed_account_and_char(
    pool: &DbPool,
    account_id: i64,
    name: &[u8],
    map_id: u16,
    x: u16,
    y: u16,
) -> i64 {
    sqlx::query(
        "INSERT INTO accounts (playerid, pass1, pass2, createdat) VALUES (?, '123456', '123456', 0)",
    )
    .bind(account_id)
    .execute(&pool.write)
    .await
    .expect("insert account");

    let repos = SqliteRepositories::new(pool.clone());
    let seed = CharacterSeed {
        level: 20,
        sex: 1,
        hair: 1,
        element: 1,
        map_id: i64::from(map_id),
        map_x: i64::from(x),
        map_y: i64::from(y),
    };
    repos
        .characters()
        .create(account_id, name, &seed)
        .await
        .expect("create character")
}

// ── Ticket 11 Tests ──────────────────────────────────────────────────────────

#[tokio::test]
async fn ticket11_multi_step_same_eve_no_without_cc12_succeeds() {
    set_eve_events_enabled(true);
    let mut data = GameData::default();
    let mut scene = SceneEveData::default();

    // Same eve_no=1, NO cc=12 condition, two branches by cc=2 (quest 500):
    // - chain 0 (cond_no=2): cc=2 step==2 -> advance quest 500 (+1 => 3)
    // - chain 1 (cond_no=1): cc=2 step==0 -> advance quest 500 (+1 => 1)
    let ev = NpcEventData {
        eve_no: 1,
        conditions: vec![
            EveCondition {
                condition_no: 2,
                condition_class: 2,
                condition_parameter_style: 1,
                condition_parameter: 500,
                condition_value: 2,
                and_num: 1,
                to_result: 1,
                results: vec![EveResult {
                    result_type: 0,
                    result_class: 2,
                    parameter_style: 1,
                    parameter: 500,
                    result_value: 1,
                    ..Default::default()
                }],
                ..Default::default()
            },
            EveCondition {
                condition_no: 1,
                condition_class: 2,
                condition_parameter_style: 1,
                condition_parameter: 500,
                condition_value: 0,
                and_num: 1,
                to_result: 1,
                results: vec![EveResult {
                    result_type: 0,
                    result_class: 2,
                    parameter_style: 1,
                    parameter: 500,
                    result_value: 1,
                    ..Default::default()
                }],
                ..Default::default()
            },
        ],
        ..Default::default()
    };
    scene.npc_events.insert(1, ev);
    scene.npcs.insert(
        1,
        EveNpcPlacement {
            id: 1,
            events: vec![1],
            ..Default::default()
        },
    );
    data.scene_eve_data.insert(9001, scene);

    let data_arc = Arc::new(data);
    let service = BattleService::new(Arc::clone(&data_arc));
    let env = ServerEnv::none();
    let mut conn = Conn {
        session: Session {
            id: 1101,
            map_id: 9001,
            map_x: 0,
            map_y: 0,
            ..Default::default()
        },
        ..Default::default()
    };

    // Click 1: step 0 -> step 1, marks (9001, 1) completed once.
    let _ = dispatch(&mut conn, &click_npc_frame(1), &data_arc, &service, &env).await;
    assert_eq!(quest_step(&conn.session, 500), Some(1));
    assert_eq!(conn.session.completed_eve_counts.get(&(9001, 1)).copied(), Some(1));

    // Player advances intermediate step to 2, then returns to same NPC (same eve_no=1 without cc=12).
    conn.session.quest_tasks.insert(500, (1, 2));
    let _ = dispatch(&mut conn, &click_npc_frame(1), &data_arc, &service, &env).await;
    assert_eq!(
        quest_step(&conn.session, 500),
        Some(3),
        "multi-step quest on same eve_no without cc=12 must advance to step 3"
    );
}

#[tokio::test]
async fn ticket11_npc_with_multiple_independent_quests_later_does_not_lock_earlier() {
    set_eve_events_enabled(true);
    let mut data = GameData::default();
    let mut scene = SceneEveData::default();

    // NPC has [eve_no=1, eve_no=2].
    // eve_no=1 requires quest 600 == 1 (unmet initially).
    // eve_no=2 requires quest 601 == 0 -> sets quest 601 = 1.
    scene.npc_events.insert(
        1,
        NpcEventData {
            eve_no: 1,
            conditions: vec![EveCondition {
                condition_no: 1,
                condition_class: 2,
                condition_parameter_style: 1,
                condition_parameter: 600,
                condition_value: 1,
                and_num: 1,
                to_result: 1,
                results: vec![EveResult {
                    result_type: 0,
                    result_class: 2,
                    parameter_style: 1,
                    parameter: 600,
                    result_value: 1,
                    ..Default::default()
                }],
                ..Default::default()
            }],
            ..Default::default()
        },
    );
    scene.npc_events.insert(
        2,
        NpcEventData {
            eve_no: 2,
            conditions: vec![EveCondition {
                condition_no: 1,
                condition_class: 2,
                condition_parameter_style: 1,
                condition_parameter: 601,
                condition_value: 0,
                and_num: 1,
                to_result: 1,
                results: vec![EveResult {
                    result_type: 0,
                    result_class: 2,
                    parameter_style: 1,
                    parameter: 601,
                    result_value: 1,
                    ..Default::default()
                }],
                ..Default::default()
            }],
            ..Default::default()
        },
    );
    scene.npcs.insert(
        1,
        EveNpcPlacement {
            id: 1,
            events: vec![1, 2],
            ..Default::default()
        },
    );
    data.scene_eve_data.insert(9001, scene);

    let data_arc = Arc::new(data);
    let service = BattleService::new(Arc::clone(&data_arc));
    let env = ServerEnv::none();
    let mut conn = Conn {
        session: Session {
            id: 1102,
            map_id: 9001,
            map_x: 0,
            map_y: 0,
            ..Default::default()
        },
        ..Default::default()
    };

    // Complete later event (eve_no=2) first.
    let _ = dispatch(&mut conn, &click_npc_frame(1), &data_arc, &service, &env).await;
    assert_eq!(quest_step(&conn.session, 601), Some(1));
    assert_eq!(conn.session.completed_eve_counts.get(&(9001, 2)).copied(), Some(1));

    // Now unlock quest 600 == 1 and click NPC again.
    conn.session.quest_tasks.insert(600, (1, 1));
    let _ = dispatch(&mut conn, &click_npc_frame(1), &data_arc, &service, &env).await;
    assert_eq!(
        quest_step(&conn.session, 600),
        Some(2),
        "completing later eve_no=2 must NOT lock out earlier eve_no=1"
    );
}

#[test]
fn ticket11_auto_chain_battle_result_zero_and_same_condition_guard() {
    let mut scene = SceneEveData::default();
    scene.npc_events.insert(
        1,
        NpcEventData {
            eve_no: 1,
            conditions: vec![EveCondition {
                condition_no: 2,
                condition_class: 2,
                condition_parameter_style: 1,
                condition_parameter: 700,
                condition_value: 1,
                and_num: 1,
                to_result: 1,
                results: vec![EveResult {
                    result_type: 1, // Talk
                    result_mean_no: 100,
                    ..Default::default()
                }],
                ..Default::default()
            }],
            ..Default::default()
        },
    );
    scene.npcs.insert(
        1,
        EveNpcPlacement {
            id: 1,
            events: vec![1],
            ..Default::default()
        },
    );

    let mut state = PlayerEventState::default();
    state.mission_steps.insert(700, 1);
    state.completed_eve_counts.insert(1, 1); // already completed once, no cc=12

    // Different matched_condition_no (1 -> 2) with battle_result == 0 must chain!
    let completed_diff = EventSession {
        map_id: 9001,
        eve_no: 1,
        npc_click_id: 1,
        trigger_kind: EventTriggerKind::ClickNpc,
        chain_depth: 0,
        results: vec![],
        planned_effects: vec![],
        current_index: 0,
        phase: EventPhase::Completed,
        last_surface_id: -1,
        last_choice_code: -1,
        battle_result: 0,
        matched_condition_no: 1,
        last_step_sent_ms: 0,
    };
    let mut rng = DotNetRandom::new(1);
    match EveAutoChainEngine::try_auto_chain(&scene, &completed_diff, &state, &mut rng) {
        AutoChainResult::Chained(next) => assert_eq!(next.matched_condition_no, 2),
        AutoChainResult::NoMatch => {
            panic!("expected auto-chain to same eve_no with new condition_no")
        }
    }

    // Negative: identical matched_condition_no (2 -> 2) must be stopped by Guard #1!
    let completed_same = EventSession {
        matched_condition_no: 2,
        last_step_sent_ms: 0,
        ..completed_diff
    };
    assert_eq!(
        EveAutoChainEngine::try_auto_chain(&scene, &completed_same, &state, &mut rng),
        AutoChainResult::NoMatch,
        "Guard #1 (SameCondition) must prevent infinite loop"
    );
}

#[tokio::test]
async fn ticket11_negative_one_time_quest_with_cc12_still_blocked() {
    set_eve_events_enabled(true);
    let mut data = GameData::default();
    let mut scene = SceneEveData::default();
    scene.npc_events.insert(
        1,
        NpcEventData {
            eve_no: 1,
            conditions: vec![EveCondition {
                condition_no: 1,
                condition_class: 12,
                condition_value: 0,
                and_num: 1,
                to_result: 1,
                results: vec![EveResult {
                    result_type: 0,
                    result_class: 2,
                    parameter_style: 1,
                    parameter: 800,
                    result_value: 1,
                    ..Default::default()
                }],
                ..Default::default()
            }],
            ..Default::default()
        },
    );
    scene.npcs.insert(
        1,
        EveNpcPlacement {
            id: 1,
            events: vec![1],
            ..Default::default()
        },
    );
    data.scene_eve_data.insert(9001, scene);

    let data_arc = Arc::new(data);
    let service = BattleService::new(Arc::clone(&data_arc));
    let env = ServerEnv::none();
    let mut conn = Conn {
        session: Session {
            id: 1103,
            map_id: 9001,
            map_x: 0,
            map_y: 0,
            ..Default::default()
        },
        ..Default::default()
    };

    let _ = dispatch(&mut conn, &click_npc_frame(1), &data_arc, &service, &env).await;
    assert_eq!(quest_step(&conn.session, 800), Some(1));

    // Second click must NOT re-grant because cc=12 (completed_eve_counts[1] == 0) is now false.
    let _ = dispatch(&mut conn, &click_npc_frame(1), &data_arc, &service, &env).await;
    assert_eq!(quest_step(&conn.session, 800), Some(1));
}

// ── Ticket 12 Tests ──────────────────────────────────────────────────────────

#[test]
fn ticket12_eve_emg_parses_dest_map_id_for_all_10336_rt2_warps() {
    let bytes = std::fs::read(data_dir().join("eve.emg")).expect("read Data/eve.emg");
    let scenes = EveDataLoader::load(&bytes).expect("parse eve.emg");

    let mut total_rt2 = 0usize;
    let mut resolved = 0usize;
    for scene in scenes.values() {
        for ev in scene.npc_events.values() {
            for cond in &ev.conditions {
                for res in &cond.results {
                    if res.result_type == 2 {
                        total_rt2 += 1;
                        if let Some(info) = scene.scene_infos.get(&res.parameter) {
                            if info.dest_map_id > 0
                                && info.player_appear_x > 0
                                && info.player_appear_y > 0
                            {
                                resolved += 1;
                            }
                        }
                    }
                }
            }
        }
    }
    assert_eq!(total_rt2, 9_653, "expected 9,653 rt=2 results in eve.emg (TTTD)");
    assert_eq!(
        resolved, 9_653,
        "all 9,653 rt=2 results must resolve via EveSceneInfo"

    );
}

#[tokio::test]
async fn ticket12_e2e_rt2_warp_uses_eve_scene_info_and_persists_party() {
    set_eve_events_enabled(true);
    let pool = setup_test_db().await;
    let leader_id = seed_account_and_char(&pool, 12001, b"Leader12", 9001, 100, 100).await as u32;
    let member_id = seed_account_and_char(&pool, 12002, b"Member12", 9001, 100, 100).await as u32;

    let mut data = GameData::default();
    let mut scene = SceneEveData::default();
    // Warp 5 ONLY in EveSceneInfo (NOT in data.warps!): tile (10, 20) -> pixel (190, 390).
    scene.scene_infos.insert(
        5,
        EveSceneInfo {
            eve_no: 5,
            dest_map_id: 12001,
            player_appear_x: 10,
            player_appear_y: 20,
            ..Default::default()
        },
    );
    scene.npc_events.insert(
        1,
        NpcEventData {
            eve_no: 1,
            conditions: vec![EveCondition {
                condition_no: 1,
                and_num: 1,
                to_result: 1,
                results: vec![EveResult {
                    result_type: 2,
                    parameter: 5,
                    ..Default::default()
                }],
                ..Default::default()
            }],
            when_happen: [true, false, false, false],
        },
    );
    scene.doors.insert(
        5,
        EveDoorPlacement {
            id: 5,
            events: vec![1],
            ..Default::default()
        },
    );
    data.scene_eve_data.insert(9001, scene);

    let data_arc = Arc::new(data);
    let service = BattleService::new(Arc::clone(&data_arc));
    let env = ServerEnv::with_pool(&pool);

    let mut leader_conn = Conn {
        session: Session {
            id: leader_id,
            db_character_id: i64::from(leader_id),
            id_leader: leader_id,
            id_mem: [member_id, 0, 0, 0],
            map_id: 9001,
            map_x: 0,
            map_y: 0,
            ..Default::default()
        },
        ..Default::default()
    };
    let member_session = Session {
        id: member_id,
        db_character_id: i64::from(member_id),
        id_leader: leader_id,
        id_mem: [leader_id, 0, 0, 0],
        map_id: 9001,
        map_x: 100,
        map_y: 100,
        ..Default::default()
    };
    registry_register_for_login(&leader_conn.session);
    registry_register_for_login(&member_session);

    let out = dispatch(&mut leader_conn, &click_door_frame(5), &data_arc, &service, &env).await;

    assert_eq!(leader_conn.session.map_id, 12001);
    assert_eq!(leader_conn.session.map_x, 190);
    assert_eq!(leader_conn.session.map_y, 390);
    assert!(has_relocate_frame(
        &outgoing_hexes(&out),
        leader_id as u32,
        12001
    ));
    assert!(has_relocate_frame(
        &direct_hexes(&out, member_id as u32),
        member_id as u32,
        12001
    ));

    let mem_reg = registry_get(member_id).expect("member in registry");
    assert_eq!(
        (mem_reg.map_id, mem_reg.map_x, mem_reg.map_y),
        (12001, 190, 390)
    );

    let repos = SqliteRepositories::new(pool.clone());
    let mut l_db = Session::new();
    assert!(repos
        .sessions()
        .load(i64::from(leader_id), &mut l_db)
        .await
        .unwrap());
    let mut m_db = Session::new();
    assert!(repos
        .sessions()
        .load(i64::from(member_id), &mut m_db)
        .await
        .unwrap());
    assert_eq!((l_db.map_id, l_db.map_x, l_db.map_y), (12001, 190, 390));
    assert_eq!((m_db.map_id, m_db.map_x, m_db.map_y), (12001, 190, 390));

    lock_online_sessions().remove(&leader_id);
    lock_online_sessions().remove(&member_id);
}

#[tokio::test]
async fn ticket12_e2e_rt2_warp_prefers_eve_scene_info_over_conflicting_warps_txt() {
    set_eve_events_enabled(true);
    let mut data = GameData::default();
    let mut scene = SceneEveData::default();
    scene.scene_infos.insert(
        7,
        EveSceneInfo {
            eve_no: 7,
            dest_map_id: 12001,
            player_appear_x: 15,
            player_appear_y: 25,
            ..Default::default()
        },
    );
    scene.npc_events.insert(
        1,
        NpcEventData {
            eve_no: 1,
            conditions: vec![EveCondition {
                condition_no: 1,
                and_num: 1,
                to_result: 1,
                results: vec![EveResult {
                    result_type: 2,
                    parameter: 7,
                    ..Default::default()
                }],
                ..Default::default()
            }],
            when_happen: [true, false, false, false],
        },
    );
    scene.doors.insert(
        7,
        EveDoorPlacement {
            id: 7,
            events: vec![1],
            ..Default::default()
        },
    );
    data.scene_eve_data.insert(9001, scene);
    // Conflicting Warps.txt row on same (9001, 7) -> must NOT be chosen!
    data.warps.insert(
        (9001, 7),
        Warp {
            map1: 9001,
            warpid: 7,
            map2: 9999,
            x: 100,
            y: 100,
        },
    );

    let data_arc = Arc::new(data);
    let service = BattleService::new(Arc::clone(&data_arc));
    let env = ServerEnv::none();
    let mut conn = Conn {
        session: Session {
            id: 12003,
            map_id: 9001,
            map_x: 0,
            map_y: 0,
            ..Default::default()
        },
        ..Default::default()
    };

    let _ = dispatch(&mut conn, &click_door_frame(7), &data_arc, &service, &env).await;
    assert_eq!(
        (conn.session.map_id, conn.session.map_x, conn.session.map_y),
        (12001, 290, 490)
    );
}

#[tokio::test]
async fn ticket12_post_battle_deferred_warp_uses_eve_scene_info() {
    let mut data = GameData::default();
    let mut scene = SceneEveData::default();
    scene.scene_infos.insert(
        9,
        EveSceneInfo {
            eve_no: 9,
            dest_map_id: 12002,
            player_appear_x: 12,
            player_appear_y: 14,
            ..Default::default()
        },
    );
    data.scene_eve_data.insert(9001, scene);

    let mut session = Session {
        id: 12004,
        map_id: 9001,
        map_x: 50,
        map_y: 50,
        ..Default::default()
    };
    let mut out = HandleOutcome::default();
    let resolved = resolve_scene_info_warp(&data, session.map_id, 9)
        .expect("scene_info 9 must resolve");
    let warped = ts_dream::server::handlers::quest::perform_party_warp_locked_with_resolved(
        &mut session,
        resolved,
        None,
        &mut out,
    )
    .await;
    assert!(warped);
    assert_eq!(
        (session.map_id, session.map_x, session.map_y),
        (12002, 230, 270)
    );
}

#[tokio::test]
async fn ticket12_negative_missing_both_or_zero_dest_map_id_skips_safely() {
    set_eve_events_enabled(true);
    let mut data = GameData::default();
    let mut scene = SceneEveData::default();
    // Door 10: EveSceneInfo present with dest_map_id == 0 (even with Warps.txt present, must skip)
    scene.scene_infos.insert(
        10,
        EveSceneInfo {
            eve_no: 10,
            dest_map_id: 0,
            player_appear_x: 10,
            player_appear_y: 10,
            ..Default::default()
        },
    );
    scene.npc_events.insert(
        1,
        NpcEventData {
            eve_no: 1,
            conditions: vec![EveCondition {
                condition_no: 1,
                and_num: 1,
                to_result: 1,
                results: vec![EveResult {
                    result_type: 2,
                    parameter: 10,
                    ..Default::default()
                }],
                ..Default::default()
            }],
            ..Default::default()
        },
    );
    // Door 11: missing from both EveSceneInfo and Warps.txt
    scene.npc_events.insert(
        2,
        NpcEventData {
            eve_no: 2,
            conditions: vec![EveCondition {
                condition_no: 1,
                and_num: 1,
                to_result: 1,
                results: vec![EveResult {
                    result_type: 2,
                    parameter: 11,
                    ..Default::default()
                }],
                ..Default::default()
            }],
            when_happen: [true, false, false, false],
        },
    );
    scene.doors.insert(
        10,
        EveDoorPlacement {
            id: 10,
            events: vec![1],
            ..Default::default()
        },
    );
    scene.doors.insert(
        11,
        EveDoorPlacement {
            id: 11,
            events: vec![2],
            ..Default::default()
        },
    );
    data.scene_eve_data.insert(9001, scene);
    data.warps.insert(
        (9001, 10),
        Warp {
            map1: 9001,
            warpid: 10,
            map2: 9999,
            x: 100,
            y: 100,
        },
    );

    let probe_session = Session {
        map_id: 9001,
        ..Default::default()
    };
    assert!(resolve_scene_info_warp(&data, 9001, 10).is_none());
    assert!(resolve_door_warp(&data, &probe_session, 9001, 10).is_none());
    assert!(resolve_door_warp(&data, &probe_session, 9001, 11).is_none());

    let data_arc = Arc::new(data);
    let service = BattleService::new(Arc::clone(&data_arc));
    let env = ServerEnv::none();
    let mut conn = Conn {
        session: Session {
            id: 12005,
            map_id: 9001,
            map_x: 50,
            map_y: 60,
            ..Default::default()
        },
        ..Default::default()
    };

    let _ = dispatch(&mut conn, &click_door_frame(10), &data_arc, &service, &env).await;
    assert_eq!(
        (conn.session.map_id, conn.session.map_x, conn.session.map_y),
        (9001, 50, 60)
    );

    let _ = dispatch(&mut conn, &click_door_frame(11), &data_arc, &service, &env).await;
    assert_eq!(
        (conn.session.map_id, conn.session.map_x, conn.session.map_y),
        (9001, 50, 60)
    );
}

// ── Ticket 13 Tests ──────────────────────────────────────────────────────────

#[test]
fn ticket13_gamedata_loads_ground_server_mmg_and_map_diahinh() {
    let data = crate::common::shared_game_data();
    assert_eq!(
        data.ground_metas.len(),
        4528,
        "GameData::load must populate all 4528 ground_metas from ground_server.mmg"
    );
    assert!(
        data.ground_terrains.len() > 3000,
        "GameData::load must populate ground_terrains from ground_server.mmg"
    );
    let expected_12001 = i32::from(
        *data
            .ground_terrains
            .get(&12001)
            .expect("map 12001 must have geolBaseAtt in ground_server.mmg"),
    );
    assert_eq!(data.map_diahinh(12001), expected_12001);
    assert!(!data.ground_terrains.contains_key(&65535));
    assert_eq!(
        data.map_diahinh(65535),
        112,
        "missing map must fallback to 112"
    );
}

#[tokio::test]
async fn ticket13_eve_battle_0x0b_fa_uses_ground_diahinh_not_scene_info_background_no() {
    set_eve_events_enabled(true);
    let mut data = GameData::default();
    let mut scene = SceneEveData::default();
    // Poison scene_infos[1].background_no with a Map ID > 255 (12001) to prove it is ignored.
    scene.scene_infos.insert(
        1,
        EveSceneInfo {
            eve_no: 1,
            background_no: 12001,
            ..Default::default()
        },
    );
    scene.fight_datas.insert(
        7,
        EveFightData {
            eve_no: 7,
            fight_type: 34,
            bk_music: 1,
            link_to_result: 1,
            left_enemies: vec![EveFightEnemy {
                no: 1,
                npc_id: 10001,
                location_pos: 2,
                ai: 1,
            }],
            ..Default::default()
        },
    );
    scene.npc_events.insert(
        1,
        NpcEventData {
            eve_no: 1,
            conditions: vec![EveCondition {
                condition_no: 1,
                and_num: 1,
                to_result: 1,
                results: vec![EveResult {
                    result_type: 3,
                    result_mean_no: 7,
                    ..Default::default()
                }],
                ..Default::default()
            }],
            ..Default::default()
        },
    );
    scene.npcs.insert(
        1,
        EveNpcPlacement {
            id: 1,
            events: vec![1],
            ..Default::default()
        },
    );
    data.scene_eve_data.insert(12001, scene);
    data.ground_terrains.insert(12001, 129);

    let data_arc = Arc::new(data);
    let service = BattleService::new(Arc::clone(&data_arc));
    let env = ServerEnv::none();
    let mut conn = Conn {
        session: Session {
            id: 13001,
            map_id: 12001,
            map_x: 0,
            map_y: 0,
            in_world: true,
            ..Default::default()
        },
        ..Default::default()
    };

    let (tx, mut rx) = mpsc::channel::<String>(16);
    service.register_sender(13001, Arc::new(RwLock::new(conn.session.clone())), tx);

    let out = dispatch(&mut conn, &click_npc_frame(1), &data_arc, &service, &env).await;
    let (fight_id, diahinh) = out
        .eve_battle
        .unwrap_or_else(|| panic!("eve_battle triggered; outgoing={:?}", out.outgoing));
    assert_eq!(fight_id, 7);
    assert_eq!(
        diahinh, 129,
        "DiaHinh must come from ground_terrains, not background_no=12001"
    );

    let _bid = service.start_eve_encounter(&mut conn.session, fight_id, diahinh);
    let mut wire_diahinh = None;
    while let Ok(frame) = rx.try_recv() {
        if let Some(d) = extract_0b_fa_diahinh(&frame) {
            wire_diahinh = Some(d);
            break;
        }
    }
    let wire_diahinh = wire_diahinh.expect("0x0B 0xFA frame sent to player channel");
    assert_eq!(wire_diahinh & 0xFF, 129, "low byte must be FField 129");
    assert_eq!(wire_diahinh >> 8, 0, "high byte (BKMusic) must be 0");
}

#[tokio::test]
async fn ticket13_random_encounter_0x0b_fa_uses_ground_diahinh_and_fallback_112() {
    let mut data = GameData::default();
    let mut scene = SceneEveData::default();
    scene.scene_infos.insert(
        1,
        EveSceneInfo {
            eve_no: 1,
            background_no: 12001, // > 255, must be ignored
            ..Default::default()
        },
    );
    scene.fight_datas.insert(
        10,
        EveFightData {
            eve_no: 10,
            fight_type: 1,
            bk_music: 1,
            link_to_result: 1,
            left_enemies: vec![EveFightEnemy {
                no: 1,
                npc_id: 10001,
                location_pos: 2,
                ai: 1,
            }],
            ..Default::default()
        },
    );
    scene.npc_events.insert(
        1,
        NpcEventData {
            eve_no: 1,
            conditions: vec![EveCondition {
                condition_no: 1,
                results: vec![EveResult {
                    result_type: 3,
                    result_mean_no: 10,
                    ..Default::default()
                }],
                ..Default::default()
            }],
            ..Default::default()
        },
    );
    scene.encounters.push(EveEncounterPlacement {
        id: 1,
        start_x: 0,
        start_y: 0,
        end_x: 1000,
        end_y: 1000,
        events: vec![1],
        ..Default::default()
    });
    data.scene_eve_data.insert(12001, scene);
    data.ground_terrains.insert(12001, 109);

    let data_arc = Arc::new(data);
    let service = BattleService::new(Arc::clone(&data_arc));
    let env = ServerEnv::none();
    let mut conn = Conn {
        session: Session {
            id: 13002,
            map_id: 12001,
            map_x: 100,
            map_y: 100,
            encounter_steps: 99,
            encounter_threshold: 1,
            ..Default::default()
        },
        ..Default::default()
    };
    let (tx, mut rx) = mpsc::channel::<String>(16);
    service.register_sender(13002, Arc::new(RwLock::new(conn.session.clone())), tx);

    let _out = dispatch(&mut conn, &move_frame(0, 120, 120), &data_arc, &service, &env).await;
    let mut wire_diahinh = None;
    while let Ok(frame) = rx.try_recv() {
        if let Some(d) = extract_0b_fa_diahinh(&frame) {
            wire_diahinh = Some(d);
            break;
        }
    }
    let wire_diahinh = wire_diahinh.expect("expected 0x0B 0xFA frame for random encounter");
    assert_eq!(wire_diahinh & 0xFF, 109);
    assert_eq!(wire_diahinh >> 8, 0);

    // Fallback 112 when map has no entry in ground_terrains:
    let mut data_fallback = (*data_arc).clone();
    data_fallback.ground_terrains.clear();
    let data_fb_arc = Arc::new(data_fallback);
    let service_fb = BattleService::new(Arc::clone(&data_fb_arc));
    let mut conn_fb = Conn {
        session: Session {
            id: 13003,
            map_id: 12001,
            map_x: 100,
            map_y: 100,
            encounter_steps: 99,
            encounter_threshold: 1,
            ..Default::default()
        },
        ..Default::default()
    };
    let (tx_fb, mut rx_fb) = mpsc::channel::<String>(16);
    service_fb.register_sender(13003, Arc::new(RwLock::new(conn_fb.session.clone())), tx_fb);

    let _out_fb = dispatch(&mut conn_fb, &move_frame(0, 120, 120), &data_fb_arc, &service_fb, &env).await;
    let mut wire_fb = None;
    while let Ok(frame) = rx_fb.try_recv() {
        if let Some(d) = extract_0b_fa_diahinh(&frame) {
            wire_fb = Some(d);
            break;
        }
    }
    let wire_fb = wire_fb.expect("expected 0x0B 0xFA fallback frame");
    assert_eq!(wire_fb, 112);
}
