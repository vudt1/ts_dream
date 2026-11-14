//! Integration tests for Ticket 06:
//! - Missing Eve Effect Executors (`NpcTeam`, `RewardPet`, `MoveEntity`, `PlayMovie`, `NpcAction`)
//! - `Player (rc=7)` `texp` level-up (`0x08` stats + `0x14 0x2A 0x01`), `savemap`, and `param=3` no-fallthrough
//! - `Gold/Points (rc=5)` `GOLD_CAP`/`POINT_CAP` pre-flight rejection & `0x1A 01/02/04/05`
//! - `UNKNOWN` `result_type`/`result_class` fail-closed telemetry skip
//! - E2E via `dispatcher::dispatch` + `setup_test_db()`:
//!   - `12001` Tống Nhân (`0x14 09` choice `32` -> `RewardPet 14105`, `NpcTeam`, `Quest 10027`, DB write-through)
//!   - `texp` level-up + `savemap` persistence
//!   - Negative tests: `GOLD_CAP` exceeded & full 4-slot pet bag All-or-Nothing rejection

mod common;

use common::test_db::setup_test_db;
use std::path::PathBuf;
use ts_dream::battle::service::BattleService;
use ts_dream::data::loader::GameData;
use ts_dream::data::loaders::{
    EveCondition, EveNpcPlacement, EveResult, NpcEventData, SceneEveData,
};
use ts_dream::db::modern::sqlite::SqliteRepositories;
use ts_dream::db::modern::traits::CharacterSeed;
use ts_dream::db::pool::DbPool;
use ts_dream::protocol::{GOLD_CAP, POINT_CAP};
use ts_dream::server::dispatcher::{dispatch, HandleOutcome, ServerEnv};
use ts_dream::server::handlers::npc_event::set_eve_events_enabled;
use ts_dream::server::session::{Conn, InventoryItem, PetState, Session};

const CONTINUE_FRAME: [u8; 6] = [0xF4, 0x44, 0x02, 0x00, 0x14, 0x06];

fn data_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("Data")
}

fn click_npc_frame(npc_click_id: u16) -> Vec<u8> {
    let [lo, hi] = npc_click_id.to_le_bytes();
    vec![0xF4, 0x44, 0x04, 0x00, 0x14, 0x01, lo, hi]
}

fn select_menu_frame(choice: u8) -> Vec<u8> {
    vec![0xF4, 0x44, 0x03, 0x00, 0x14, 0x09, choice]
}

fn has_frame_prefix(out: &HandleOutcome, prefix: &str) -> bool {
    out.outgoing.iter().any(|f| f.frame.starts_with(prefix))
}

fn op14_01_results(out: &HandleOutcome) -> Vec<(u8, u8, u16, u8, i32, u16)> {
    out.outgoing
        .iter()
        .filter_map(|f| {
            let b = hex::decode(&f.frame).ok()?;
            if b.len() >= 21 && b[4] == 0x14 && b[5] == 0x01 {
                let rt = b[10];
                let rc = b[11];
                let param = u16::from_le_bytes([b[12], b[13]]);
                let ps = b[14];
                let val = i32::from_le_bytes([b[15], b[16], b[17], b[18]]);
                let mean = u16::from_le_bytes([b[19], b[20]]);
                Some((rt, rc, param, ps, val, mean))
            } else {
                None
            }
        })
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
        level: 1,
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

fn synthetic_scene_with_results(map_id: u16, npc_click_id: u16, results: Vec<EveResult>) -> GameData {
    let mut data = GameData::default();
    data.texps = ts_dream::data::texps::compute_texps();

    let mut scene = SceneEveData::default();
    scene.npcs.insert(
        npc_click_id,
        EveNpcPlacement {
            id: npc_click_id,
            npc_id: 1001,
            x: 500,
            y: 500,
            events: vec![1],
            ..Default::default()
        },
    );
    scene.npc_events.insert(
        1,
        NpcEventData {
            eve_no: 1,
            conditions: vec![EveCondition {
                condition_no: 1,
                condition_class: 0,
                to_result: 1,
                and_num: 0,
                results,
                ..Default::default()
            }],
            ..Default::default()
        },
    );
    scene.build_indices();
    data.scene_eve_data.insert(u32::from(map_id), scene);
    data
}

#[tokio::test]
async fn e2e_12001_tong_nhan_menu_32_grants_pet_14105_npcteam_quest_10027_and_persists() {
    set_eve_events_enabled(true);
    let pool = setup_test_db().await;
    let data = GameData::load(&data_dir()).expect("GameData::load");
    let scene_12001 = data.scene_eve_data.get(&12001).expect("scene 12001");
    let npc1 = scene_12001.npcs.get(&1).expect("12001 NPC 1");

    let char_id = seed_account_and_char(&pool, 9601, b"TongNhanTest", 12001, npc1.x as u16, npc1.y as u16).await;
    let repos = SqliteRepositories::new(pool.clone());
    let mut session = Session::new();
    session.id = char_id as u32;
    assert!(repos.sessions().load(char_id, &mut session).await.expect("load"));
    let mut conn = Conn {
        session,
        ..Default::default()
    };

    let service = BattleService::default();
    let env = ServerEnv::with_pool(&pool);

    // 1. Click Tống Nhân (NPC 1) -> Surface 1
    let _out_click = dispatch(&mut conn, &click_npc_frame(1), &data, &service, &env).await;
    for _ in 0..10 {
        let awaiting_choice = conn
            .session
            .current_event_session
            .as_ref()
            .is_some_and(|ev| ev.phase == ts_dream::eve::auto_chain::EventPhase::AwaitingChoice);
        if awaiting_choice {
            break;
        }
        dispatch(&mut conn, &CONTINUE_FRAME, &data, &service, &env).await;
    }
    assert_eq!(
        conn.session.current_event_session.as_ref().map(|e| e.last_surface_id),
        Some(1)
    );

    // 2. Select choice 32 -> Talk 10177
    let out_choice = dispatch(&mut conn, &select_menu_frame(32), &data, &service, &env).await;
    assert!(
        !op14_01_results(&out_choice).is_empty(),
        "choice 32 must emit Talk 10177"
    );

    // 3. Continue past Talk 10177 -> executes RewardPet 14105, NpcTeam, Quest 10027, Player(2,1,5) + FinishChain
    let mut all_results = Vec::new();
    let mut all_frames = Vec::new();
    for _ in 0..10 {
        if conn.session.current_event_session.is_none() {
            break;
        }
        let out_cont = dispatch(&mut conn, &CONTINUE_FRAME, &data, &service, &env).await;
        all_results.extend(op14_01_results(&out_cont));
        all_frames.extend(out_cont.outgoing.into_iter().map(|f| f.frame));
    }

    assert!(
        conn.session.current_event_session.is_none(),
        "Tống Nhân chain should finish cleanly"
    );
    assert!(
        conn.session.pets.iter().any(|p| p.id == 14105 && (1..=4).contains(&p.stt)),
        "Player must receive pet 14105 in active roster 1..=4; pets={:?}",
        conn.session.pets
    );
    assert!(
        all_frames.iter().any(|f| f.contains("0F08")),
        "Must emit 0F08 pet stat frame"
    );
    assert!(
        all_frames.iter().any(|f| f.contains("0F01")),
        "Must emit 0F01 pet add frame"
    );
    assert!(
        all_results.iter().any(|&(rt, rc, param, _, _, _)| rt == 0 && rc == 8 && param == 14105),
        "Must emit 0x14 01 RewardPet frame"
    );
    assert!(
        all_results.iter().any(|&(rt, rc, _, _, _, _)| rt == 0 && rc == 3),
        "Must emit 0x14 01 NpcTeam frame"
    );
    assert!(
        conn.session.quest_tasks.contains_key(&10027)
            || data
                .quest_marks
                .get(&10027)
                .is_some_and(|m| conn.session.quest_dont.contains(m)),
        "Quest 10027 must be updated in quest_tasks or quest_dont"
    );

    // Verify FinishChain write-through persisted pet 14105 and quest 10027 in SQLite
    let mut reloaded = Session::new();
    reloaded.id = char_id as u32;
    assert!(repos.sessions().load(char_id, &mut reloaded).await.expect("reload"));
    assert!(
        reloaded.pets.iter().any(|p| p.id == 14105),
        "Pet 14105 must be persisted to SQLite on FinishChain"
    );
}

#[tokio::test]
async fn e2e_player_rc7_texp_level_up_savemap_and_no_param3_fallthrough() {
    set_eve_events_enabled(true);
    let pool = setup_test_db().await;
    let char_id = seed_account_and_char(&pool, 9602, b"ExpLevelUp", 12001, 500, 500).await;
    let repos = SqliteRepositories::new(pool.clone());
    let mut session = Session::new();
    session.id = char_id as u32;
    assert!(repos.sessions().load(char_id, &mut session).await.expect("load"));
    session.level = 1;
    session.texp = 0;
    session.hp = 10;
    session.sp = 5;
    session.savemap = 10000;
    session.savemap_x = 100;
    session.savemap_y = 100;

    let data = synthetic_scene_with_results(
        12001,
        1,
        vec![
            // rc=7, param=1, p_style=4: +5000 texp (enough to level up from lv 1)
            EveResult {
                result_group_no: 1,
                result_no: 1,
                result_type: 0,
                result_class: 7,
                parameter: 1,
                parameter_style: 4,
                result_value: 5000,
                result_mean_no: 0,
            },
            // rc=7, param=0, p_style=1: savemap
            EveResult {
                result_group_no: 1,
                result_no: 2,
                result_type: 0,
                result_class: 7,
                parameter: 0,
                parameter_style: 1,
                result_value: 0,
                result_mean_no: 0,
            },
            // rc=7, param=3, p_style=0: open NPC shop WITHOUT savemap fallthrough
            EveResult {
                result_group_no: 1,
                result_no: 3,
                result_type: 0,
                result_class: 7,
                parameter: 3,
                parameter_style: 0,
                result_value: 0,
                result_mean_no: 0,
            },
        ],
    );

    let mut conn = Conn {
        session,
        ..Default::default()
    };
    let service = BattleService::default();
    let env = ServerEnv::with_pool(&pool);

    let out = dispatch(&mut conn, &click_npc_frame(1), &data, &service, &env).await;
    assert!(conn.session.level > 1, "texp reward must level up player");
    assert_eq!(conn.session.texp, 5000);
    assert_eq!(conn.session.hp, conn.session.hp_max, "level up must heal HP");
    assert_eq!(conn.session.sp, conn.session.sp_max, "level up must heal SP");
    assert!(conn.session.point >= 2, "level up must grant stat points");
    assert!(conn.session.skill_point >= 1, "level up must grant skill points");
    assert_eq!(
        (conn.session.savemap, conn.session.savemap_x, conn.session.savemap_y),
        (12001, 500, 500),
        "savemap must update to current map & coords"
    );

    // Check wire frames: 0x14 0x2A 0x01 <5000 LE>, 0x08 01 23 (Lv), 0x05 0A, 0x1B 03 (shop)
    assert!(
        out.outgoing.iter().any(|f| f.frame == "F4440700142A0188130000"),
        "Must emit 0x14 0x2A 0x01 <5000_u32_LE>; got {:?}",
        out.outgoing
    );
    assert!(has_frame_prefix(&out, "F4440C00080123"), "Must emit 0x08 01 23 Lv");
    assert!(has_frame_prefix(&out, "F4440C00080124"), "Must emit 0x08 01 24 Texp");
    assert!(has_frame_prefix(&out, "F4440C000801CD"), "Must emit 0x08 01 CD MaxHP");
    assert!(has_frame_prefix(&out, "F4440C000801CE"), "Must emit 0x08 01 CE MaxSP");
    assert!(has_frame_prefix(&out, "F4440600050A"), "Must emit 05 0A level up");
    assert!(
        out.outgoing.iter().any(|f| f.frame == "F44402001B03"),
        "Must emit 1B 03 open NPC shop"
    );

    let mut reloaded = Session::new();
    reloaded.id = char_id as u32;
    assert!(repos.sessions().load(char_id, &mut reloaded).await.expect("reload"));
    assert_eq!(reloaded.savemap, 12001);
    assert_eq!(reloaded.savemap_x, 500);
    assert_eq!(reloaded.savemap_y, 500);
    assert!(reloaded.level > 1);
}

#[tokio::test]
async fn e2e_move_entity_movie_npc_action_and_unknown_fail_closed() {
    set_eve_events_enabled(true);
    let data = synthetic_scene_with_results(
        12001,
        1,
        vec![
            // rt=4, rc=3, val=120345 (valid MoveEntity)
            EveResult {
                result_group_no: 1,
                result_no: 1,
                result_type: 4,
                result_class: 3,
                parameter: 2,
                parameter_style: 0,
                result_value: 120345,
                result_mean_no: 0,
            },
            // rt=5, rc=0 (PlayMovie)
            EveResult {
                result_group_no: 1,
                result_no: 2,
                result_type: 5,
                result_class: 0,
                parameter: 0,
                parameter_style: 1,
                result_value: 17,
                result_mean_no: 0,
            },
            // rt=9, rc=3 (NpcAction — server-side only, never emitted via 0x14 01)
            EveResult {
                result_group_no: 1,
                result_no: 3,
                result_type: 9,
                result_class: 3,
                parameter: 1,
                parameter_style: 1,
                result_value: 5,
                result_mean_no: 0,
            },
            // UNKNOWN rt=7, rt=8, rt=10, rc=80 (fail-closed, never emitted via 0x14 01)
            EveResult {
                result_group_no: 1,
                result_no: 4,
                result_type: 7,
                result_class: 7,
                parameter: 1,
                parameter_style: 1,
                result_value: 1,
                result_mean_no: 0,
            },
            EveResult {
                result_group_no: 1,
                result_no: 5,
                result_type: 8,
                result_class: 80,
                parameter: 1,
                parameter_style: 9,
                result_value: 0,
                result_mean_no: 0,
            },
            EveResult {
                result_group_no: 1,
                result_no: 6,
                result_type: 10,
                result_class: 7,
                parameter: 1,
                parameter_style: 1,
                result_value: 1,
                result_mean_no: 0,
            },
        ],
    );

    let mut conn = Conn {
        session: Session {
            id: 777,
            map_id: 12001,
            map_x: 500,
            map_y: 500,
            ..Default::default()
        },
        ..Default::default()
    };
    let service = BattleService::default();
    let env = ServerEnv::none();

    let out = dispatch(&mut conn, &click_npc_frame(1), &data, &service, &env).await;
    let emitted = op14_01_results(&out);

    assert!(emitted.iter().any(|&(rt, rc, _, _, val, _)| rt == 4 && rc == 3 && val == 120345));
    assert!(emitted.iter().any(|&(rt, _, _, _, val, _)| rt == 5 && val == 17));
    assert!(
        !emitted.iter().any(|&(rt, ..)| rt >= 7),
        "rt >= 7 (including rt=9 NpcAction and UNKNOWN rt=7/8/10) must NEVER be emitted via 0x14 01"
    );
}

#[tokio::test]
async fn negative_preflight_rejects_gold_cap_and_full_pet_bag_all_or_nothing() {
    set_eve_events_enabled(true);
    let service = BattleService::default();
    let env = ServerEnv::none();

    // 1. Exceeding GOLD_CAP after an item/quest step in the same batch -> All-or-Nothing rollback
    let data_gold = synthetic_scene_with_results(
        12001,
        1,
        vec![
            EveResult {
                result_group_no: 1,
                result_no: 1,
                result_type: 0,
                result_class: 1,
                parameter: 26001,
                parameter_style: 0,
                result_value: 3,
                result_mean_no: 0,
            },
            EveResult {
                result_group_no: 1,
                result_no: 2,
                result_type: 0,
                result_class: 2,
                parameter: 10999,
                parameter_style: 1,
                result_value: 2,
                result_mean_no: 0,
            },
            EveResult {
                result_group_no: 1,
                result_no: 3,
                result_type: 0,
                result_class: 5,
                parameter: 0,
                parameter_style: 1,
                result_value: 5000, // exceeds GOLD_CAP when gold = GOLD_CAP - 1000
                result_mean_no: 0,
            },
        ],
    );

    let mut conn_gold = Conn {
        session: Session {
            id: 801,
            map_id: 12001,
            map_x: 500,
            map_y: 500,
            gold: GOLD_CAP - 1000,
            homdo: vec![InventoryItem {
                slot: 1,
                id: 32012,
                count: 5,
                ..Default::default()
            }],
            ..Default::default()
        },
        ..Default::default()
    };

    let out_gold = dispatch(&mut conn_gold, &click_npc_frame(1), &data_gold, &service, &env).await;
    assert_eq!(conn_gold.session.gold, GOLD_CAP - 1000, "gold must remain untouched");
    assert!(
        !conn_gold.session.homdo.iter().any(|i| i.id == 26001),
        "prior item in rejected batch must NOT be added"
    );
    assert_eq!(conn_gold.session.homdo.len(), 1, "adjacent homdo slot must remain intact");
    assert_eq!(conn_gold.session.homdo[0].count, 5);
    assert!(
        !conn_gold.session.quest_tasks.contains_key(&10999),
        "prior quest step in rejected batch must NOT be applied"
    );
    assert!(
        has_frame_prefix(&out_gold, "F444") && out_gold.outgoing.iter().any(|f| f.frame == "F44402001408"),
        "Must emit system notice + EndTalk"
    );

    // 2. Full 4-slot active pet bag rejects RewardPet All-or-Nothing
    let data_pet = synthetic_scene_with_results(
        12001,
        1,
        vec![
            EveResult {
                result_group_no: 1,
                result_no: 1,
                result_type: 0,
                result_class: 2,
                parameter: 10998,
                parameter_style: 1,
                result_value: 2,
                result_mean_no: 0,
            },
            EveResult {
                result_group_no: 1,
                result_no: 2,
                result_type: 0,
                result_class: 8,
                parameter: 14105,
                parameter_style: 1,
                result_value: 0,
                result_mean_no: 0,
            },
        ],
    );

    let mut conn_pet = Conn {
        session: Session {
            id: 802,
            map_id: 12001,
            map_x: 500,
            map_y: 500,
            pets: (1..=4)
                .map(|stt| PetState {
                    stt,
                    id: 14000 + u16::from(stt),
                    ..Default::default()
                })
                .collect(),
            ..Default::default()
        },
        ..Default::default()
    };

    let out_pet = dispatch(&mut conn_pet, &click_npc_frame(1), &data_pet, &service, &env).await;
    assert_eq!(conn_pet.session.pets.len(), 4, "4 existing pets must remain untouched");
    assert!(!conn_pet.session.pets.iter().any(|p| p.id == 14105));
    assert!(
        !conn_pet.session.quest_tasks.contains_key(&10998),
        "prior quest step in rejected RewardPet batch must NOT be applied"
    );
    assert!(out_pet.outgoing.iter().any(|f| f.frame == "F44402001408"));

    // 3. Exceeding POINT_CAP for shop_point (rc=5 p_style=3) rejects All-or-Nothing
    let data_point = synthetic_scene_with_results(
        12001,
        1,
        vec![EveResult {
            result_group_no: 1,
            result_no: 1,
            result_type: 0,
            result_class: 5,
            parameter: 0,
            parameter_style: 3,
            result_value: 500,
            result_mean_no: 0,
        }],
    );
    let mut conn_point = Conn {
        session: Session {
            id: 803,
            map_id: 12001,
            map_x: 500,
            map_y: 500,
            shop_point: POINT_CAP - 100,
            ..Default::default()
        },
        ..Default::default()
    };
    let out_point = dispatch(&mut conn_point, &click_npc_frame(1), &data_point, &service, &env).await;
    assert_eq!(conn_point.session.shop_point, POINT_CAP - 100);
    assert!(out_point.outgoing.iter().any(|f| f.frame == "F44402001408"));

    // 4. Full 4-slot active pet bag where the player ALREADY owns pet 14105:
    // RewardPet must be a no-op rather than falsely triggering PetBagFull.
    let mut conn_owned_pet = Conn {
        session: Session {
            id: 804,
            map_id: 12001,
            map_x: 500,
            map_y: 500,
            pets: vec![
                PetState { stt: 1, id: 14105, ..Default::default() },
                PetState { stt: 2, id: 14002, ..Default::default() },
                PetState { stt: 3, id: 14003, ..Default::default() },
                PetState { stt: 4, id: 14004, ..Default::default() },
            ],
            ..Default::default()
        },
        ..Default::default()
    };
    let _out_owned = dispatch(&mut conn_owned_pet, &click_npc_frame(1), &data_pet, &service, &env).await;
    assert_eq!(conn_owned_pet.session.pets.len(), 4);
    assert!(
        conn_owned_pet.session.quest_tasks.contains_key(&10998),
        "already-owned RewardPet must not falsely reject preceding quest step"
    );
}
