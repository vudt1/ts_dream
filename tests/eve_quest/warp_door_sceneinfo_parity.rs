//! Integration tests for Ticket 01 (ADR 0010):
//! - Separation of `Door ID` (`0x14 08` / BattleGate) vs `SceneInfo ID` (Eve)
//! - `Eve-first` condition evaluation & fail-closed on `0x14 08`
//! - Explicit `Warps.txt` overrides vs fallback boundary
//! - Consistent `BattleGate` gating in `MeetDoor` & `0x14 08` + post-battle party warp & DB persistence

use crate::common::test_db::setup_test_db;
use sqlx::Row;
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};
use ts_dream::battle::runner::Outcome;
use ts_dream::battle::service::BattleService;
use ts_dream::data::loader::GameData;
use ts_dream::data::loaders::{
    EveCondition, EveDoorPlacement, EveResult, EveSceneInfo, NpcEventData, SceneEveData,
};
use ts_dream::data::tables::{BattleGate, Npc, Warp};
use ts_dream::db::modern::sqlite::SqliteRepositories;
use ts_dream::db::modern::traits::CharacterSeed;
use ts_dream::db::pool::DbPool;
use ts_dream::server::dispatcher::{dispatch, ServerEnv};
use ts_dream::server::handlers::npc_event::set_eve_events_enabled;
use ts_dream::server::session::{
    lock_online_sessions, registry_get, registry_register_for_login, Conn, Session,
};

fn move_frame(x: u16, y: u16) -> Vec<u8> {
    let [x0, x1] = x.to_le_bytes();
    let [y0, y1] = y.to_le_bytes();
    vec![0xF4, 0x44, 0x07, 0x00, 0x06, 0x01, 0x00, x0, x1, y0, y1]
}

async fn seed_char(
    pool: &DbPool,
    account_id: i64,
    name: &[u8],
    map_id: i64,
    x: i64,
    y: i64,
) -> i64 {
    sqlx::query(
        "INSERT INTO accounts (playerid, pass1, pass2, createdat) VALUES (?, '123456', '123456', 0)",
    )
    .bind(account_id)
    .execute(&pool.write)
    .await
    .expect("insert account");

    let repos = SqliteRepositories::new(pool.clone());
    repos
        .characters()
        .create(
            account_id,
            name,
            &CharacterSeed {
                level: 10,
                sex: 1,
                hair: 1,
                element: 1,
                map_id,
                map_x: x,
                map_y: y,
            },
        )
        .await
        .expect("create_character")
}

fn talk_warp_frame(door_id: u16) -> Vec<u8> {
    let [lo, hi] = door_id.to_le_bytes();
    vec![0xF4, 0x44, 0x04, 0x00, 0x14, 0x08, lo, hi]
}

#[tokio::test]
async fn door_id_vs_scene_info_id_0x14_08_resolves_via_door_chain_not_scene_info_key() {
    set_eve_events_enabled(true);
    let mut data = GameData::default();
    let mut scene = SceneEveData::default();

    // Trap: scene_infos[14] points to wrong map 12421 (same numeric ID as Door 14).
    scene.scene_infos.insert(
        14,
        EveSceneInfo {
            eve_no: 14,
            dest_map_id: 12421,
            player_appear_x: 5,
            player_appear_y: 5,
            ..Default::default()
        },
    );
    // Real target for Door 14 is SceneInfo 20 -> map 12441, tile (10, 20) -> pixel (190, 390).
    scene.scene_infos.insert(
        20,
        EveSceneInfo {
            eve_no: 20,
            dest_map_id: 12441,
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
                    parameter: 20,
                    ..Default::default()
                }],
                ..Default::default()
            }],
            when_happen: [false, true, false, false],
        },
    );
    scene.doors.insert(
        14,
        EveDoorPlacement {
            id: 14,
            events: vec![1],
            ..Default::default()
        },
    );
    scene.build_indices();
    data.scene_eve_data.insert(12000, scene);

    let data_arc = Arc::new(data);
    let service = BattleService::new(Arc::clone(&data_arc));
    let env = ServerEnv::none();
    let mut conn = Conn {
        session: Session {
            id: 50001,
            map_id: 12000,
            map_x: 0,
            map_y: 0,
            in_world: true,
            ..Default::default()
        },
        ..Default::default()
    };

    let out = dispatch(&mut conn, &talk_warp_frame(14), &data_arc, &service, &env).await;
    assert_eq!(
        (conn.session.map_id, conn.session.map_x, conn.session.map_y),
        (12441, 190, 390),
        "0x14 08 must resolve Door 14 -> NpcEvent -> rt=2 -> SceneInfo 20, not scene_infos[14]"
    );
    assert!(out.outgoing.iter().any(|f| f.frame == "F44402001407"));
}

#[tokio::test]
async fn conditional_door_0x14_08_evaluates_quest_step_and_fails_closed_without_warps_fallback() {
    set_eve_events_enabled(true);
    let mut data = GameData::default();
    let mut scene = SceneEveData::default();

    scene.scene_infos.insert(
        50,
        EveSceneInfo {
            eve_no: 50,
            dest_map_id: 9002,
            player_appear_x: 10,
            player_appear_y: 10,
            ..Default::default()
        },
    );
    // Door 5 requires quest 100 >= step 2 (condition_class = 2, ops = 5 GreaterThanOrEqual).
    scene.npc_events.insert(
        1,
        NpcEventData {
            eve_no: 1,
            conditions: vec![EveCondition {
                condition_no: 1,
                and_num: 1,
                condition_class: 2,
                condition_parameter: 100,
                condition_ops: 5,
                condition_value: 2,
                to_result: 1,
                results: vec![EveResult {
                    result_type: 2,
                    parameter: 50,
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
    scene.build_indices();
    data.scene_eve_data.insert(9001, scene);

    // Even if Warps.txt has a fallback row, failing condition on a door with rt=2 MUST fail-closed.
    data.warps.insert(
        (9001, 5),
        Warp {
            map1: 9001,
            warpid: 5,
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
            id: 50002,
            map_id: 9001,
            map_x: 50,
            map_y: 50,
            in_world: true,
            ..Default::default()
        },
        ..Default::default()
    };

    // Negative: quest step not met -> refused, stays on 9001, no fallback to 9999.
    let out_fail = dispatch(&mut conn, &talk_warp_frame(5), &data_arc, &service, &env).await;
    assert_eq!(conn.session.map_id, 9001);
    assert!(out_fail.outgoing.iter().any(|f| f.frame == "F44402001408"));

    // Positive: quest 100 at step 2 -> warps to 9002.
    conn.session.quest_tasks.insert(100, (0, 2));
    let out_ok = dispatch(&mut conn, &talk_warp_frame(5), &data_arc, &service, &env).await;
    assert_eq!(
        (conn.session.map_id, conn.session.map_x, conn.session.map_y),
        (9002, 190, 190)
    );
    assert!(out_ok.outgoing.iter().any(|f| f.frame == "F44402001407"));
}

#[tokio::test]
async fn override_warps_win_before_eve_and_fallback_warps_only_when_no_rt2() {
    set_eve_events_enabled(true);
    let mut data = GameData::default();

    // 1. Override (25303, 2) -> 25304 must beat broken eve.emg (35304).
    let mut scene_override = SceneEveData::default();
    scene_override.scene_infos.insert(
        2,
        EveSceneInfo {
            eve_no: 2,
            dest_map_id: 35304,
            player_appear_x: 10,
            player_appear_y: 10,
            ..Default::default()
        },
    );
    scene_override.npc_events.insert(
        1,
        NpcEventData {
            eve_no: 1,
            conditions: vec![EveCondition {
                condition_no: 1,
                and_num: 1,
                to_result: 1,
                results: vec![EveResult {
                    result_type: 2,
                    parameter: 2,
                    ..Default::default()
                }],
                ..Default::default()
            }],
            when_happen: [false, true, false, false],
        },
    );
    scene_override.doors.insert(
        2,
        EveDoorPlacement {
            id: 2,
            events: vec![1],
            ..Default::default()
        },
    );
    scene_override.build_indices();
    data.scene_eve_data.insert(25303, scene_override);
    data.warps.insert(
        (25303, 2),
        Warp {
            map1: 25303,
            warpid: 2,
            map2: 25304,
            x: 170,
            y: 790,
        },
    );

    // 2. Fallback (11901, 2) -> 11902 when Door 2 has only rt=1 (no rt=2).
    let mut scene_fb = SceneEveData::default();
    scene_fb.npc_events.insert(
        1,
        NpcEventData {
            eve_no: 1,
            conditions: vec![EveCondition {
                condition_no: 1,
                and_num: 1,
                to_result: 1,
                results: vec![EveResult {
                    result_type: 1,
                    parameter: 100,
                    ..Default::default()
                }],
                ..Default::default()
            }],
            when_happen: [false, true, false, false],
        },
    );
    scene_fb.doors.insert(
        2,
        EveDoorPlacement {
            id: 2,
            events: vec![1],
            ..Default::default()
        },
    );
    scene_fb.build_indices();
    data.scene_eve_data.insert(11901, scene_fb);
    data.warps.insert(
        (11901, 2),
        Warp {
            map1: 11901,
            warpid: 2,
            map2: 11902,
            x: 670,
            y: 1230,
        },
    );

    let data_arc = Arc::new(data);
    let service = BattleService::new(Arc::clone(&data_arc));
    let env = ServerEnv::none();

    let mut conn1 = Conn {
        session: Session {
            id: 50003,
            map_id: 25303,
            map_x: 0,
            map_y: 0,
            in_world: true,
            ..Default::default()
        },
        ..Default::default()
    };
    let _ = dispatch(&mut conn1, &talk_warp_frame(2), &data_arc, &service, &env).await;
    assert_eq!(
        (conn1.session.map_id, conn1.session.map_x, conn1.session.map_y),
        (25304, 170, 790)
    );

    let mut conn2 = Conn {
        session: Session {
            id: 50004,
            map_id: 11901,
            map_x: 0,
            map_y: 0,
            in_world: true,
            ..Default::default()
        },
        ..Default::default()
    };
    let _ = dispatch(&mut conn2, &talk_warp_frame(2), &data_arc, &service, &env).await;
    assert_eq!(
        (conn2.session.map_id, conn2.session.map_x, conn2.session.map_y),
        (11902, 670, 1230)
    );
}

#[tokio::test]
async fn party_member_0x14_08_is_refused() {
    set_eve_events_enabled(true);
    let mut data = GameData::default();
    data.warps.insert(
        (11901, 2),
        Warp {
            map1: 11901,
            warpid: 2,
            map2: 11902,
            x: 670,
            y: 1230,
        },
    );
    let data_arc = Arc::new(data);
    let service = BattleService::new(Arc::clone(&data_arc));
    let env = ServerEnv::none();

    let mut conn = Conn {
        session: Session {
            id: 50005,
            id_leader: 50000, // following a different leader
            map_id: 11901,
            map_x: 100,
            map_y: 100,
            in_world: true,
            ..Default::default()
        },
        ..Default::default()
    };
    let out = dispatch(&mut conn, &talk_warp_frame(2), &data_arc, &service, &env).await;
    assert_eq!(conn.session.map_id, 11901);
    assert!(out.outgoing.iter().any(|f| f.frame == "F44402001408"));
}

fn battlegate_test_data() -> GameData {
    let mut data = GameData::default();
    data.npcs.insert(
        10175,
        Npc {
            id: 10175,
            name: b"Guard".to_vec(),
            hp: 50,
            lv: 5,
            ..Default::default()
        },
    );
    let mut scene = SceneEveData::default();
    scene.scene_infos.insert(
        2,
        EveSceneInfo {
            eve_no: 2,
            dest_map_id: 49903,
            player_appear_x: 10,
            player_appear_y: 10,
            ..Default::default()
        },
    );
    // Door 2 has MeetDoor rt=2 -> SceneInfo 2 (simulating 49902..49907).
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
                    parameter: 2,
                    ..Default::default()
                }],
                ..Default::default()
            }],
            when_happen: [false, true, false, false],
        },
    );
    scene.doors.insert(
        2,
        EveDoorPlacement {
            id: 2,
            x: 200,
            y: 200,
            events: vec![1],
            ..Default::default()
        },
    );
    scene.build_indices();
    data.scene_eve_data.insert(49902, scene);

    let mut defenders = [0i64; 10];
    defenders[0] = 10175;
    data.battle_gates.insert(
        (49902, 2),
        BattleGate {
            mapid1: 49902,
            warpid: 2,
            diahinh: 365,
            defenders,
        },
    );
    data
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn battlegate_triggers_on_meet_door_and_warps_party_with_db_on_win_without_repeat() {
    set_eve_events_enabled(true);
    let pool = setup_test_db().await;
    let l_id = seed_char(&pool, 60001, b"Leader", 49902, 100, 100).await;
    let m_id = seed_char(&pool, 60002, b"Member", 49902, 100, 100).await;
    let l_u32 = l_id as u32;
    let m_u32 = m_id as u32;

    let data_arc = Arc::new(battlegate_test_data());
    let service = BattleService::new(Arc::clone(&data_arc)).with_pool(pool.clone());

    let mut leader_conn = Conn {
        session: Session {
            id: l_u32,
            db_character_id: l_id,
            id_leader: l_u32,
            id_mem: [m_u32, 0, 0, 0],
            map_id: 49902,
            map_x: 180,
            map_y: 180,
            hp: 100,
            hp_max: 100,
            in_world: true,
            ..Default::default()
        },
        ..Default::default()
    };
    let member_session = Session {
        id: m_u32,
        db_character_id: m_id,
        id_leader: l_u32,
        map_id: 49902,
        map_x: 180,
        map_y: 180,
        hp: 100,
        hp_max: 100,
        in_world: true,
        ..Default::default()
    };

    registry_register_for_login(&leader_conn.session);
    registry_register_for_login(&member_session);
    let (tx_l, mut rx_l) = mpsc::channel(64);
    let (tx_m, mut rx_m) = mpsc::channel(64);
    service.register_sender(l_id, Arc::new(RwLock::new(leader_conn.session.clone())), tx_l.clone());
    service.register_sender(m_id, Arc::new(RwLock::new(member_session.clone())), tx_m.clone());

    let env = ServerEnv::with_pool(&pool);

    // 1. Walk onto Door 2 (200, 200) -> MeetDoor MUST trigger BattleGate, NOT warp straight through.
    let _ = dispatch(&mut leader_conn, &move_frame(200, 200), &data_arc, &service, &env).await;
    assert!(
        leader_conn.session.battle_id > 0,
        "MeetDoor on unpassed BattleGate (49902, 2) must trigger battle instead of warping through"
    );
    assert_eq!(leader_conn.session.map_id, 49902);

    // Sync leader session after dispatch (like server_control does).
    let bid = leader_conn.session.battle_id;
    lock_online_sessions().insert(l_u32, leader_conn.session.clone());
    service.register_sender(l_id, Arc::new(RwLock::new(leader_conn.session.clone())), tx_l.clone());

    // 2. Win the BattleGate battle -> must mark passed & auto-warp leader + member to (49903, 190, 190) + persist DB.
    service.battle_ended(bid, Outcome::PlayerWin);

    let l_reg = registry_get(l_u32).expect("leader in registry");
    let m_reg = registry_get(m_u32).expect("member in registry");
    assert_eq!((l_reg.map_id, l_reg.map_x, l_reg.map_y), (49903, 190, 190));
    assert_eq!((m_reg.map_id, m_reg.map_x, m_reg.map_y), (49903, 190, 190));

    let mut l_frames = Vec::new();
    while let Ok(f) = rx_l.try_recv() {
        l_frames.push(f);
    }
    let mut m_frames = Vec::new();
    while let Ok(f) = rx_m.try_recv() {
        m_frames.push(f);
    }
    assert!(l_frames.iter().any(|f| f == "F44402001407"), "leader must receive 14 07");
    assert!(m_frames.iter().any(|f| f == "F44402001407"), "member must receive 14 07");

    for char_id in [l_id, m_id] {
        let row = sqlx::query("SELECT mapid, mapx, mapy FROM characters WHERE playerid = ?")
            .bind(char_id)
            .fetch_one(&pool.read)
            .await
            .expect("fetch character");
        let mid: i64 = row.get("mapid");
        let mx: i64 = row.get("mapx");
        let my: i64 = row.get("mapy");
        assert_eq!((mid, mx, my), (49903, 190, 190), "DB must persist BattleGate warp for {char_id}");
    }

    // 3. Returning to (49902, 2) in the same session must NOT repeat the BattleGate battle.
    leader_conn.session = l_reg;
    leader_conn.session.map_id = 49902;
    leader_conn.session.map_x = 180;
    leader_conn.session.map_y = 180;
    let _ = dispatch(&mut leader_conn, &talk_warp_frame(2), &data_arc, &service, &env).await;
    assert_eq!(leader_conn.session.battle_id, 0, "passed BattleGate must not re-trigger battle");
    assert_eq!(leader_conn.session.map_id, 49903, "passed BattleGate must warp directly");

    lock_online_sessions().remove(&l_u32);
    lock_online_sessions().remove(&m_u32);
}

#[tokio::test]
async fn battlegate_lose_or_flee_does_not_mark_passed_or_warp() {
    set_eve_events_enabled(true);
    let data_arc = Arc::new(battlegate_test_data());
    let service = BattleService::new(Arc::clone(&data_arc));
    let env = ServerEnv::none();

    let mut conn = Conn {
        session: Session {
            id: 60003,
            map_id: 49902,
            map_x: 180,
            map_y: 180,
            hp: 100,
            hp_max: 100,
            in_world: true,
            ..Default::default()
        },
        ..Default::default()
    };
    let (tx, _rx) = mpsc::channel(32);

    // 1. Trigger BattleGate via 0x14 08 -> PlayerLose.
    let _ = dispatch(&mut conn, &talk_warp_frame(2), &data_arc, &service, &env).await;
    assert!(conn.session.battle_id > 0);
    let bid1 = conn.session.battle_id;
    registry_register_for_login(&conn.session);
    service.register_sender(60003, Arc::new(RwLock::new(conn.session.clone())), tx.clone());
    service.battle_ended(bid1, Outcome::PlayerLose);

    let after_lose = registry_get(60003).expect("session in registry");
    assert_eq!(after_lose.map_id, 49902, "PlayerLose must not warp");
    assert_eq!(after_lose.battle_id, 0);

    // 2. Re-trying 0x14 08 must still trigger the BattleGate battle -> PlayerFled.
    conn.session = after_lose;
    let _ = dispatch(&mut conn, &talk_warp_frame(2), &data_arc, &service, &env).await;
    assert!(conn.session.battle_id > 0, "unpassed BattleGate must re-trigger after loss");
    let bid2 = conn.session.battle_id;
    registry_register_for_login(&conn.session);
    service.register_sender(60003, Arc::new(RwLock::new(conn.session.clone())), tx);
    service.battle_ended(bid2, Outcome::PlayerFled);

    let after_flee = registry_get(60003).expect("session in registry");
    assert_eq!(after_flee.map_id, 49902, "PlayerFled must not warp");

    lock_online_sessions().remove(&60003);
}

#[tokio::test]
async fn production_data_0x14_08_fixes_door_vs_sceneinfo_and_honors_override_fallback_battlegate() {
    set_eve_events_enabled(true);
    let data = crate::common::shared_game_data();
    let service = BattleService::new(Arc::clone(data));
    let env = ServerEnv::none();

    // 1. (12000, 14): Door 14 -> rt=2 -> SceneInfo -> 12441 (whereas scene_infos[14] was 12421).
    let mut c1 = Conn {
        session: Session {
            id: 60010,
            map_id: 12000,
            map_x: 0,
            map_y: 0,
            in_world: true,
            ..Default::default()
        },
        ..Default::default()
    };
    let _ = dispatch(&mut c1, &talk_warp_frame(14), data, &service, &env).await;
    assert_eq!(c1.session.map_id, 12441);

    // 2. (25303, 2): Override in Warps.txt -> 25304 (instead of broken eve.emg 35304).
    let mut c2 = Conn {
        session: Session {
            id: 60011,
            map_id: 25303,
            map_x: 0,
            map_y: 0,
            in_world: true,
            ..Default::default()
        },
        ..Default::default()
    };
    let _ = dispatch(&mut c2, &talk_warp_frame(2), data, &service, &env).await;
    assert_eq!((c2.session.map_id, c2.session.map_x, c2.session.map_y), (25304, 170, 790));

    // 3. (11901, 2): Fallback in Warps.txt (missing rt=2 in eve.emg) -> 11902.
    let mut c3 = Conn {
        session: Session {
            id: 60012,
            map_id: 11901,
            map_x: 0,
            map_y: 0,
            in_world: true,
            ..Default::default()
        },
        ..Default::default()
    };
    let _ = dispatch(&mut c3, &talk_warp_frame(2), data, &service, &env).await;
    assert_eq!((c3.session.map_id, c3.session.map_x, c3.session.map_y), (11902, 670, 1230));

    // 4. (49902, 2): Unpassed BattleGate triggers battle via 0x14 08 and emits 14 09.
    let mut c4 = Conn {
        session: Session {
            id: 60013,
            map_id: 49902,
            map_x: 0,
            map_y: 0,
            hp: 100,
            hp_max: 100,
            in_world: true,
            ..Default::default()
        },
        ..Default::default()
    };
    let out_gate = dispatch(&mut c4, &talk_warp_frame(2), data, &service, &env).await;
    assert!(c4.session.battle_id > 0);
    assert_eq!(c4.session.map_id, 49902);
    assert!(
        out_gate.outgoing.iter().any(|f| f.frame == "F44402001409"),
        "BattleGate triggered from 0x14 08 must emit 14 09 to clear 0xA094, got {:?}",
        out_gate.outgoing
    );

    // 5. (10817, 1): Door 1 has eve_no=7 (MeetDoor dialogue + 2x PlayMovie -> Warp to 10867).
    let door1 = data
        .scene_eve_data
        .get(&10817)
        .and_then(|s| s.doors.get(&1))
        .expect("10817 door 1");
    let mut c5 = Conn {
        session: Session {
            id: 60014,
            map_id: 10817,
            map_x: door1.x as u16,
            map_y: door1.y as u16,
            in_world: true,
            ..Default::default()
        },
        ..Default::default()
    };
    let out_open = dispatch(&mut c5, &talk_warp_frame(1), data, &service, &env).await;
    assert_eq!(c5.session.map_id, 10817, "must NOT warp before dialogue ends");
    assert!(c5.session.current_event_session.is_some(), "EventSession must activate");
    assert!(
        out_open.outgoing.iter().any(|f| f.frame.len() >= 12 && &f.frame[8..12] == "1401"),
        "expected opening 14 01 step, got {:?}",
        out_open.outgoing
    );
    let mut warped = false;
    for _ in 0..9 {
        let out_step = dispatch(
            &mut c5,
            &[0xF4, 0x44, 0x02, 0x00, 0x14, 0x06],
            data,
            &service,
            &env,
        )
        .await;
        if out_step.outgoing.iter().any(|f| f.frame == "F44402001407") {
            warped = true;
            break;
        }
    }
    assert!(warped, "10817 Door 1 must warp after dialogue finishes");
    assert_eq!(c5.session.map_id, 10867);
}
