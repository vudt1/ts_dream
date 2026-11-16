//! Integration & unit tests for Tickets 16 and 17:
//! - Ticket 16: `WalkStep` encounter-zone fast-path before `snapshot_state_with_data`,
//!   borrowed `GameData.quest_marks` reference in `EveStateBuilder`/`PlayerStateInputs`
//!   (no per-step 2,394-entry HashMap clone), `try_trigger_random_encounter` + `WalkStep`
//!   snapshot coordination, and isolation of `run_sync_future`/sync wrappers from production.
//! - Ticket 17: `Two-Phase State Lock` (`lock_player_operations` ascending IDs),
//!   `registry_update_if_online`, and multi-member SQLite `persist_sessions_transaction`
//!   for special event NPCs (`15312`, `15313`, `61210`), North Star quiz (`0x10`),
//!   and `MovementHandler` party position sync.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use ts_dream::battle::service::BattleService;
use ts_dream::data::loader::GameData;
use ts_dream::data::loaders::{
    EveCondition, EveEncounterPlacement, EveResult, NpcEventData, SceneEveData,
};
use ts_dream::db::modern::traits::CharacterSeed;
use ts_dream::db::modern::sqlite::SqliteRepositories;
use ts_dream::db::pool::DbPool;
use ts_dream::eve::evaluator::evaluate;
use ts_dream::server::dispatcher::{dispatch, ServerEnv};
use ts_dream::server::handlers::npc_event::{set_eve_events_enabled, snapshot_state_with_data};
use ts_dream::server::session::{
    lock_online_sessions, registry_get, registry_register_for_login, Conn, Session,
};

use crate::common::test_db::setup_test_db;

fn click_npc_frame(place_id: u16) -> Vec<u8> {
    let [lo, hi] = place_id.to_le_bytes();
    vec![0xF4, 0x44, 0x04, 0x00, 0x14, 0x01, lo, hi]
}

fn move_frame(dir: u8, x: u16, y: u16) -> Vec<u8> {
    let [x0, x1] = x.to_le_bytes();
    let [y0, y1] = y.to_le_bytes();
    vec![0xF4, 0x44, 0x07, 0x00, 0x06, 0x01, dir, x0, x1, y0, y1]
}

fn quiz_answer_frame(answer_index: u32) -> Vec<u8> {
    let [a0, a1, a2, a3] = answer_index.to_le_bytes();
    vec![0xF4, 0x44, 0x06, 0x00, 0x10, 0x01, a0, a1, a2, a3]
}

async fn seed_account_and_char(
    pool: &DbPool,
    account_id: i64,
    name: &[u8],
    map_id: u16,
    x: u16,
    y: u16,
) -> u32 {
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
        .expect("create character") as u32
}

fn collect_rs_files(dir: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    if dir.is_dir() {
        if let Ok(entries) = fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    files.extend(collect_rs_files(&path));
                } else if path.extension().and_then(|s| s.to_str()) == Some("rs") {
                    files.push(path);
                }
            }
        }
    }
    files
}

// ===========================================================================
// Ticket 16 — WalkStep hot-path, borrowed quest_marks & run_sync_future isolation
// ===========================================================================

#[tokio::test]
async fn ticket16_walkstep_outside_encounter_zone_skips_cleanly_and_inside_reuses_snapshot() {
    set_eve_events_enabled(true);
    let mut data = GameData::default();
    let mut scene = SceneEveData::default();

    // Scripted encounter zone at (500..=600, 500..=600) with a non-battle quest step (rt=0, rc=2).
    scene.npc_events.insert(
        10,
        NpcEventData {
            eve_no: 10,
            conditions: vec![EveCondition {
                condition_no: 1,
                and_num: 1,
                to_result: 1,
                results: vec![EveResult {
                    result_type: 0,
                    result_class: 2,
                    parameter: 10555,
                    parameter_style: 1,
                    result_value: 1,
                    ..Default::default()
                }],
                ..Default::default()
            }],
            when_happen: [true, false, false, false],
        },
    );
    scene.encounters.push(EveEncounterPlacement {
        id: 1,
        start_x: 500,
        start_y: 500,
        end_x: 600,
        end_y: 600,
        events: vec![10],
        ..Default::default()
    });
    data.scene_eve_data.insert(12001, scene);

    let data_arc = Arc::new(data);
    let service = BattleService::new(Arc::clone(&data_arc));
    let env = ServerEnv::none();

    let mut conn = Conn {
        session: Session {
            id: 16001,
            map_id: 12001,
            map_x: 100,
            map_y: 100,
            encounter_steps: 19,
            encounter_threshold: 20,
            ..Default::default()
        },
        ..Default::default()
    };

    // 1. Move OUTSIDE encounter zone (150, 150) -> no Eve event, no quest side-effect.
    let out_outside = dispatch(&mut conn, &move_frame(3, 150, 150), &data_arc, &service, &env).await;
    assert!(conn.session.current_event_session.is_none());
    assert!(conn.session.quest_tasks.is_empty());
    assert!(conn.session.completed_eve_counts.is_empty());
    assert!(
        out_outside.outgoing.is_empty(),
        "WalkStep outside encounter zone must emit no Eve frames"
    );

    // 2. Move INSIDE encounter zone (550, 550) when encounter_steps >= threshold:
    // try_trigger_random_encounter rolls, builds snapshot, finds no rt=3 battle, and passes
    // cached snapshot to WalkStep which triggers quest 10555 step 1.
    conn.session.encounter_steps = 25;
    conn.session.encounter_threshold = 20;
    let _out_inside = dispatch(&mut conn, &move_frame(3, 550, 550), &data_arc, &service, &env).await;
    assert_eq!(conn.session.battle_id, 0);
    assert_eq!(
        conn.session.quest_tasks.get(&10555).map(|&(_, s)| s),
        Some(1),
        "WalkStep inside encounter zone must still trigger scripted Eve event"
    );
}

#[test]
fn ticket16_quest_marks_borrowed_reference_evaluates_cc2_pstyle3_correctly() {
    let mut data = GameData::default();
    data.quest_marks.insert(10012, 42);
    data.quest_marks.insert(10013, 43);

    let cond_10012 = EveCondition {
        condition_no: 1,
        condition_class: 2,
        condition_parameter: 10012,
        condition_parameter_style: 3,
        condition_ops: 5, // >=
        condition_value: 1,
        and_num: 1,
        ..Default::default()
    };

    // Negative: empty quest_dont -> false
    let mut session = Session::default();
    let state_empty = snapshot_state_with_data(&session, Some(&data), None);
    assert!(state_empty.mission_flags.is_empty());
    assert!(!evaluate(&cond_10012, &state_empty));

    // Negative: unrelated bit_id 99 -> false for 10012
    session.quest_dont.insert(99);
    let state_unrelated = snapshot_state_with_data(&session, Some(&data), None);
    assert!(!evaluate(&cond_10012, &state_unrelated));

    // Positive: bit_id 42 -> projects to quest_id 10012 -> true
    session.quest_dont.insert(42);
    let state_marked = snapshot_state_with_data(&session, Some(&data), None);
    assert_eq!(state_marked.mission_flags.get(&10012), Some(&1));
    assert!(!state_marked.mission_flags.contains_key(&42));
    assert!(evaluate(&cond_10012, &state_marked));
}

#[test]
fn ticket16_no_production_caller_of_run_sync_future_or_sync_wrappers() {
    let rs_files = collect_rs_files(Path::new("src"));
    let mut violations = Vec::new();
    let forbidden_calls = [
        "run_sync_future(",
        "execute_event_step(",
        "resume_eve_after_battle_sync(",
        "resume_eve_after_battle_sync_full(",
    ];

    for file in rs_files {
        let norm = file.to_string_lossy().replace('\\', "/");
        let content = fs::read_to_string(&file).unwrap_or_default();
        for (idx, line) in content.lines().enumerate() {
            let trimmed = line.trim();
            if trimmed.starts_with("//") || trimmed.starts_with("/*") || trimmed.starts_with('*') {
                continue;
            }
            // Allow the definitions inside eve_runner.rs and #[doc(hidden)] re-export in talk.rs
            if norm.ends_with("src/server/handlers/eve_runner.rs")
                && (trimmed.starts_with("fn run_sync_future")
                    || trimmed.starts_with("pub fn execute_event_step")
                    || trimmed.starts_with("pub fn resume_eve_after_battle_sync")
                    || trimmed.starts_with("run_sync_future(")
                    || trimmed.starts_with("let out = resume_eve_after_battle_sync_full("))
            {
                continue;
            }
            if norm.ends_with("src/server/handlers/talk.rs")
                && trimmed.contains("resume_eve_after_battle_sync")
            {
                continue;
            }
            for pat in forbidden_calls {
                if trimmed.contains(pat) {
                    violations.push(format!("{norm}:{}: calls {pat}", idx + 1));
                }
            }
        }
    }

    assert!(
        violations.is_empty(),
        "Production code must not call run_sync_future or sync Eve wrappers:\n{}",
        violations.join("\n")
    );
}

// ===========================================================================
// Ticket 17 — Two-Phase State Lock & Multi-Member Persistence Outside EventRunner
// ===========================================================================

#[tokio::test]
async fn ticket17_e2e_event_npcs_15312_15313_61210_party_lock_registry_and_sqlite_persist() {
    let pool = setup_test_db().await;
    let repos = SqliteRepositories::new(pool.clone());

    // 1. Test 15312 (requires 1 or 5 party members) with 5-player party
    let leader_id = seed_account_and_char(&pool, 17101, b"L17", 12001, 100, 100).await;
    let mem_ids = [
        seed_account_and_char(&pool, 17102, b"M17_1", 12001, 100, 100).await,
        seed_account_and_char(&pool, 17103, b"M17_2", 12001, 100, 100).await,
        seed_account_and_char(&pool, 17104, b"M17_3", 12001, 100, 100).await,
        seed_account_and_char(&pool, 17105, b"M17_4", 12001, 100, 100).await,
    ];

    let data_arc = Arc::new(GameData::default());
    let service = BattleService::new(Arc::clone(&data_arc));
    let env = ServerEnv::with_pool(&pool);

    let mut leader_conn = Conn {
        session: Session {
            id: leader_id,
            db_character_id: i64::from(leader_id),
            id_leader: leader_id,
            id_mem: mem_ids,
            map_id: 12001,
            map_x: 100,
            map_y: 100,
            ..Default::default()
        },
        ..Default::default()
    };
    registry_register_for_login(&leader_conn.session);

    let mut member_conns: Vec<Conn> = mem_ids
        .iter()
        .map(|&mid| {
            let s = Session {
                id: mid,
                db_character_id: i64::from(mid),
                id_leader: leader_id,
                id_mem: [leader_id, mem_ids[0], mem_ids[1], mem_ids[2]],
                map_id: 12001,
                map_x: 100,
                map_y: 100,
                ..Default::default()
            };
            registry_register_for_login(&s);
            Conn {
                session: s,
                ..Default::default()
            }
        })
        .collect();

    // Negative test: party member clicking 15312 is rejected without changing map/DB
    let out_member_reject = dispatch(
        &mut member_conns[0],
        &click_npc_frame(15312),
        &data_arc,
        &service,
        &env,
    )
    .await;
    assert_eq!(member_conns[0].session.map_id, 12001);
    assert!(out_member_reject
        .outgoing
        .iter()
        .any(|f| f.frame.contains(&ts_dream::server::spawn::sys_msg_frame("Chi doi truong moi co the dang ky"))));

    // Positive: leader clicks 15312 -> warps leader & all 4 members to 12911 (400, 400) + persists DB
    let _out_15312 = dispatch(
        &mut leader_conn,
        &click_npc_frame(15312),
        &data_arc,
        &service,
        &env,
    )
    .await;
    assert_eq!(leader_conn.session.map_id, 12911);
    assert_eq!(
        leader_conn.session.quest_tasks.get(&212).map(|&(_, s)| s),
        Some(1)
    );
    for &mid in &mem_ids {
        let reg = registry_get(mid).expect("member in registry");
        assert_eq!((reg.map_id, reg.map_x, reg.map_y), (12911, 400, 400));
        let mut db_s = Session::new();
        assert!(repos.sessions().load(i64::from(mid), &mut db_s).await.unwrap());
        assert_eq!((db_s.map_id, db_s.map_x, db_s.map_y), (12911, 400, 400));
    }

    // 2. Test 15313: leader clicks 15313 on map 12911 -> warps leader & members to 12912 + quest 212=2 in DB
    let _out_15313 = dispatch(
        &mut leader_conn,
        &click_npc_frame(15313),
        &data_arc,
        &service,
        &env,
    )
    .await;
    assert_eq!(leader_conn.session.map_id, 12912);
    assert_eq!(
        leader_conn.session.quest_tasks.get(&212).map(|&(_, s)| s),
        Some(2)
    );
    for &mid in &mem_ids {
        let reg = registry_get(mid).expect("member in registry");
        assert_eq!((reg.map_id, reg.map_x, reg.map_y), (12912, 400, 400));
        assert_eq!(reg.quest_tasks.get(&212).map(|&(_, s)| s), Some(2));
        let mut db_s = Session::new();
        assert!(repos.sessions().load(i64::from(mid), &mut db_s).await.unwrap());
        assert_eq!((db_s.map_id, db_s.map_x, db_s.map_y), (12912, 400, 400));
        assert_eq!(db_s.quest_tasks.get(&212).map(|&(_, s)| s), Some(2));
    }

    // 3. Test 61210: Nine Heavens floor transition (49960 -> 49961, quest 12309: 1 -> 2)
    leader_conn.session.map_id = 49960;
    leader_conn.session.quest_tasks.insert(12309, (1, 1));
    registry_register_for_login(&leader_conn.session);
    {
        let mut online = lock_online_sessions();
        for &mid in &mem_ids {
            if let Some(m) = online.get_mut(&mid) {
                m.map_id = 49960;
                m.quest_tasks.insert(12309, (1, 1));
            }
        }
    }

    let _out_61210 = dispatch(
        &mut leader_conn,
        &click_npc_frame(61210),
        &data_arc,
        &service,
        &env,
    )
    .await;
    assert_eq!(leader_conn.session.map_id, 49961);
    assert_eq!(
        leader_conn.session.quest_tasks.get(&12309).map(|&(_, s)| s),
        Some(2)
    );
    for &mid in &mem_ids {
        let reg = registry_get(mid).expect("member in registry");
        assert_eq!((reg.map_id, reg.map_x, reg.map_y), (49961, 400, 400));
        assert_eq!(reg.quest_tasks.get(&12309).map(|&(_, s)| s), Some(2));
        let mut db_s = Session::new();
        assert!(repos.sessions().load(i64::from(mid), &mut db_s).await.unwrap());
        assert_eq!((db_s.map_id, db_s.map_x, db_s.map_y), (49961, 400, 400));
        assert_eq!(db_s.quest_tasks.get(&12309).map(|&(_, s)| s), Some(2));
    }

    let mut online = lock_online_sessions();
    online.remove(&leader_id);
    for &mid in &mem_ids {
        online.remove(&mid);
    }
}

#[tokio::test]
async fn ticket17_e2e_north_star_quiz_0x10_party_lock_registry_and_sqlite_persist() {
    let pool = setup_test_db().await;
    let repos = SqliteRepositories::new(pool.clone());

    let leader_id = seed_account_and_char(&pool, 17201, b"QuizL", 12001, 200, 200).await;
    let member_id = seed_account_and_char(&pool, 17202, b"QuizM", 12001, 200, 200).await;

    let data_arc = Arc::new(GameData::default());
    let service = BattleService::new(Arc::clone(&data_arc));
    let env = ServerEnv::with_pool(&pool);

    let mut leader_conn = Conn {
        session: Session {
            id: leader_id,
            db_character_id: i64::from(leader_id),
            id_leader: leader_id,
            id_mem: [member_id, 0, 0, 0],
            map_id: 12001,
            map_x: 200,
            map_y: 200,
            hp: 100,
            hp_max: 100,
            ..Default::default()
        },
        ..Default::default()
    };
    let member_session = Session {
        id: member_id,
        db_character_id: i64::from(member_id),
        id_leader: leader_id,
        id_mem: [leader_id, 0, 0, 0],
        map_id: 12001,
        map_x: 200,
        map_y: 200,
        hp: 100,
        hp_max: 100,
        ..Default::default()
    };
    registry_register_for_login(&leader_conn.session);
    registry_register_for_login(&member_session);

    let (battle_id, _modal) = service.start_north_star_battle(&mut leader_conn.session, 1643, 112);
    assert!(battle_id > 0);
    assert_eq!(registry_get(member_id).unwrap().battle_id, battle_id);

    // Dispatch 0x10 01 with correct answer (index 1 in fallback question bank)
    let _out = dispatch(
        &mut leader_conn,
        &quiz_answer_frame(1),
        &data_arc,
        &service,
        &env,
    )
    .await;

    assert_eq!(leader_conn.session.battle_id, 0);
    assert_eq!(leader_conn.session.god, 10);
    assert_eq!(
        registry_get(leader_id).unwrap().god,
        10,
        "leader god must be synced to registry"
    );
    assert_eq!(
        registry_get(member_id).unwrap().battle_id,
        0,
        "member must be evacuated from North Star battle in registry"
    );

    let mut l_db = Session::new();
    assert!(repos.sessions().load(i64::from(leader_id), &mut l_db).await.unwrap());
    assert_eq!(l_db.god, 10, "leader god buff must persist to SQLite");

    let mut online = lock_online_sessions();
    online.remove(&leader_id);
    online.remove(&member_id);
}

#[tokio::test]
async fn ticket17_e2e_movement_party_sync_updates_member_coords_safely() {
    let data_arc = Arc::new(GameData::default());
    let service = BattleService::new(Arc::clone(&data_arc));
    let env = ServerEnv::none();

    let leader_id = 17301u32;
    let member_id = 17302u32;
    let mut leader_conn = Conn {
        session: Session {
            id: leader_id,
            id_leader: leader_id,
            id_mem: [member_id, 0, 0, 0],
            map_id: 12001,
            map_x: 100,
            map_y: 100,
            ..Default::default()
        },
        ..Default::default()
    };
    let member_session = Session {
        id: member_id,
        id_leader: leader_id,
        id_mem: [leader_id, 0, 0, 0],
        map_id: 12001,
        map_x: 100,
        map_y: 100,
        ..Default::default()
    };
    registry_register_for_login(&leader_conn.session);
    registry_register_for_login(&member_session);

    let _out = dispatch(
        &mut leader_conn,
        &move_frame(2, 320, 480),
        &data_arc,
        &service,
        &env,
    )
    .await;

    let l_reg = registry_get(leader_id).expect("leader in registry");
    let m_reg = registry_get(member_id).expect("member in registry");
    assert_eq!((l_reg.map_x, l_reg.map_y, l_reg.gocnhin), (320, 480, 2));
    assert_eq!((m_reg.map_x, m_reg.map_y, m_reg.gocnhin), (320, 480, 2));

    let mut online = lock_online_sessions();
    online.remove(&leader_id);
    online.remove(&member_id);
}

#[tokio::test]
async fn ticket17_negative_db_failure_rolls_back_ram_and_suppresses_relocate_frames() {
    let pool = setup_test_db().await;
    let leader_id = seed_account_and_char(&pool, 17401, b"RbL", 12001, 100, 100).await;
    let member_id = seed_account_and_char(&pool, 17402, b"RbM", 12001, 100, 100).await;

    let data_arc = Arc::new(GameData::default());
    let service = BattleService::new(Arc::clone(&data_arc));

    let mut leader_conn = Conn {
        session: Session {
            id: leader_id,
            db_character_id: i64::from(leader_id),
            id_leader: leader_id,
            id_mem: [member_id, 0, 0, 0],
            map_id: 12001,
            map_x: 100,
            map_y: 100,
            ..Default::default()
        },
        ..Default::default()
    };
    let member_session = Session {
        id: member_id,
        db_character_id: i64::from(member_id),
        id_leader: leader_id,
        id_mem: [leader_id, 0, 0, 0],
        map_id: 12001,
        map_x: 100,
        map_y: 100,
        ..Default::default()
    };
    registry_register_for_login(&leader_conn.session);
    registry_register_for_login(&member_session);

    // Force DB write transaction failure by closing the write pool before dispatch.
    pool.write.close().await;
    let env = ServerEnv::with_pool(&pool);

    // 1. NPC 15313 on map 12911 -> DB fails -> RAM state & registry rolled back, no relocate frame
    leader_conn.session.map_id = 12911;
    leader_conn.session.quest_tasks.insert(212, (1, 1));
    registry_register_for_login(&leader_conn.session);

    let out_15313 = dispatch(
        &mut leader_conn,
        &click_npc_frame(15313),
        &data_arc,
        &service,
        &env,
    )
    .await;
    assert_eq!(leader_conn.session.map_id, 12911, "leader map_id must roll back on DB failure");
    assert_eq!(
        leader_conn.session.quest_tasks.get(&212).map(|&(_, s)| s),
        Some(1),
        "leader quest_tasks[212] must roll back on DB failure"
    );
    assert_eq!(
        registry_get(member_id).unwrap().map_id,
        12001,
        "member registry must not be updated when DB fails"
    );
    let relocate_12912 = ts_dream::server::spawn::build_relocate_packet(leader_id, 12912, 400, 400, 0);
    assert!(
        !out_15313.outgoing.iter().any(|f| f.frame.contains(&relocate_12912)),
        "must not emit relocate frame when DB commit fails"
    );

    // 2. 0x10 North Star Quiz -> DB fails -> god & battle_id rolled back
    let (battle_id, _modal) = service.start_north_star_battle(&mut leader_conn.session, 1643, 112);
    assert!(battle_id > 0);
    let out_quiz = dispatch(
        &mut leader_conn,
        &quiz_answer_frame(1),
        &data_arc,
        &service,
        &env,
    )
    .await;
    assert_eq!(leader_conn.session.god, 0, "leader god must roll back on DB failure");
    assert_eq!(leader_conn.session.battle_id, battle_id, "leader battle_id must roll back on DB failure");
    assert_eq!(
        registry_get(member_id).unwrap().battle_id,
        battle_id,
        "member battle_id in registry must not be cleared on DB failure"
    );
    assert!(
        out_quiz.outgoing.is_empty(),
        "must not emit quiz reward/battle_trailer frames when DB commit fails"
    );

    let mut online = lock_online_sessions();
    online.remove(&leader_id);
    online.remove(&member_id);
}
