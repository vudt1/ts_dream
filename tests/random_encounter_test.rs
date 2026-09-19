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
use ts_dream::server::session::{lock_online_sessions, Conn, Session};

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

    lock_online_sessions().insert(2001, leader_sess.try_read().unwrap().clone());
    lock_online_sessions().insert(2002, member_sess.try_read().unwrap().clone());

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

    lock_online_sessions().remove(&2001);
    lock_online_sessions().remove(&2002);
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

/// 6. Test member session lock concurrency: when member session is briefly locked by another task,
/// start_encounter_battle retries and successfully brings member into battle.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_party_member_concurrency_lock_retry() {
    let mut data = GameData::default();
    let enemy_npc = Npc {
        id: 41001,
        name: b"Wolf".to_vec(),
        hp: 200,
        sp: 100,
        lv: 15,
        ..Default::default()
    };
    data.npcs.insert(41001, enemy_npc);

    let fight = EveFightData {
        eve_no: 502,
        fight_type: 0,
        link_to_result: 0,
        left_enemies: vec![EveFightEnemy {
            no: 1,
            npc_id: 41001,
            location_pos: 2,
            ai: 0,
        }],
        right_enemies: Vec::new(),
        fight_limit: 0,
    };

    let service = BattleService::new(Arc::new(data.clone()));

    let leader_sess = Arc::new(RwLock::new(Session {
        id: 4001,
        map_id: 12001,
        id_leader: 4001,
        id_mem: [4002, 0, 0, 0],
        ..Default::default()
    }));
    let member_sess = Arc::new(RwLock::new(Session {
        id: 4002,
        map_id: 12001,
        id_leader: 4001,
        ..Default::default()
    }));

    let _rx_leader = service.register(4001, leader_sess.clone());
    let mut rx_member = service.register(4002, member_sess.clone());

    lock_online_sessions().insert(4001, leader_sess.try_read().unwrap().clone());
    lock_online_sessions().insert(4002, member_sess.try_read().unwrap().clone());

    // Spawn a task that holds the member's write lock for 15ms (simulating concurrent async op)
    let mem_clone = member_sess.clone();
    tokio::spawn(async move {
        let _guard = mem_clone.write().await;
        tokio::time::sleep(std::time::Duration::from_millis(15)).await;
    });

    // Give the spawned task a few microseconds to acquire the lock first
    tokio::time::sleep(std::time::Duration::from_millis(2)).await;

    let mut leader_guard = leader_sess.write().await;
    let battle_id = service.start_encounter_battle(&mut leader_guard, &fight, 112);
    assert!(battle_id > 0);
    assert_eq!(leader_guard.battle_id, battle_id);
    drop(leader_guard);

    // Verify member session was successfully locked and joined after retry
    let mem_guard = member_sess.read().await;
    assert_eq!(mem_guard.battle_id, battle_id);
    drop(mem_guard);

    // Verify member received battle start frame (0x0B Sub FA)
    let mut received_battle_start = false;
    while let Ok(frame) = rx_member.try_recv() {
        if frame.contains("0BFA") {
            received_battle_start = true;
            break;
        }
    }
    assert!(received_battle_start, "Member must receive 0BFA frame despite initial lock contention");

    // Verify online_sessions() synchronized
    assert_eq!(
        lock_online_sessions().get(&4002).unwrap().battle_id,
        battle_id
    );

    lock_online_sessions().remove(&4001);
    lock_online_sessions().remove(&4002);
}

/// 7. Test Flee / Escape synchronization:
/// - When player flees, battle state is cleaned up
/// - last_battle_end_ms is set, encounter_steps is reset to 0
/// - 3-second cooldown immunity prevents immediate re-encounter
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_player_flee_synchronization_and_cooldown() {
    let mut data = GameData::default();
    let enemy_npc = Npc {
        id: 41001,
        name: b"Wolf".to_vec(),
        hp: 200,
        sp: 100,
        lv: 15,
        ..Default::default()
    };
    data.npcs.insert(41001, enemy_npc);
    data.skills.insert(
        10000,
        ts_dream::data::tables::Skill {
            id: 10000,
            name: "Tấn Công".to_string(),
            skill_type: 1,
            ..Default::default()
        },
    );
    data.skills.insert(
        14002,
        ts_dream::data::tables::Skill {
            id: 14002,
            name: "Đào Tẩu".to_string(),
            skill_type: 12,
            ..Default::default()
        },
    );
    data.skills.insert(
        14003,
        ts_dream::data::tables::Skill {
            id: 14003,
            name: "Đào Tẩu".to_string(),
            skill_type: 12,
            ..Default::default()
        },
    );

    let fight = EveFightData {
        eve_no: 503,
        fight_type: 0,
        link_to_result: 0,
        left_enemies: vec![EveFightEnemy {
            no: 1,
            npc_id: 41001,
            location_pos: 2,
            ai: 0,
        }],
        right_enemies: Vec::new(),
        fight_limit: 0,
    };

    let mut service = BattleService::new(Arc::new(data.clone()));
    service.set_input_timeout(std::time::Duration::from_millis(50));

    let leader_sess = Arc::new(RwLock::new(Session {
        id: 5001,
        map_id: 12001,
        id_leader: 5001,
        encounter_steps: 25,
        encounter_threshold: 20,
        ..Default::default()
    }));
    let member_sess = Arc::new(RwLock::new(Session {
        id: 5002,
        map_id: 12001,
        id_leader: 5001,
        encounter_steps: 15,
        encounter_threshold: 20,
        ..Default::default()
    }));

    let mut rx_leader = service.register(5001, leader_sess.clone());
    let _rx_member = service.register(5002, member_sess.clone());

    lock_online_sessions().insert(5001, leader_sess.try_read().unwrap().clone());
    lock_online_sessions().insert(5002, member_sess.try_read().unwrap().clone());

    let mut leader_guard = leader_sess.write().await;
    leader_guard.id_mem = [5002, 0, 0, 0];
    let battle_id = service.start_encounter_battle(&mut leader_guard, &fight, 112);
    assert!(battle_id > 0);
    drop(leader_guard);

    // Send flee command (skill 14002) for leader (cell 3, 2)
    let cmd_leader = ts_dream::battle::runner::BattleCommand {
        row: 3,
        col: 2,
        skill_id: 14002, // Flee skill
        skill_lv: 1,
        row_attack: 0,
        col_attack: 2,
        use_item: 0,
    };
    let leader_snap = leader_sess.read().await.clone();
    assert!(service.submit_command(&leader_snap, cmd_leader));

    // Send normal attack command for member (cell 3, 1)
    let cmd_member = ts_dream::battle::runner::BattleCommand {
        row: 3,
        col: 1,
        skill_id: 10000,
        skill_lv: 1,
        row_attack: 0,
        col_attack: 2,
        use_item: 0,
    };
    let member_snap = member_sess.read().await.clone();
    assert!(service.submit_command(&member_snap, cmd_member));

    // Wait briefly for battle task to run the turn and finish with PlayerFled
    let deadline = tokio::time::Instant::now() + std::time::Duration::from_millis(1000);
    loop {
        let sess = leader_sess.read().await;
        if sess.battle_id == 0 {
            break;
        }
        drop(sess);
        if tokio::time::Instant::now() > deadline {
            panic!("Battle did not end within timeout");
        }
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
    }

    // Verify leader state:
    let leader_after = leader_sess.read().await;
    assert_eq!(leader_after.battle_id, 0);
    assert!(leader_after.last_battle_end_ms > 0);
    assert_eq!(leader_after.encounter_steps, 0);
    drop(leader_after);

    // Verify member state was also cleaned up:
    let member_after = member_sess.read().await;
    assert_eq!(member_after.battle_id, 0);
    assert!(member_after.last_battle_end_ms > 0);
    assert_eq!(member_after.encounter_steps, 0);
    drop(member_after);

    // Verify global online_sessions() synchronized:
    let online_map = lock_online_sessions();
    assert_eq!(online_map.get(&5001).unwrap().battle_id, 0);
    assert!(online_map.get(&5001).unwrap().last_battle_end_ms > 0);
    assert_eq!(online_map.get(&5001).unwrap().encounter_steps, 0);
    assert_eq!(online_map.get(&5002).unwrap().battle_id, 0);
    assert!(online_map.get(&5002).unwrap().last_battle_end_ms > 0);
    assert_eq!(online_map.get(&5002).unwrap().encounter_steps, 0);
    drop(online_map);

    // Verify leader received exit move frame (0x05 0x04)
    let mut received_exit_move = false;
    while let Ok(frame) = rx_leader.try_recv() {
        if frame.contains("0504") {
            received_exit_move = true;
            break;
        }
    }
    assert!(received_exit_move, "Leader must receive 0504 battle exit frame");

    lock_online_sessions().remove(&5001);
    lock_online_sessions().remove(&5002);
}

/// 8. Test party member individual flee: a party member flees while the leader remains in combat.
/// The fleeing member's state is reset and synchronized, while the leader's battle continues.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_party_member_individual_flee() {
    let mut data = GameData::default();
    let enemy_npc = Npc {
        id: 41001,
        name: b"Wolf".to_vec(),
        hp: 500, // High HP so leader doesn't finish combat in turn 1
        sp: 100,
        lv: 15,
        ..Default::default()
    };
    data.npcs.insert(41001, enemy_npc);
    data.skills.insert(
        10000,
        ts_dream::data::tables::Skill {
            id: 10000,
            name: "Tấn Công".to_string(),
            skill_type: 1,
            ..Default::default()
        },
    );
    data.skills.insert(
        14002,
        ts_dream::data::tables::Skill {
            id: 14002,
            name: "Đào Tẩu".to_string(),
            skill_type: 12,
            ..Default::default()
        },
    );

    let fight = EveFightData {
        eve_no: 504,
        fight_type: 0,
        link_to_result: 0,
        left_enemies: vec![EveFightEnemy {
            no: 1,
            npc_id: 41001,
            location_pos: 2,
            ai: 0,
        }],
        right_enemies: Vec::new(),
        fight_limit: 0,
    };

    let mut service = BattleService::new(Arc::new(data.clone()));
    service.set_input_timeout(std::time::Duration::from_millis(50));

    let leader_sess = Arc::new(RwLock::new(Session {
        id: 6001,
        map_id: 12001,
        id_leader: 6001,
        encounter_steps: 25,
        encounter_threshold: 20,
        ..Default::default()
    }));
    let member_sess = Arc::new(RwLock::new(Session {
        id: 6002,
        map_id: 12001,
        id_leader: 6001,
        encounter_steps: 15,
        encounter_threshold: 20,
        ..Default::default()
    }));

    let _rx_leader = service.register(6001, leader_sess.clone());
    let mut rx_member = service.register(6002, member_sess.clone());

    lock_online_sessions().insert(6001, leader_sess.try_read().unwrap().clone());
    lock_online_sessions().insert(6002, member_sess.try_read().unwrap().clone());

    let mut leader_guard = leader_sess.write().await;
    leader_guard.id_mem = [6002, 0, 0, 0];
    let battle_id = service.start_encounter_battle(&mut leader_guard, &fight, 112);
    assert!(battle_id > 0);
    drop(leader_guard);

    // Verify both are initially marked in battle
    assert_eq!(leader_sess.read().await.battle_id, battle_id);
    assert_eq!(member_sess.read().await.battle_id, battle_id);

    // Member (cell 3, 1) submits Flee command (skill 14002)
    let cmd_member = ts_dream::battle::runner::BattleCommand {
        row: 3,
        col: 1,
        skill_id: 14002,
        skill_lv: 1,
        row_attack: 0,
        col_attack: 2,
        use_item: 0,
    };
    let member_snap = member_sess.read().await.clone();
    assert!(service.submit_command(&member_snap, cmd_member));

    // Leader (cell 3, 2) submits normal Attack command (skill 10000)
    let cmd_leader = ts_dream::battle::runner::BattleCommand {
        row: 3,
        col: 2,
        skill_id: 10000,
        skill_lv: 1,
        row_attack: 0,
        col_attack: 2,
        use_item: 0,
    };
    let leader_snap = leader_sess.read().await.clone();
    assert!(service.submit_command(&leader_snap, cmd_leader));

    // Wait for member to successfully flee
    let deadline = tokio::time::Instant::now() + std::time::Duration::from_millis(1000);
    loop {
        let mem = member_sess.read().await;
        if mem.battle_id == 0 {
            break;
        }
        drop(mem);
        if tokio::time::Instant::now() > deadline {
            panic!("Member did not flee within timeout");
        }
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
    }

    // 1. Member session verification: battle_id is 0, cooldown timestamp recorded, steps reset
    let member_after = member_sess.read().await;
    assert_eq!(member_after.battle_id, 0);
    assert!(member_after.last_battle_end_ms > 0);
    assert_eq!(member_after.encounter_steps, 0);
    drop(member_after);

    // 2. Leader session verification: still in combat!
    let leader_after = leader_sess.read().await;
    assert_eq!(
        leader_after.battle_id, battle_id,
        "Leader should still remain in battle after member flees"
    );
    drop(leader_after);

    // 3. online_sessions() synchronization check
    let online_map = lock_online_sessions();
    let mem_online = online_map.get(&6002).unwrap();
    assert_eq!(mem_online.battle_id, 0);
    assert!(mem_online.last_battle_end_ms > 0);
    assert_eq!(mem_online.encounter_steps, 0);

    let leader_online = online_map.get(&6001).unwrap();
    assert_eq!(
        leader_online.battle_id, battle_id,
        "Leader's online_sessions record must still reflect ongoing battle"
    );
    drop(online_map);

    // 4. Verify member received battle exit packets
    let mut received_exit_move = false;
    let mut received_exit_talk = false;
    while let Ok(frame) = rx_member.try_recv() {
        if frame.contains("0504") {
            received_exit_move = true;
        }
        if frame.contains("1408") {
            received_exit_talk = true;
        }
    }
    assert!(received_exit_move, "Member must receive 0504 battle exit frame");
    assert!(received_exit_talk, "Member must receive 1408 battle exit frame");

    lock_online_sessions().remove(&6001);
    lock_online_sessions().remove(&6002);
}

/// 9. Test full session synchronization on battle end (Win path):
/// Verifies that combat results (e.g. EXP gained, HP/SP updates) are completely
/// copied into global `online_sessions()`, preventing state erasure on subsequent packets.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_battle_rewards_and_drops_synchronize_to_online_sessions() {
    let mut data = GameData::default();
    data.npcs.insert(
        41001,
        Npc {
            id: 41001,
            name: b"Luu Bi".to_vec(),
            lv: 10,
            hp: 1, // 1 HP so basic attack defeats it in 1 turn
            sp: 10,
            ..Default::default()
        },
    );
    data.skills.insert(
        10000,
        ts_dream::data::tables::Skill {
            id: 10000,
            name: "Tan Cong".to_string(),
            skill_type: 1,
            ..Default::default()
        },
    );

    let fight = EveFightData {
        eve_no: 700,
        fight_type: 0,
        link_to_result: 0,
        left_enemies: vec![
            EveFightEnemy {
                no: 1,
                npc_id: 41001,
                location_pos: 2,
                ai: 0,
            },
            EveFightEnemy {
                no: 2,
                npc_id: 41001,
                location_pos: 1,
                ai: 0,
            },
        ],
        right_enemies: Vec::new(),
        fight_limit: 0,
    };

    let mut service = BattleService::new(Arc::new(data));
    service.set_input_timeout(std::time::Duration::from_millis(50));

    let leader_sess = Arc::new(RwLock::new(Session {
        id: 7001,
        map_id: 12001,
        id_leader: 7001,
        level: 1,
        agi: 50,
        hp: 100,
        hp_max: 100,
        sp: 100,
        sp_max: 100,
        texp: 0,
        encounter_steps: 25,
        encounter_threshold: 20,
        ..Default::default()
    }));
    let member_sess = Arc::new(RwLock::new(Session {
        id: 7002,
        map_id: 12001,
        id_leader: 7001,
        level: 1,
        agi: 10,
        hp: 100,
        hp_max: 100,
        sp: 100,
        sp_max: 100,
        texp: 0,
        encounter_steps: 15,
        encounter_threshold: 20,
        ..Default::default()
    }));

    let _rx_leader = service.register(7001, leader_sess.clone());
    let _rx_member = service.register(7002, member_sess.clone());

    lock_online_sessions().insert(7001, leader_sess.try_read().unwrap().clone());
    lock_online_sessions().insert(7002, member_sess.try_read().unwrap().clone());

    let mut leader_guard = leader_sess.write().await;
    leader_guard.id_mem = [7002, 0, 0, 0];
    let battle_id = service.start_encounter_battle(&mut leader_guard, &fight, 112);
    assert!(battle_id > 0);
    drop(leader_guard);

    // Leader submits normal Attack command (skill 10000) against enemy at (0, 2)
    let cmd_leader = ts_dream::battle::runner::BattleCommand {
        row: 3,
        col: 2,
        skill_id: 10000,
        skill_lv: 1,
        row_attack: 0,
        col_attack: 2,
        use_item: 0,
    };
    let leader_snap = leader_sess.read().await.clone();
    assert!(service.submit_command(&leader_snap, cmd_leader));

    // Member submits normal Attack command (skill 10000) against enemy at (0, 1)
    let cmd_member = ts_dream::battle::runner::BattleCommand {
        row: 3,
        col: 1,
        skill_id: 10000,
        skill_lv: 1,
        row_attack: 0,
        col_attack: 1,
        use_item: 0,
    };
    let member_snap = member_sess.read().await.clone();
    assert!(service.submit_command(&member_snap, cmd_member));

    // Wait for battle to finish (PlayerWin)
    let deadline = tokio::time::Instant::now() + std::time::Duration::from_millis(1500);
    loop {
        let sess = leader_sess.read().await;
        if sess.battle_id == 0 {
            break;
        }
        drop(sess);
        if tokio::time::Instant::now() > deadline {
            panic!("Battle did not finish within timeout");
        }
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
    }

    // Verify global online_sessions() has FULL updated state (EXP, cooldown, battle_id = 0)
    let online_map = lock_online_sessions();
    let leader_online = online_map.get(&7001).unwrap();
    assert_eq!(leader_online.battle_id, 0);
    assert!(leader_online.last_battle_end_ms > 0);
    assert_eq!(leader_online.encounter_steps, 0);
    assert!(
        leader_online.texp > 0,
        "Leader's online_sessions record must have battle EXP synchronized, got texp={}",
        leader_online.texp
    );

    let member_online = online_map.get(&7002).unwrap();
    assert_eq!(member_online.battle_id, 0);
    assert!(member_online.last_battle_end_ms > 0);
    assert_eq!(member_online.encounter_steps, 0);
    assert!(
        member_online.texp > 0,
        "Member's online_sessions record must have battle EXP synchronized, got texp={}",
        member_online.texp
    );
    drop(online_map);

    lock_online_sessions().remove(&7001);
    lock_online_sessions().remove(&7002);
}

/// 10. Test Opcode 0x0B Sub 0x05 (PlayerFled client packet):
/// Verifies that when the client sends Opcode 0x0B Sub 0x05, the server cleanly
/// clears battle_id, sets last_battle_end_ms, resets encounter_steps, syncs online_sessions,
/// and responds with battle exit move (0504) and talk (1408) frames.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_opcode_0b_sub_5_flee_battle() {
    let data = GameData::default();
    let service = BattleService::new(Arc::new(data.clone()));

    let player_sess = Arc::new(RwLock::new(Session {
        id: 8001,
        map_id: 12001,
        battle_id: 99,
        encounter_steps: 18,
        encounter_threshold: 20,
        ..Default::default()
    }));

    // Register a peer on the same map to verify clear_battle_smoke broadcast
    let peer_sess = Arc::new(RwLock::new(Session {
        id: 8002,
        map_id: 12001,
        ..Default::default()
    }));
    let mut rx_peer = service.register(8002, peer_sess.clone());

    let _rx = service.register(8001, player_sess.clone());
    lock_online_sessions().insert(8001, player_sess.read().await.clone());
    lock_online_sessions().insert(8002, peer_sess.read().await.clone());

    let mut conn = Conn {
        session: player_sess.read().await.clone(),
        ..Default::default()
    };
    let mut out = HandleOutcome::default();
    let payload = [0u8];
    let mut ctx = test_ctx(
        &mut conn,
        &data,
        &service,
        &mut out,
        0x0B,
        5,
        &payload,
    );

    ts_dream::server::handlers::battle::handle_battle(&mut ctx);

    // Verify session in ctx.conn was cleaned up
    assert_eq!(ctx.conn.session.battle_id, 0);
    assert!(ctx.conn.session.last_battle_end_ms > 0);
    assert_eq!(ctx.conn.session.encounter_steps, 0);

    // Verify global online_sessions was updated
    let map = lock_online_sessions();
    let online = map.get(&8001).unwrap();
    assert_eq!(online.battle_id, 0);
    assert!(online.last_battle_end_ms > 0);
    assert_eq!(online.encounter_steps, 0);
    drop(map);

    // Verify out received exit move (0504) and talk (1408) frames and 0B00 hide frame
    assert!(out.outgoing.iter().any(|f| f.frame.contains("0504")));
    assert!(out.outgoing.iter().any(|f| f.frame.contains("1408")));
    assert!(out.outgoing.iter().any(|f| f.frame.contains("0B00")));

    // Verify peer on the map received ClearBattleSmoke frame (0B00)
    let mut peer_received_smoke_clear = false;
    while let Ok(frame) = rx_peer.try_recv() {
        if frame.contains("0B00") {
            peer_received_smoke_clear = true;
            break;
        }
    }
    assert!(
        peer_received_smoke_clear,
        "Peer on map must receive ClearBattleSmoke frame (0B00) when player flees"
    );

    lock_online_sessions().remove(&8001);
    lock_online_sessions().remove(&8002);
}

/// 11. Test that a party member already in combat (battle_id != 0)
/// is NOT pulled into a new encounter battle by the leader.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_party_member_already_in_battle_not_pulled_into_new_encounter() {
    let mut data = GameData::default();
    data.npcs.insert(
        41001,
        Npc {
            id: 41001,
            name: b"Wolf".to_vec(),
            hp: 50,
            ..Default::default()
        },
    );
    let fight = EveFightData {
        eve_no: 800,
        fight_type: 0,
        link_to_result: 0,
        left_enemies: vec![EveFightEnemy {
            no: 1,
            npc_id: 41001,
            location_pos: 2,
            ai: 0,
        }],
        right_enemies: Vec::new(),
        fight_limit: 0,
    };

    let service = BattleService::new(Arc::new(data));

    let leader_sess = Arc::new(RwLock::new(Session {
        id: 9001,
        map_id: 12001,
        id_leader: 9001,
        id_mem: [9002, 0, 0, 0],
        ..Default::default()
    }));

    // Member is ALREADY in battle 42!
    let member_sess = Arc::new(RwLock::new(Session {
        id: 9002,
        map_id: 12001,
        id_leader: 9001,
        battle_id: 42,
        ..Default::default()
    }));

    let _rx_leader = service.register(9001, leader_sess.clone());
    let mut rx_member = service.register(9002, member_sess.clone());

    lock_online_sessions().insert(9001, leader_sess.try_read().unwrap().clone());
    lock_online_sessions().insert(9002, member_sess.try_read().unwrap().clone());

    let mut leader_guard = leader_sess.write().await;
    let battle_id = service.start_encounter_battle(&mut leader_guard, &fight, 112);
    assert!(battle_id > 0);
    drop(leader_guard);

    // Member must NOT be in the leader's battle, battle_id must remain 42
    let mem_guard = member_sess.read().await;
    assert_eq!(
        mem_guard.battle_id, 42,
        "Member must not have their battle_id overwritten by leader's encounter"
    );
    drop(mem_guard);

    let map = lock_online_sessions();
    let mem_online = map.get(&9002).unwrap();
    assert_eq!(
        mem_online.battle_id, 42,
        "Member's global online record must remain 42"
    );
    drop(map);

    // Member must NOT receive any battle start frame for leader's battle
    let mut received_start = false;
    while let Ok(frame) = rx_member.try_recv() {
        if frame.contains("0BFA") {
            received_start = true;
        }
    }
    assert!(
        !received_start,
        "Busy member must NOT receive 0BFA start frame for leader's battle"
    );

    lock_online_sessions().remove(&9001);
    lock_online_sessions().remove(&9002);
}

