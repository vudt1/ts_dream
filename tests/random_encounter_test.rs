//! Unit and integration tests for Wild Random Encounter (eve.emg, Opcode 0x06, 0x0B, 0x32).

use std::fs;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;

use ts_dream::battle::service::BattleService;
use ts_dream::data::loader::GameData;
use ts_dream::data::loaders::eve::{
    EveEncounterPlacement, EveFightData, EveFightEnemy, EveResult, NpcEventData, SceneEveData,
};
use ts_dream::data::loaders::EveDataLoader;
use ts_dream::data::tables::Npc;
use ts_dream::server::dispatcher::{HandleOutcome, OpcodeCtx, ServerEnv};
use ts_dream::server::handlers::movement;
use ts_dream::server::session::{online_sessions, Conn, Session};

fn data_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("Data")
}

fn test_ctx<'a>(
    conn: &'a mut Conn,
    data: &'a GameData,
    service: &'a BattleService,
    out: &'a mut HandleOutcome,
    opcode: u8,
    sub: u8,
    payload: &'a [u8],
) -> OpcodeCtx<'a> {
    OpcodeCtx {
        conn,
        data,
        service,
        out,
        opcode,
        sub,
        payload,
        decoded: &[],
        env: ServerEnv::none(),
    }
}

/// 1. Verify loading Section 4 encounters from actual binary eve.emg.
#[test]
fn test_eve_loader_parses_section_4_encounters() {
    let eve_path = data_dir().join("eve.emg");
    if !eve_path.exists() {
        eprintln!("Skipping test: Data/eve.emg not found");
        return;
    }

    let bytes = fs::read(&eve_path).expect("read eve.emg");
    let scenes = EveDataLoader::load(&bytes).expect("parse eve.emg");

    assert!(!scenes.is_empty(), "eve.emg should contain scenes");

    // Check scenes that have wild encounters
    let mut scenes_with_encounters = 0;
    let mut total_encounters = 0;

    for (_scene_id, scene) in &scenes {
        if !scene.encounters.is_empty() {
            scenes_with_encounters += 1;
            total_encounters += scene.encounters.len();

            for enc in &scene.encounters {
                assert!(enc.id > 0, "Encounter ID must be > 0");
                if !enc.full_map {
                    assert!(enc.end_x >= enc.start_x, "end_x >= start_x");
                    assert!(enc.end_y >= enc.start_y, "end_y >= start_y");
                }
            }
        }
    }

    println!(
        "Loaded {} scenes, {} with encounters, {} total encounter zones",
        scenes.len(),
        scenes_with_encounters,
        total_encounters
    );
    assert!(
        scenes_with_encounters > 0,
        "Expected at least one scene with wild encounters in eve.emg"
    );
}

/// 2. Test encounter placement hit-testing (bounding box & full map).
#[test]
fn test_encounter_placement_contains() {
    let bounded = EveEncounterPlacement {
        id: 1,
        events: vec![1],
        start_x: 100,
        start_y: 200,
        end_x: 500,
        end_y: 600,
        full_map: false,
    };

    assert!(bounded.contains(100, 200));
    assert!(bounded.contains(300, 400));
    assert!(bounded.contains(500, 600));
    assert!(!bounded.contains(99, 200));
    assert!(!bounded.contains(100, 199));
    assert!(!bounded.contains(501, 600));
    assert!(!bounded.contains(500, 601));

    let full_map = EveEncounterPlacement {
        id: 2,
        events: vec![1],
        start_x: 0,
        start_y: 0,
        end_x: 65000,
        end_y: 65000,
        full_map: true,
    };

    assert!(full_map.contains(0, 0));
    assert!(full_map.contains(30000, 30000));
    assert!(full_map.contains(65535, 65535));
}

/// 3. Test EveFightEnemy row/col indexing standard (4 rows x 5 cols).
#[test]
fn test_eve_fight_enemy_row_col() {
    // location_pos 0..9:
    // pos 0: row 0, col 0
    // pos 1: row 0, col 1
    // pos 4: row 0, col 4
    // pos 5: row 1, col 0
    // pos 9: row 1, col 4
    let e0 = EveFightEnemy {
        no: 1,
        npc_id: 1001,
        location_pos: 0,
        ai: 0,
    };
    assert_eq!(e0.row(), 0);
    assert_eq!(e0.col(), 0);

    let e4 = EveFightEnemy {
        no: 2,
        npc_id: 1002,
        location_pos: 4,
        ai: 0,
    };
    assert_eq!(e4.row(), 0);
    assert_eq!(e4.col(), 4);

    let e5 = EveFightEnemy {
        no: 3,
        npc_id: 1003,
        location_pos: 5,
        ai: 0,
    };
    assert_eq!(e5.row(), 1);
    assert_eq!(e5.col(), 0);

    let e9 = EveFightEnemy {
        no: 4,
        npc_id: 1004,
        location_pos: 9,
        ai: 0,
    };
    assert_eq!(e9.row(), 1);
    assert_eq!(e9.col(), 4);
}

/// 4. Test Step Counting, Threshold Trigger, and Battle Initiation with Party.
#[tokio::test]
async fn test_movement_triggers_random_encounter_for_party() {
    let mut data = GameData::default();

    // Create a mock enemy NPC
    let enemy_npc = Npc {
        id: 41001,
        name: b"Wild Wolf".to_vec(),
        hp: 200,
        sp: 100,
        lv: 15,
        thuoctinh: 1,
        atk: 50,
        def: 30,
        int1: 10,
        agi: 20,
        reborn: 0,
        ..Default::default()
    };
    data.npcs.insert(41001, enemy_npc);

    // Setup scene 12001 with encounter zone, npc_events, fight_datas, scene_info
    let mut scene = SceneEveData::default();
    scene.encounters.push(EveEncounterPlacement {
        id: 1,
        events: vec![10],
        start_x: 0,
        start_y: 0,
        end_x: 1000,
        end_y: 1000,
        full_map: false,
    });

    let mut event_data = NpcEventData::default();
    event_data.eve_no = 10;
    event_data.conditions.push(ts_dream::data::loaders::EveCondition {
        condition_no: 1,
        condition_class: 0, // unconditional
        and_num: 1,
        results: vec![EveResult {
            result_group_no: 0,
            result_no: 1,
            result_type: 3, // Battle
            result_class: 0,
            parameter: 0,
            parameter_style: 0,
            result_value: 0,
            result_mean_no: 501, // Fight ID
        }],
        ..Default::default()
    });
    scene.npc_events.insert(10, event_data);

    let fight = EveFightData {
        eve_no: 501,
        fight_type: 0,
        link_to_result: 0,
        left_enemies: vec![
            EveFightEnemy {
                no: 1,
                npc_id: 41001,
                location_pos: 2, // Row 0, Col 2
                ai: 0,
            },
            EveFightEnemy {
                no: 2,
                npc_id: 41001,
                location_pos: 7, // Row 1, Col 2
                ai: 0,
            },
        ],
        right_enemies: Vec::new(),
        fight_limit: 0,
    };
    scene.fight_datas.insert(501, fight);

    data.scene_eve_data.insert(12001, scene);

    let service = BattleService::new(Arc::new(data.clone()));

    // Setup Leader (2001) and Member (2002)
    let leader_sess = Arc::new(RwLock::new(Session {
        id: 2001,
        map_id: 12001,
        map_x: 100,
        map_y: 100,
        id_leader: 2001,
        id_mem: [2002, 0, 0, 0],
        encounter_steps: 19,
        encounter_threshold: 20,
        last_battle_end_ms: 0,
        ..Default::default()
    }));

    let member_sess = Arc::new(RwLock::new(Session {
        id: 2002,
        map_id: 12001,
        map_x: 100,
        map_y: 100,
        id_leader: 2001,
        ..Default::default()
    }));

    let _rx_leader = service.register(2001, leader_sess.clone());
    let mut rx_member = service.register(2002, member_sess.clone());

    online_sessions()
        .lock()
        .unwrap()
        .insert(2001, leader_sess.try_read().unwrap().clone());
    online_sessions()
        .lock()
        .unwrap()
        .insert(2002, member_sess.try_read().unwrap().clone());

    let mut leader_conn = Conn {
        session: leader_sess.try_read().unwrap().clone(),
        ..Default::default()
    };
    let mut out = HandleOutcome::default();

    // Move step payload: orient=1, x=110, y=110
    let payload = [1, 110, 0, 110, 0, 0, 0];
    let mut ctx = test_ctx(
        &mut leader_conn,
        &data,
        &service,
        &mut out,
        0x06,
        1,
        &payload,
    );

    movement::handle_move(&mut ctx);

    // 1. Check Leader session now in battle!
    assert!(
        ctx.conn.session.battle_id > 0,
        "Leader should have triggered and entered battle"
    );
    let battle_id = ctx.conn.session.battle_id;

    // 2. Check steps were reset and threshold re-rolled
    assert_eq!(ctx.conn.session.encounter_steps, 0);
    assert!(ctx.conn.session.encounter_threshold >= 15);
    assert!(ctx.conn.session.encounter_threshold <= 31);

    // 3. Check Member session was also put into battle
    let mem_guard = member_sess.try_read().unwrap();
    assert_eq!(
        mem_guard.battle_id, battle_id,
        "Party member must have their battle_id set to leader's battle"
    );
    drop(mem_guard);

    // 4. Verify member received battle start frames (0x0B Sub FA)
    let mut received_battle_start = false;
    while let Ok(frame) = rx_member.try_recv() {
        if frame.contains("0BFA") {
            received_battle_start = true;
            break;
        }
    }
    assert!(
        received_battle_start,
        "Member must receive 0BFA battle start packet"
    );

    online_sessions().lock().unwrap().remove(&2001);
    online_sessions().lock().unwrap().remove(&2002);
}

/// 5. Test 3-second cooldown prevents instant re-triggering.
#[test]
fn test_cooldown_prevents_encounter() {
    let mut data = GameData::default();
    let mut scene = SceneEveData::default();
    scene.encounters.push(EveEncounterPlacement {
        id: 1,
        events: vec![10],
        start_x: 0,
        start_y: 0,
        end_x: 1000,
        end_y: 1000,
        full_map: true,
    });
    data.scene_eve_data.insert(12001, scene);

    let service = BattleService::new(Arc::new(data.clone()));

    let now_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0);

    let mut conn = Conn {
        session: Session {
            id: 3001,
            map_id: 12001,
            map_x: 100,
            map_y: 100,
            encounter_steps: 25,
            encounter_threshold: 20,
            last_battle_end_ms: now_ms, // Battle just ended 0ms ago
            ..Default::default()
        },
        ..Default::default()
    };
    let mut out = HandleOutcome::default();

    let payload = [1, 105, 0, 105, 0, 0, 0];
    let mut ctx = test_ctx(&mut conn, &data, &service, &mut out, 0x06, 1, &payload);

    movement::handle_move(&mut ctx);

    // In cooldown -> should NOT enter battle
    assert_eq!(ctx.conn.session.battle_id, 0);
    // Steps should NOT even increment during cooldown
    assert_eq!(ctx.conn.session.encounter_steps, 25);
}
