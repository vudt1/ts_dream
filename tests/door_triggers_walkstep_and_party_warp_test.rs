//! Integration tests for Ticket 07:
//! - `NpcTrigger::ClickDoor` via `0x14 04 <door_id>` (and bare `0x14 04` EndTalk parity)
//! - `NpcTrigger::MeetDoor` via `0x06 01` movement into door proximity
//! - `NpcTrigger::WalkStep` via `0x06 01` movement into scripted encounter zone
//! - Party `Door Warp` (`Two-Phase State Lock` + `registry_update_if_online` + `0x0C` + SQLite write-through for Leader & Member)
//! - Negative tests: unmet door condition & invalid warp destination leave Leader/Member RAM + DB untouched

mod common;

use common::test_db::setup_test_db;
use ts_dream::battle::service::BattleService;
use ts_dream::data::loader::GameData;
use ts_dream::data::loaders::{
    EveCondition, EveDoorPlacement, EveEncounterPlacement, EveResult, NpcEventData, SceneEveData,
};
use ts_dream::data::tables::Warp;
use ts_dream::db::modern::sqlite::SqliteRepositories;
use ts_dream::db::modern::traits::CharacterSeed;
use ts_dream::db::pool::DbPool;
use ts_dream::server::dispatcher::{dispatch, HandleOutcome, ServerEnv};
use ts_dream::server::handlers::npc_event::set_eve_events_enabled;
use ts_dream::server::session::{
    lock_online_sessions, registry_get, registry_register_for_login, Conn, Session,
};

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

fn build_door_test_data(require_quest: Option<u16>, valid_warp: bool) -> GameData {
    let mut data = GameData::default();
    let mut scene = SceneEveData::default();

    scene.doors.insert(
        10,
        EveDoorPlacement {
            id: 10,
            events: vec![1],
            x: 300,
            y: 400,
        },
    );

    let cond = if let Some(qid) = require_quest {
        EveCondition {
            condition_no: 1,
            condition_class: 2, // QuestStep
            condition_parameter: qid,
            condition_parameter_style: 1,
            condition_ops: 0,
            condition_value: 1,
            to_result: 1,
            and_num: 0,
            results: vec![EveResult {
                result_group_no: 1,
                result_no: 1,
                result_type: 2, // Warp
                result_class: 0,
                parameter: 77,
                parameter_style: 0,
                result_value: 0,
                result_mean_no: 0,
            }],
            ..Default::default()
        }
    } else {
        EveCondition {
            condition_no: 1,
            condition_class: 0,
            to_result: 1,
            and_num: 0,
            results: vec![EveResult {
                result_group_no: 1,
                result_no: 1,
                result_type: 2, // Warp
                result_class: 0,
                parameter: 77,
                parameter_style: 0,
                result_value: 0,
                result_mean_no: 0,
            }],
            ..Default::default()
        }
    };

    scene.npc_events.insert(
        1,
        NpcEventData {
            eve_no: 1,
            conditions: vec![cond],
            ..Default::default()
        },
    );

    // Scripted WalkStep encounter zone at (600..=700, 600..=700)
    scene.encounters.push(EveEncounterPlacement {
        id: 50,
        events: vec![2],
        start_x: 600,
        start_y: 600,
        end_x: 700,
        end_y: 700,
        full_map: false,
    });
    scene.npc_events.insert(
        2,
        NpcEventData {
            eve_no: 2,
            conditions: vec![EveCondition {
                condition_no: 1,
                condition_class: 0,
                to_result: 1,
                and_num: 0,
                results: vec![EveResult {
                    result_group_no: 1,
                    result_no: 1,
                    result_type: 0,
                    result_class: 2, // UpdateQuest
                    parameter: 10888,
                    parameter_style: 1,
                    result_value: 2,
                    result_mean_no: 0,
                }],
                ..Default::default()
            }],
            ..Default::default()
        },
    );

    scene.build_indices();
    data.scene_eve_data.insert(12001, scene);

    if valid_warp {
        data.warps.insert(
            (12001, 77),
            Warp {
                map1: 12001,
                warpid: 77,
                map2: 12002,
                x: 820,
                y: 940,
            },
        );
    }
    data
}

#[tokio::test]
async fn e2e_click_door_party_warp_locks_syncs_and_persists_db() {
    set_eve_events_enabled(true);
    let pool = setup_test_db().await;
    let repos = SqliteRepositories::new(pool.clone());

    let leader_id = seed_account_and_char(&pool, 9701, b"LeaderWarp", 12001, 300, 400).await;
    let member_id = seed_account_and_char(&pool, 9702, b"MemberWarp", 12001, 300, 400).await;

    let mut leader_session = Session::new();
    leader_session.id = leader_id as u32;
    assert!(repos.sessions().load(leader_id, &mut leader_session).await.unwrap());
    leader_session.id_leader = leader_session.id;
    leader_session.id_mem = [member_id as u32, 0, 0, 0];

    let mut member_session = Session::new();
    member_session.id = member_id as u32;
    assert!(repos.sessions().load(member_id, &mut member_session).await.unwrap());
    member_session.id_leader = leader_session.id;

    registry_register_for_login(&leader_session);
    registry_register_for_login(&member_session);

    let mut conn = Conn {
        session: leader_session,
        ..Default::default()
    };
    let data = build_door_test_data(None, true);
    let service = BattleService::default();
    let env = ServerEnv::with_pool(&pool);

    let out = dispatch(&mut conn, &click_door_frame(10), &data, &service, &env).await;

    assert_eq!((conn.session.map_id, conn.session.map_x, conn.session.map_y), (12002, 820, 940));
    assert!(has_relocate_frame(&outgoing_hexes(&out), leader_id as u32, 12002));
    assert!(has_relocate_frame(&direct_hexes(&out, member_id as u32), member_id as u32, 12002));

    let reg_leader = registry_get(leader_id as u32).expect("leader in registry");
    let reg_member = registry_get(member_id as u32).expect("member in registry");
    assert_eq!((reg_leader.map_id, reg_leader.map_x, reg_leader.map_y), (12002, 820, 940));
    assert_eq!((reg_member.map_id, reg_member.map_x, reg_member.map_y), (12002, 820, 940));

    // Verify SQLite write-through for BOTH Leader and Member
    let mut db_leader = Session::new();
    db_leader.id = leader_id as u32;
    assert!(repos.sessions().load(leader_id, &mut db_leader).await.unwrap());
    assert_eq!((db_leader.map_id, db_leader.map_x, db_leader.map_y), (12002, 820, 940));

    let mut db_member = Session::new();
    db_member.id = member_id as u32;
    assert!(repos.sessions().load(member_id, &mut db_member).await.unwrap());
    assert_eq!((db_member.map_id, db_member.map_x, db_member.map_y), (12002, 820, 940));

    // Verify bare `F4 44 02 00 14 04` remains EndTalk
    let out_end = dispatch(&mut conn, &[0xF4, 0x44, 0x02, 0x00, 0x14, 0x04], &data, &service, &env).await;
    assert!(out_end.outgoing.iter().any(|f| f.frame == "F44402001408"));

    {
        let mut sessions = lock_online_sessions();
        sessions.remove(&(leader_id as u32));
        sessions.remove(&(member_id as u32));
    }
}

#[tokio::test]
async fn e2e_meet_door_party_warp_and_walkstep_event() {
    set_eve_events_enabled(true);
    let pool = setup_test_db().await;
    let repos = SqliteRepositories::new(pool.clone());

    let leader_id = seed_account_and_char(&pool, 9703, b"MeetLead", 12001, 100, 100).await;
    let member_id = seed_account_and_char(&pool, 9704, b"MeetMem", 12001, 100, 100).await;

    let mut leader_session = Session::new();
    leader_session.id = leader_id as u32;
    assert!(repos.sessions().load(leader_id, &mut leader_session).await.unwrap());
    leader_session.id_leader = leader_session.id;
    leader_session.id_mem = [member_id as u32, 0, 0, 0];

    let mut member_session = Session::new();
    member_session.id = member_id as u32;
    assert!(repos.sessions().load(member_id, &mut member_session).await.unwrap());
    member_session.id_leader = leader_session.id;

    registry_register_for_login(&leader_session);
    registry_register_for_login(&member_session);

    let mut conn = Conn {
        session: leader_session,
        ..Default::default()
    };
    let data = build_door_test_data(None, true);
    let service = BattleService::default();
    let env = ServerEnv::with_pool(&pool);

    // 1. WalkStep into scripted encounter zone (650, 650) -> sets Quest 10888
    let _out_walk = dispatch(&mut conn, &move_frame(1, 650, 650), &data, &service, &env).await;
    assert!(
        conn.session.quest_tasks.contains_key(&10888),
        "WalkStep must trigger scripted Eve event in encounter zone"
    );

    // 2. MeetDoor by walking onto door 10 proximity (310, 405) -> warps Leader & Member to 12002
    let out_meet = dispatch(&mut conn, &move_frame(1, 310, 405), &data, &service, &env).await;
    assert_eq!((conn.session.map_id, conn.session.map_x, conn.session.map_y), (12002, 820, 940));
    assert!(has_relocate_frame(&outgoing_hexes(&out_meet), leader_id as u32, 12002));
    assert!(has_relocate_frame(&direct_hexes(&out_meet, member_id as u32), member_id as u32, 12002));

    let mut db_member = Session::new();
    db_member.id = member_id as u32;
    assert!(repos.sessions().load(member_id, &mut db_member).await.unwrap());
    assert_eq!((db_member.map_id, db_member.map_x, db_member.map_y), (12002, 820, 940));

    {
        let mut sessions = lock_online_sessions();
        sessions.remove(&(leader_id as u32));
        sessions.remove(&(member_id as u32));
    }
}

#[tokio::test]
async fn negative_door_unmet_condition_and_invalid_warp_keep_party_untouched() {
    set_eve_events_enabled(true);
    let pool = setup_test_db().await;
    let repos = SqliteRepositories::new(pool.clone());

    let leader_id = seed_account_and_char(&pool, 9705, b"NegLead", 12001, 300, 400).await;
    let member_id = seed_account_and_char(&pool, 9706, b"NegMem", 12001, 300, 400).await;

    let mut leader_session = Session::new();
    leader_session.id = leader_id as u32;
    assert!(repos.sessions().load(leader_id, &mut leader_session).await.unwrap());
    leader_session.id_leader = leader_session.id;
    leader_session.id_mem = [member_id as u32, 0, 0, 0];

    let mut member_session = Session::new();
    member_session.id = member_id as u32;
    assert!(repos.sessions().load(member_id, &mut member_session).await.unwrap());
    member_session.id_leader = leader_session.id;

    registry_register_for_login(&leader_session);
    registry_register_for_login(&member_session);

    let mut conn = Conn {
        session: leader_session,
        ..Default::default()
    };
    let service = BattleService::default();
    let env = ServerEnv::with_pool(&pool);

    // 1. Unmet door condition (requires Quest 10999)
    let data_unmet = build_door_test_data(Some(10999), true);
    let out_unmet = dispatch(&mut conn, &click_door_frame(10), &data_unmet, &service, &env).await;
    assert!(out_unmet.outgoing.iter().any(|f| f.frame == "F44402001408"));
    assert_eq!((conn.session.map_id, conn.session.map_x, conn.session.map_y), (12001, 300, 400));
    let reg_mem = registry_get(member_id as u32).unwrap();
    assert_eq!((reg_mem.map_id, reg_mem.map_x, reg_mem.map_y), (12001, 300, 400));

    // 2. Invalid warp destination (missing Warps.dat entry)
    let data_invalid_warp = build_door_test_data(None, false);
    let out_invalid = dispatch(&mut conn, &click_door_frame(10), &data_invalid_warp, &service, &env).await;
    assert!(out_invalid.outgoing.iter().any(|f| f.frame == "F44402001408"));
    assert_eq!((conn.session.map_id, conn.session.map_x, conn.session.map_y), (12001, 300, 400));

    let mut db_leader = Session::new();
    db_leader.id = leader_id as u32;
    assert!(repos.sessions().load(leader_id, &mut db_leader).await.unwrap());
    assert_eq!((db_leader.map_id, db_leader.map_x, db_leader.map_y), (12001, 300, 400));

    let mut db_member = Session::new();
    db_member.id = member_id as u32;
    assert!(repos.sessions().load(member_id, &mut db_member).await.unwrap());
    assert_eq!((db_member.map_id, db_member.map_x, db_member.map_y), (12001, 300, 400));

    {
        let mut sessions = lock_online_sessions();
        sessions.remove(&(leader_id as u32));
        sessions.remove(&(member_id as u32));
    }
}
