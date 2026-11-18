//! Integration tests for Ticket 04:
//! - Shared map granular gating vs exclusive seasonal map gating
//! - Dynamic festival NPC spawn (16 01) on login, map transfer, and Admin Start broadcast
//! - Safe evacuation to savemap (or 12001 fallback) on Admin Stop, including deferred evacuation during battle

use axum::body::Body;
use axum::http::{Request, StatusCode};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tower::ServiceExt;

use ts_dream::battle::runner::{BattleAction, BattleCommand};
use ts_dream::battle::service::BattleService;
use ts_dream::data::loader::GameData;
use ts_dream::data::loaders::eve::{EveDoorPlacement, EveNpcPlacement, SceneEveData};
use ts_dream::data::tables::Warp;
use ts_dream::server::dispatcher::{dispatch, ServerEnv};
use ts_dream::server::handlers::quest::BattleTrigger;
use ts_dream::server::seasonal_festival::{
    DynamicFestivalNpc, FestivalProfile, SeasonalFestivalManager, SharedMapGate,
    FESTIVAL_GIANG_SINH, FESTIVAL_HALLOWEEN, FESTIVAL_TET_NGUYEN_TIEU, FESTIVAL_THAT_TICH,
    FESTIVAL_TRUNG_THU,
};
use ts_dream::server::session::{
    registry_get, registry_register_for_login, registry_update_if_online, Conn, Session,
};
use ts_dream::server::spawn::{build_relocate_packet, npc_appear_frame, sys_msg_frame};
use ts_dream::state::AppState;
use ts_dream::web::app::{router, WebState};
use ts_dream::web::server_control::ServerControl;

use crate::common::test_db::setup_test_db;

#[tokio::test]
async fn test_festival_profile_defaults_and_shared_vs_exclusive_gating() {
    let mgr = SeasonalFestivalManager::default();

    // 1. Default profiles must cover all exclusive maps from SEASONAL_FESTIVAL_AUDIT.md
    let expected_exclusive: Vec<u16> = vec![
        54811, 54812, 54825, 54827, 54828, 54829, 54830, 54831, 12941, 12942, 12943, 57518,
        57531, 57532, 57533, 57534, 57535, 57536, 61051, 61178, 61551, 61567,
    ];
    for map_id in &expected_exclusive {
        assert!(
            mgr.check_door_access(12001, 1, *map_id).is_err(),
            "exclusive map {map_id} must be gated when all festivals are Idle"
        );
        assert!(
            mgr.check_npc_access(*map_id, 1).is_err(),
            "NPC on exclusive map {map_id} must be gated when all festivals are Idle"
        );
    }

    // 2. Shared map negative & positive gating on custom manager covering 12000, 12001, 12005, 12421
    let custom = SeasonalFestivalManager::new(vec![
        FestivalProfile {
            id: FESTIVAL_TET_NGUYEN_TIEU.to_string(),
            name: "Tết Nguyên Tiêu".to_string(),
            lore_record_id: Some(2),
            exp_multiplier_pct: Some(200),
            exclusive_maps: vec![54811, 54812],
            shared_map_gates: vec![SharedMapGate {
                map_id: 12001,
                door_ids: vec![88],
                npc_place_ids: vec![77],
                quest_ids: vec![10786],
            }],
            dynamic_npcs: vec![DynamicFestivalNpc {
                map_id: 12001,
                npc_place_id: 77,
                npc_id: 1825,
            }],
            bonus_drops: vec![],
        },
        FestivalProfile {
            id: FESTIVAL_GIANG_SINH.to_string(),
            name: "Lễ Giáng Sinh".to_string(),
            lore_record_id: Some(11),
            exp_multiplier_pct: None,
            exclusive_maps: vec![54827],
            shared_map_gates: vec![
                SharedMapGate {
                    map_id: 12005,
                    door_ids: vec![91],
                    npc_place_ids: vec![61],
                    quest_ids: vec![10044],
                },
                SharedMapGate {
                    map_id: 12421,
                    door_ids: vec![92],
                    npc_place_ids: vec![62],
                    quest_ids: vec![11542],
                },
            ],
            dynamic_npcs: vec![],
            bonus_drops: vec![],
        },
    ]);

    // When Idle: main-story doors/NPCs/quests on shared maps 12000, 12001, 12005, 12421 MUST succeed
    for shared_map in [12000u16, 12001, 12005, 12421] {
        assert!(
            custom.check_door_access(shared_map, 1, 12002).is_ok(),
            "main-story door on shared map {shared_map} must stay open when Idle"
        );
        assert!(
            custom.check_npc_access(shared_map, 1).is_ok(),
            "main-story NPC on shared map {shared_map} must stay open when Idle"
        );
        assert!(
            custom.check_quest_access(shared_map, 999).is_ok(),
            "main-story quest on shared map {shared_map} must stay open when Idle"
        );
    }

    // When Idle: festival-specific doors/NPCs/quests on shared maps MUST be rejected
    assert!(custom.check_door_access(12001, 88, 0).is_err());
    assert!(custom.check_npc_access(12001, 77).is_err());
    assert!(custom.check_quest_access(12001, 10786).is_err());
    assert!(custom.check_door_access(12005, 91, 0).is_err());
    assert!(custom.check_npc_access(12421, 62).is_err());
    assert!(custom.check_quest_access(12421, 11542).is_err());
    assert!(custom.active_dynamic_npcs_for_map(12001).is_empty());

    // Start Tet Nguyen Tieu -> 12001 gates & 54811 open, 12005/12421 remain gated
    custom
        .start_festival(FESTIVAL_TET_NGUYEN_TIEU, true, None)
        .await
        .expect("start tet_nguyen_tieu");
    assert!(custom.check_door_access(12001, 88, 54811).is_ok());
    assert!(custom.check_npc_access(12001, 77).is_ok());
    assert!(custom.check_quest_access(12001, 10786).is_ok());
    assert_eq!(custom.active_dynamic_npcs_for_map(12001).len(), 1);
    assert_eq!(custom.active_dynamic_npcs_for_map(12001)[0].npc_place_id, 77);
    assert!(custom.check_door_access(12005, 91, 0).is_err());

    // Default manager also activates exclusive maps when started
    mgr.start_festival(FESTIVAL_THAT_TICH, true, None).await.unwrap();
    assert!(mgr.check_door_access(12001, 1, 54825).is_ok());
    mgr.start_festival(FESTIVAL_TRUNG_THU, true, None).await.unwrap();
    assert!(mgr.check_door_access(12001, 1, 61051).is_ok());
    mgr.start_festival(FESTIVAL_HALLOWEEN, true, None).await.unwrap();
    assert!(mgr.check_door_access(12001, 1, 57531).is_ok());
}

fn build_gating_test_data() -> GameData {
    let mut data = GameData::default();
    let mut doors = HashMap::new();
    // Main-story door 1 -> 12002
    doors.insert(
        1,
        EveDoorPlacement {
            id: 1,
            x: 100,
            y: 100,
            events: vec![],
        },
    );
    // Festival door 88 -> 54811
    doors.insert(
        88,
        EveDoorPlacement {
            id: 88,
            x: 300,
            y: 300,
            events: vec![],
        },
    );

    let mut npcs = HashMap::new();
    npcs.insert(
        1,
        EveNpcPlacement {
            id: 1,
            npc_id: 10001,
            x: 100,
            y: 100,
            events: vec![],
            direction: 0,
            close: false,
        },
    );
    npcs.insert(
        77,
        EveNpcPlacement {
            id: 77,
            npc_id: 1825,
            x: 100,
            y: 100,
            events: vec![],
            direction: 0,
            close: false,
        },
    );

    let scene_eve = SceneEveData {
        doors,
        npcs,
        ..Default::default()
    };
    data.scene_eve_data.insert(12001, scene_eve);

    data.warps.insert(
        (12001, 1),
        Warp {
            map1: 12001,
            warpid: 1,
            map2: 12002,
            x: 200,
            y: 200,
        },
    );
    data.warps.insert(
        (12001, 88),
        Warp {
            map1: 12001,
            warpid: 88,
            map2: 54811,
            x: 200,
            y: 200,
        },
    );
    data
}

fn raw_frame(body: &[u8]) -> Vec<u8> {
    let len = body.len() as u16;
    let mut out = vec![0xF4, 0x44, (len & 0xFF) as u8, (len >> 8) as u8];
    out.extend_from_slice(body);
    out
}

#[tokio::test]
async fn test_shared_and_exclusive_map_gating_through_dispatcher() {
    let data = Arc::new(build_gating_test_data());
    let state = Arc::new(RwLock::new(AppState::new(100)));
    let service = Arc::new(BattleService::new(Arc::clone(&data)));
    let control = ServerControl::new(0, Arc::clone(&state), Some(Arc::clone(&data)), None);
    let mut conn = Conn::new();
    conn.session.id = 7001;
    conn.session.map_id = 12001;
    conn.session.map_x = 100;
    conn.session.map_y = 100;

    // 1. ClickNpc (0x14 01) on main-story NPC 1 on 12001 -> NOT rejected with festival sys_msg
    let click_main_npc = raw_frame(&[0x14, 0x01, 0x01, 0x00]);
    let env = ServerEnv {
        pool: None,
        repos: None,
        hub: Some(&control),
        sender: None,
    };
    let out_main = dispatch(&mut conn, &click_main_npc, &data, &service, &env).await;
    assert!(
        !out_main.outgoing.iter().any(|f| f.frame.starts_with("F444") && f.frame.contains("020C")),
        "main-story NPC on shared map 12001 must not trigger festival sys_msg"
    );

    // 2. ClickDoor (0x14 04) on festival door 88 (targets 54811) while Idle -> rejected with sys_msg_frame
    let click_fest_door = raw_frame(&[0x14, 0x04, 88, 0x00]);
    let out_door = dispatch(&mut conn, &click_fest_door, &data, &service, &env).await;
    let expected_err = sys_msg_frame("Lễ hội chưa mở: không thể vào bản đồ này.");
    assert!(
        out_door.outgoing.iter().any(|f| f.frame == expected_err),
        "ClickDoor to exclusive map 54811 while Idle must emit sys_msg_frame, got {:?}",
        out_door.outgoing
    );

    // 3. Talk-Warp (0x14 08) targeting door 88 -> rejected with sys_msg_frame while Idle
    conn.session.idtalking = 88;
    let talk_warp = raw_frame(&[0x14, 0x08, 88, 0x00]);
    let out_warp = dispatch(&mut conn, &talk_warp, &data, &service, &env).await;
    assert!(
        out_warp.outgoing.iter().any(|f| f.frame == expected_err),
        "0x14 08 Talk-Warp to exclusive map 54811 while Idle must emit sys_msg_frame"
    );
    assert_eq!(conn.session.map_id, 12001, "player must not warp while festival is Idle");

    // 4. MeetDoor (0x06 01) near door 88 at (300, 300) -> rejected with sys_msg_frame while Idle
    let meet_door = raw_frame(&[
        0x06,
        0x01,
        0x01,
        (300u16 & 0xFF) as u8,
        (300u16 >> 8) as u8,
        (300u16 & 0xFF) as u8,
        (300u16 >> 8) as u8,
    ]);
    let out_meet = dispatch(&mut conn, &meet_door, &data, &service, &env).await;
    assert!(
        out_meet.outgoing.iter().any(|f| f.frame == expected_err),
        "MeetDoor to exclusive map 54811 while Idle must emit sys_msg_frame"
    );

    // 5. Start festival -> Talk-Warp (0x14 08) to 54811 now succeeds!
    control
        .festivals
        .start_festival(FESTIVAL_TET_NGUYEN_TIEU, true, None)
        .await
        .unwrap();
    let out_warp_active = dispatch(&mut conn, &talk_warp, &data, &service, &env).await;
    assert!(
        !out_warp_active.outgoing.iter().any(|f| f.frame == expected_err),
        "0x14 08 Talk-Warp to 54811 must succeed when festival is Active"
    );
    assert_eq!(conn.session.map_id, 54811, "player should warp to 54811 when Active");
}

#[tokio::test]
async fn test_dynamic_npc_16_01_login_map_enter_and_admin_start_broadcast() {
    let mut raw_data = GameData::default();
    raw_data.npcs.insert(
        10001,
        ts_dream::data::tables::Npc {
            id: 10001,
            name: b"Dummy".to_vec(),
            lv: 1,
            hp: 1,
            ..Default::default()
        },
    );
    let data = Arc::new(raw_data);
    let state = Arc::new(RwLock::new(AppState::new(100)));
    state.write().await.running = true;
    let service = Arc::new(BattleService::new(Arc::clone(&data)));
    let festivals = Arc::new(SeasonalFestivalManager::new(vec![FestivalProfile {
        id: FESTIVAL_TET_NGUYEN_TIEU.to_string(),
        name: "Tết Nguyên Tiêu".to_string(),
        lore_record_id: Some(2),
        exp_multiplier_pct: Some(200),
        exclusive_maps: vec![54811],
        shared_map_gates: vec![],
        dynamic_npcs: vec![DynamicFestivalNpc {
            map_id: 12001,
            npc_place_id: 77,
            npc_id: 1825,
        }],
        bonus_drops: vec![],
    }]));

    let control = Arc::new(
        ServerControl::new(0, Arc::clone(&state), Some(Arc::clone(&data)), None)
            .with_festivals(festivals),
    );
    let expected_16_01 = npc_appear_frame(77);
    assert_eq!(expected_16_01, "F444050016014D0000");

    // 1. Online player 7101 standing on map 12001 when Idle -> receives 16 01 immediately on Admin Start
    let (tx, mut rx) = tokio::sync::mpsc::channel::<String>(16);
    assert!(control.login_register(7101, &tx).await);
    let mut online_sess = Session::default();
    online_sess.id = 7101;
    online_sess.map_id = 12001;
    online_sess.in_world = true;
    registry_register_for_login(&online_sess);

    // Map transfer (0x0C 0x01) while Idle -> must NOT emit 16 01 for dynamic NPC 77
    let mut conn_idle = Conn::new();
    conn_idle.session.id = 7102;
    conn_idle.session.map_id = 12001;
    let warp_confirm = raw_frame(&[0x0C, 0x01]);
    let env = ServerEnv {
        pool: None,
        repos: None,
        hub: Some(&control),
        sender: None,
    };
    let out_idle_enter = dispatch(&mut conn_idle, &warp_confirm, &data, &service, &env).await;
    assert!(
        !out_idle_enter.outgoing.iter().any(|f| f.frame == expected_16_01),
        "Idle festival must not emit dynamic NPC 16 01 on map enter"
    );

    // Admin clicks Start via HTTP router -> player 7101 on map 12001 receives 17 39 AND 16 01 immediately!
    let web_state = WebState {
        app: Arc::clone(&state),
        pool: None,
        data: Some(Arc::clone(&data)),
        server_control: Some(Arc::clone(&control)),
    };
    let resp = router(web_state)
        .oneshot(
            Request::builder()
                .uri("/api/festivals/tet_nguyen_tieu/start")
                .method("POST")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    let mut received = Vec::new();
    while let Ok(frame) = rx.try_recv() {
        received.push(frame);
    }
    assert!(
        received.contains(&expected_16_01),
        "Admin Start must immediately broadcast 16 01 to players on map 12001, got {:?}",
        received
    );

    // 2. New player entering map 12001 (0x0C 0x01) while Active -> receives 16 01
    let out_active_enter = dispatch(&mut conn_idle, &warp_confirm, &data, &service, &env).await;
    assert!(
        out_active_enter.outgoing.iter().any(|f| f.frame == expected_16_01),
        "Active festival must emit dynamic NPC 16 01 on map enter"
    );

    // 3. New player completing login (0x25 0x01) on map 12001 while Active -> receives 16 01
    let mut conn_login = Conn::new();
    conn_login.session.id = 7103;
    conn_login.session.map_id = 12001;
    let login_complete = raw_frame(&[0x25, 0x01]);
    let out_login = dispatch(&mut conn_login, &login_complete, &data, &service, &env).await;
    assert!(
        out_login.outgoing.iter().any(|f| f.frame == expected_16_01),
        "Active festival must emit dynamic NPC 16 01 on login"
    );

    // Clean up global online_sessions
    control.disconnect_player(7101).await;
}

async fn seed_test_character(pool: &ts_dream::db::pool::DbPool, player_id: i64, name: &[u8], map_id: i64) {
    sqlx::query("INSERT INTO characters (playerid, name, mapid) VALUES (?, ?, ?)")
        .bind(player_id)
        .bind(name)
        .bind(map_id)
        .execute(&pool.write)
        .await
        .expect("seed character");
}

#[tokio::test]
async fn test_stop_evacuates_exclusive_map_players_to_savemap_and_persists_sqlite() {
    let db = setup_test_db().await;
    seed_test_character(&db, 7201, b"EvacA", 54811).await;
    seed_test_character(&db, 7202, b"EvacB", 54811).await;

    let mut raw_data = GameData::default();
    raw_data.npcs.insert(
        10001,
        ts_dream::data::tables::Npc {
            id: 10001,
            name: b"Dummy".to_vec(),
            lv: 1,
            hp: 1,
            ..Default::default()
        },
    );
    let data = Arc::new(raw_data);
    let state = Arc::new(RwLock::new(AppState::new(100)));
    state.write().await.running = true;
    let control = Arc::new(ServerControl::new(
        0,
        Arc::clone(&state),
        Some(Arc::clone(&data)),
        Some(db.clone()),
    ));

    let (tx1, mut rx1) = tokio::sync::mpsc::channel::<String>(16);
    let (tx2, mut rx2) = tokio::sync::mpsc::channel::<String>(16);
    assert!(control.login_register(7201, &tx1).await);
    assert!(control.login_register(7202, &tx2).await);

    let mut s1 = Session::default();
    s1.id = 7201;
    s1.db_character_id = 7201;
    s1.map_id = 54811;
    s1.savemap = 12005;
    s1.savemap_x = 320;
    s1.savemap_y = 240;
    s1.in_world = true;
    registry_register_for_login(&s1);

    let mut s2 = Session::default();
    s2.id = 7202;
    s2.db_character_id = 7202;
    s2.map_id = 54811;
    s2.savemap = 0; // fallback -> 12001
    s2.in_world = true;
    registry_register_for_login(&s2);

    let web_state = WebState {
        app: Arc::clone(&state),
        pool: Some(db.clone()),
        data: Some(Arc::clone(&data)),
        server_control: Some(Arc::clone(&control)),
    };

    // Start then Stop tet_nguyen_tieu (owns exclusive map 54811)
    let resp_start = router(web_state.clone())
        .oneshot(
            Request::builder()
                .uri("/api/festivals/tet_nguyen_tieu/start")
                .method("POST")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp_start.status(), StatusCode::OK);
    while rx1.try_recv().is_ok() {}
    while rx2.try_recv().is_ok() {}

    let resp_stop = router(web_state)
        .oneshot(
            Request::builder()
                .uri("/api/festivals/tet_nguyen_tieu/stop")
                .method("POST")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp_stop.status(), StatusCode::OK);

    let mut frames1 = Vec::new();
    while let Ok(f) = rx1.try_recv() {
        frames1.push(f);
    }
    let mut frames2 = Vec::new();
    while let Ok(f) = rx2.try_recv() {
        frames2.push(f);
    }

    assert!(
        frames1.contains(&build_relocate_packet(7201, 12005, 320, 240, 0)),
        "Player 7201 must receive relocate packet to savemap 12005, got {:?}",
        frames1
    );
    assert!(
        frames2.contains(&build_relocate_packet(7202, 12001, 500, 500, 0)),
        "Player 7202 with savemap=0 must receive relocate packet to fallback 12001, got {:?}",
        frames2
    );

    assert_eq!(registry_get(7201).unwrap().map_id, 12005);
    assert_eq!(registry_get(7202).unwrap().map_id, 12001);

    let db_map1: i64 = sqlx::query_scalar("SELECT mapid FROM characters WHERE playerid = 7201")
        .fetch_one(&db.read)
        .await
        .unwrap();
    let db_map2: i64 = sqlx::query_scalar("SELECT mapid FROM characters WHERE playerid = 7202")
        .fetch_one(&db.read)
        .await
        .unwrap();
    assert_eq!(db_map1, 12005);
    assert_eq!(db_map2, 12001);

    control.disconnect_player(7201).await;
    control.disconnect_player(7202).await;
}

#[tokio::test]
async fn test_stop_defers_evacuation_during_battle_and_evacuates_after_battle_ends() {
    let db = setup_test_db().await;
    seed_test_character(&db, 7203, b"EvacBattle", 54811).await;

    let mut raw_data = GameData::default();
    raw_data.npcs.insert(
        10001,
        ts_dream::data::tables::Npc {
            id: 10001,
            name: b"Dummy".to_vec(),
            lv: 1,
            hp: 1,
            ..Default::default()
        },
    );
    let data = Arc::new(raw_data);
    let state = Arc::new(RwLock::new(AppState::new(100)));
    state.write().await.running = true;
    let control = Arc::new(ServerControl::new(
        0,
        Arc::clone(&state),
        Some(Arc::clone(&data)),
        Some(db.clone()),
    ));

    let (tx, mut rx) = tokio::sync::mpsc::channel::<String>(32);
    assert!(control.login_register(7203, &tx).await);

    let mut s = Session::default();
    s.id = 7203;
    s.db_character_id = 7203;
    s.map_id = 54811;
    s.savemap = 12005;
    s.savemap_x = 400;
    s.savemap_y = 300;
    s.hp = 100;
    s.hp_max = 100;
    s.atk = 500;
    s.agi = 200;
    s.in_world = true;
    registry_register_for_login(&s);

    // Start a real battle in BattleService
    let battle_id = control.battle_service.start_teamdef_battle(
        &mut s,
        &BattleTrigger {
            teamdef: vec![1, 10001],
            diahinh: 1,
        },
    );
    assert!(battle_id > 0);
    assert!(s.battle_id > 0);
    registry_update_if_online(&s);

    let web_state = WebState {
        app: Arc::clone(&state),
        pool: Some(db.clone()),
        data: Some(Arc::clone(&data)),
        server_control: Some(Arc::clone(&control)),
    };

    router(web_state.clone())
        .oneshot(
            Request::builder()
                .uri("/api/festivals/tet_nguyen_tieu/start")
                .method("POST")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    while rx.try_recv().is_ok() {}

    // Stop festival while player is in battle -> must NOT yank mid-battle
    let resp_stop = router(web_state)
        .oneshot(
            Request::builder()
                .uri("/api/festivals/tet_nguyen_tieu/stop")
                .method("POST")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp_stop.status(), StatusCode::OK);

    let reg_mid = registry_get(7203).unwrap();
    assert_eq!(reg_mid.map_id, 54811, "must not evacuate mid-battle");
    assert_eq!(reg_mid.pending_evacuate_map, Some(12005));

    // End battle by attacking the 1-HP enemy -> battle_id clears and deferred evacuation executes cleanly
    let cmd_ok = control.battle_service.submit_command(
        &s,
        BattleCommand {
            row: 3,
            col: 2,
            target_row: 0,
            target_col: 2,
            action: BattleAction::NormalAttack,
        },
    );
    assert!(cmd_ok, "battle command must be accepted");

    // Wait up to 2s for async battle turn to finish and persist
    for _ in 0..40 {
        if registry_get(7203).is_some_and(|r| r.battle_id == 0) {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    }

    let reg_after = registry_get(7203).unwrap();
    assert_eq!(reg_after.battle_id, 0);
    assert_eq!(reg_after.pending_evacuate_map, None);
    assert_eq!(
        reg_after.map_id, 12005,
        "player must be evacuated to savemap 12005 after battle ends"
    );

    let db_map: i64 = sqlx::query_scalar("SELECT mapid FROM characters WHERE playerid = 7203")
        .fetch_one(&db.read)
        .await
        .unwrap();
    assert_eq!(db_map, 12005);

    control.disconnect_player(7203).await;
}
