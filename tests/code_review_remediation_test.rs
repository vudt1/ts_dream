//! Integration tests for Ticket 09 (Code Review Remediation of Tickets 01–08).
//!
//! Covers:
//! 1. Pre-Flight Probe All-or-Nothing across `TalkStep` + `InsufficientItem`
//! 2. `POINT_CAP = 2_000_000_000`, `0x1A 05`/`0x1A 06` (no `0x1A 03`), and `0x14 01` on `MutateItem`
//! 3. `PetStorageType` isolation (`Carried`, `Cart`, `Hotel`) in `pet_actions` & `RewardPet`
//! 4. `registry_update_if_online` on `rc=1`/`rc=2`, multi-session atomic SQLite tx, & Warp DB rollback
//! 5. `Warp` `completed_eve_counts` + trailing effects, `WalkStep` (`16`) `AutoChain` & `0x14 09`
//! 6. `evaluator.rs` `cc=1` & `cc=7 (param=0,2)` truth table preservation

mod common;

use common::test_db::setup_test_db;
use ts_dream::battle::service::BattleService;
use ts_dream::data::loader::GameData;
use ts_dream::data::tables::Warp;
use ts_dream::data::loaders::{
    EveCondition, EveEncounterPlacement, EveResult, NpcEventData, SceneEveData,
};
use ts_dream::db::modern::model::PetStorageType;
use ts_dream::db::persist::persist_sessions_transaction;
use ts_dream::db::pool::DbPool;
use ts_dream::eve::auto_chain::{
    AutoChainResult, EveAutoChainEngine, EventPhase, EventSession, EventTriggerKind,
};
use ts_dream::eve::evaluator::evaluate;
use ts_dream::eve::state::PlayerEventState;
use ts_dream::protocol::codecs::npc_talk::NpcTalkCodec;
use ts_dream::protocol::encoder;
use ts_dream::server::dispatcher::{dispatch, HandleOutcome, ServerEnv};
use ts_dream::server::handlers::npc_event::set_eve_events_enabled;
use ts_dream::server::handlers::quest::perform_party_warp_and_persist;
use ts_dream::server::handlers::talk::execute_event_step;
use ts_dream::server::session::{
    lock_online_sessions, registry_get, registry_register_for_login, Conn, InventoryItem,
    PetState, Session,
};

fn make_mut_item(item_id: u16, delta: i32) -> EveResult {
    EveResult {
        result_type: 0,
        result_class: 1,
        parameter: item_id,
        result_value: delta,
        ..Default::default()
    }
}

fn make_talk_step(mean_no: u16) -> EveResult {
    EveResult {
        result_type: 1,
        result_mean_no: mean_no,
        ..Default::default()
    }
}

fn make_money_result(p_style: u8, amount: i32) -> EveResult {
    EveResult {
        result_type: 0,
        result_class: 5,
        parameter_style: p_style,
        result_value: amount,
        ..Default::default()
    }
}

fn make_quest_result(quest_id: u16, p_style: u8, val: i32) -> EveResult {
    EveResult {
        result_type: 0,
        result_class: 2,
        parameter: quest_id,
        parameter_style: p_style,
        result_value: val,
        ..Default::default()
    }
}

fn make_warp_result(warp_id: u16) -> EveResult {
    EveResult {
        result_type: 2,
        parameter: warp_id,
        ..Default::default()
    }
}

async fn seed_char(pool: &DbPool, id: i64, name: &[u8], map_id: u16) {
    sqlx::query("INSERT INTO accounts (playerid, pass1, pass2, createdat) VALUES (?, '1', '1', 0)")
        .bind(id)
        .execute(&pool.write)
        .await
        .expect("insert account");
    sqlx::query(
        "INSERT INTO characters (playerid, name, level, gender, hair, element, mapid, mapx, mapy) \
         VALUES (?, ?, 10, 1, 1, 1, ?, 100, 100)",
    )
    .bind(id)
    .bind(name)
    .bind(i64::from(map_id))
    .execute(&pool.write)
    .await
    .expect("insert char");
    sqlx::query("INSERT INTO character_money (playerid) VALUES (?)")
        .bind(id)
        .execute(&pool.write)
        .await
        .expect("insert money");
}

#[test]
fn test_preflight_walks_through_talk_step_and_checks_item_sufficiency() {
    let data = GameData::default();
    let mut sess = Session::new();
    sess.id = 88001;
    // Fill all 25 homdo slots; slot 1 has 1x item 50001.
    for slot in 1..=25u8 {
        sess.homdo.push(InventoryItem {
            slot,
            id: if slot == 1 { 50001 } else { 51000 + u16::from(slot) },
            count: 1,
            ..Default::default()
        });
    }
    // Wait: removing 1x 50001 frees slot 1, so rewarding 1x 50002 fits!
    // Instead, give 2x 50001 in slot 1 and take 1x 50001 (slot 1 stays occupied!),
    // then after TalkStep reward 1x 50002 -> homdo is STILL full -> MUST reject All-or-Nothing!
    sess.homdo[0].count = 2;
    sess.current_event_session = Some(EventSession {
        map_id: 12001,
        eve_no: 1,
        npc_click_id: 1,
        trigger_kind: EventTriggerKind::ClickNpc,
        chain_depth: 0,
        results: vec![
            make_mut_item(50001, -1),
            make_talk_step(100),
            make_mut_item(50002, 1),
        ],
        current_index: 0,
        phase: EventPhase::Executing,
        last_surface_id: -1,
        last_choice_code: -1,
        battle_result: 0,
        matched_condition_no: 1,
    });

    let mut out = HandleOutcome::default();
    execute_event_step(&mut sess, &data, &mut out);

    // Must be rejected up front: item 50001 count must still be 2, no TalkStep (14 01) sent.
    assert_eq!(sess.homdo[0].count, 2, "must not deduct input item when post-TalkStep reward overflows");
    assert!(
        !out.outgoing.iter().any(|f| f.frame.contains("1401")),
        "must not emit TalkStep when pre-flight rejects post-TalkStep reward"
    );

    // Negative: Insufficient input item (needs 5, has 2)
    sess.homdo.truncate(1);
    sess.gold = 500;
    sess.current_event_session = Some(EventSession {
        map_id: 12001,
        eve_no: 2,
        npc_click_id: 1,
        trigger_kind: EventTriggerKind::ClickNpc,
        chain_depth: 0,
        results: vec![
            make_mut_item(50001, -5),
            make_talk_step(101),
            make_money_result(1, 1000),
        ],
        current_index: 0,
        phase: EventPhase::Executing,
        last_surface_id: -1,
        last_choice_code: -1,
        battle_result: 0,
        matched_condition_no: 1,
    });
    let mut out2 = HandleOutcome::default();
    execute_event_step(&mut sess, &data, &mut out2);
    assert_eq!(sess.homdo[0].count, 2, "must not partially deduct insufficient item");
    assert_eq!(sess.gold, 500, "must not grant gold on insufficient item");
}

#[test]
fn test_point_cap_wire_0x1a_05_06_and_item_0x14_01() {
    assert_eq!(ts_dream::protocol::POINT_CAP, 2_000_000_000);

    let data = GameData::default();
    let mut sess = Session::new();
    sess.id = 88002;
    sess.shop_point = 15_000_000; // > old 9_999_999 cap, < 2_000_000_000

    let add_res = make_money_result(3, 500);
    let sub_res = make_money_result(4, 200);
    let item_res = make_mut_item(26041, 1);

    sess.current_event_session = Some(EventSession {
        map_id: 12001,
        eve_no: 3,
        npc_click_id: 1,
        trigger_kind: EventTriggerKind::ClickNpc,
        chain_depth: 0,
        results: vec![add_res, sub_res, item_res.clone()],
        current_index: 0,
        phase: EventPhase::Executing,
        last_surface_id: -1,
        last_choice_code: -1,
        battle_result: 0,
        matched_condition_no: 1,
    });

    let mut out = HandleOutcome::default();
    execute_event_step(&mut sess, &data, &mut out);

    assert_eq!(sess.shop_point, 15_000_300);
    let expected_add = format!("F44406001A05{}", encoder::le32(500));
    let expected_sub = format!("F44406001A06{}", encoder::le32(200));
    let expected_item_1401 = NpcTalkCodec::build_eve_result_hex(&item_res);

    assert!(out.outgoing.iter().any(|f| f.frame == expected_add), "must emit 0x1A 05");
    assert!(out.outgoing.iter().any(|f| f.frame == expected_sub), "must emit 0x1A 06");
    assert!(
        !out.outgoing.iter().any(|f| f.frame.starts_with("F44406001A03")),
        "must NEVER emit 0x1A 03 (gold-cap error) on shop_point add/sub"
    );
    assert!(
        out.outgoing.iter().any(|f| f.frame == expected_item_1401),
        "MutateItem must emit 0x14 01 alongside 0x17 05"
    );

    // Negative: shop_point exceeding 2_000_000_000 rejected by pre-flight
    sess.shop_point = 1_999_999_900;
    sess.current_event_session = Some(EventSession {
        map_id: 12001,
        eve_no: 4,
        npc_click_id: 1,
        trigger_kind: EventTriggerKind::ClickNpc,
        chain_depth: 0,
        results: vec![make_money_result(3, 200)],
        current_index: 0,
        phase: EventPhase::Executing,
        last_surface_id: -1,
        last_choice_code: -1,
        battle_result: 0,
        matched_condition_no: 1,
    });
    let mut out_cap = HandleOutcome::default();
    execute_event_step(&mut sess, &data, &mut out_cap);
    assert_eq!(sess.shop_point, 1_999_999_900, "must reject POINT_CAP overflow");
}

#[tokio::test]
async fn test_pet_storage_type_isolation_in_actions_and_reward_pet() {
    let data = GameData::default();
    let service = BattleService::new(std::sync::Arc::new(data.clone()));
    let env = ServerEnv::none();

    let mut conn = Conn::default();
    conn.session.id = 88003;
    conn.session.logined = true;
    conn.session.in_world = true;
    // Cart pet at slot 1 (stt=5), Hotel pet at slot 1 (stt=5), Carried pet at slot 1 (stt=1).
    conn.session.pets = vec![
        PetState {
            stt: 5,
            storage_type: PetStorageType::Cart,
            id: 11111,
            ..Default::default()
        },
        PetState {
            stt: 5,
            storage_type: PetStorageType::Hotel,
            id: 22222,
            ..Default::default()
        },
        PetState {
            stt: 1,
            storage_type: PetStorageType::Carried,
            id: 33333,
            ..Default::default()
        },
    ];
    assert!(!conn.session.pets[0].is_carried());
    assert!(!conn.session.pets[1].is_carried());
    assert!(conn.session.pets[2].is_carried());
    assert_eq!(conn.session.pets[0].effective_storage_and_slot(), (2, 1));
    assert_eq!(conn.session.pets[1].effective_storage_and_slot(), (3, 1));
    assert_eq!(conn.session.pets[2].effective_storage_and_slot(), (1, 1));

    // Withdraw Hotel slot 1 (0x0F sub 3, payload [1]) -> must withdraw 22222 (Hotel), NOT 11111 (Cart)
    let out = dispatch(&mut conn, &[0xF4, 0x44, 0x03, 0x00, 0x0F, 0x03, 0x01], &data, &service, &env).await;
    assert!(!out.outgoing.is_empty());
    let cart_pet = conn.session.pets.iter().find(|p| p.id == 11111).unwrap();
    assert_eq!(cart_pet.storage_type, PetStorageType::Cart);
    assert_eq!(cart_pet.stt, 5);
    let hotel_pet = conn.session.pets.iter().find(|p| p.id == 22222).unwrap();
    assert_eq!(hotel_pet.storage_type, PetStorageType::Carried);
    assert_eq!(hotel_pet.stt, 2);

    // Deposit Carried slot 1 (33333) into Hotel (0x0F sub 7, payload [1]) -> must get stt=5 in Hotel
    let _ = dispatch(&mut conn, &[0xF4, 0x44, 0x03, 0x00, 0x0F, 0x07, 0x01], &data, &service, &env).await;
    let deposited = conn.session.pets.iter().find(|p| p.id == 33333).unwrap();
    assert_eq!(deposited.storage_type, PetStorageType::Hotel);
    assert_eq!(deposited.stt, 5);

    // RewardPet p_style=2 for 11111 (only in Cart, NOT Carried) must NOT remove Cart pet
    conn.session.current_event_session = Some(EventSession {
        map_id: 12001,
        eve_no: 5,
        npc_click_id: 1,
        trigger_kind: EventTriggerKind::ClickNpc,
        chain_depth: 0,
        results: vec![EveResult {
            result_type: 0,
            result_class: 8,
            parameter: 11111,
            parameter_style: 2,
            ..Default::default()
        }],
        current_index: 0,
        phase: EventPhase::Executing,
        last_surface_id: -1,
        last_choice_code: -1,
        battle_result: 0,
        matched_condition_no: 1,
    });
    let mut out_rp = HandleOutcome::default();
    execute_event_step(&mut conn.session, &data, &mut out_rp);
    assert!(conn.session.pets.iter().any(|p| p.id == 11111 && p.storage_type == PetStorageType::Cart));
}

#[tokio::test]
async fn test_registry_sync_atomic_party_tx_and_warp_rollback() {
    let pool = setup_test_db().await;
    seed_char(&pool, 88010, b"Leader", 12001).await;
    seed_char(&pool, 88011, b"Member", 12001).await;

    let mut s1 = Session::new();
    s1.id = 88010;
    s1.db_character_id = 88010;
    s1.map_id = 12001;
    s1.level = 20;
    s1.id_leader = 88010;
    s1.id_mem = [88010, 88011, 0, 0];

    let mut s2 = Session::new();
    s2.id = 88011;
    s2.db_character_id = 88011;
    s2.map_id = 12001;
    s2.level = 20;
    s2.id_leader = 88010;

    lock_online_sessions().clear();
    registry_register_for_login(&s1);
    registry_register_for_login(&s2);

    // 1. Registry sync on rc=1 & rc=2 mid-chain pause at TalkStep
    let data = GameData::default();
    s1.current_event_session = Some(EventSession {
        map_id: 12001,
        eve_no: 6,
        npc_click_id: 1,
        trigger_kind: EventTriggerKind::ClickNpc,
        chain_depth: 0,
        results: vec![
            make_mut_item(26041, 2),
            make_quest_result(10001, 1, 1),
            make_talk_step(200),
        ],
        current_index: 0,
        phase: EventPhase::Executing,
        last_surface_id: -1,
        last_choice_code: -1,
        battle_result: 0,
        matched_condition_no: 1,
    });
    let mut out = HandleOutcome::default();
    execute_event_step(&mut s1, &data, &mut out);
    let reg_s1 = registry_get(88010).expect("s1 in registry");
    assert!(reg_s1.homdo.iter().any(|i| i.id == 26041 && i.count == 2), "rc=1 must sync registry");
    assert_eq!(reg_s1.quest_tasks.get(&10001).map(|&(_, st)| st), Some(1), "rc=2 must sync registry");

    // 2. Atomic multi-session SQLite transaction rollback across s1 & s2
    sqlx::raw_sql(
        "CREATE TRIGGER fail_s2_save BEFORE UPDATE ON characters WHEN NEW.playerid = 88011
         BEGIN
             SELECT RAISE(ABORT, 'injected party save failure');
         END;",
    )
    .execute(&pool.write)
    .await
    .expect("create fail trigger");

    s1.level = 99;
    s2.level = 99;
    let ok = persist_sessions_transaction(Some(&pool), &[&s1, &s2], &[]).await;
    assert!(!ok, "multi-session transaction must fail when s2 fails");

    let db_lv1: i64 = sqlx::query_scalar("SELECT level FROM characters WHERE playerid = 88010")
        .fetch_one(&pool.read)
        .await
        .unwrap();
    assert_eq!(db_lv1, 10, "s1 update must be rolled back atomically with s2");

    // 3. perform_party_warp_and_persist rollback on DB failure
    let mut data_warp = GameData::default();
    data_warp.warps.insert(
        (12001, 1),
        Warp {
            map1: 12001,
            warpid: 1,
            map2: 12002,
            x: 300,
            y: 400,
        },
    );
    s1.map_id = 12001;
    s1.map_x = 100;
    s1.map_y = 100;
    registry_register_for_login(&s1);

    let mut warp_out = HandleOutcome::default();
    let warped = perform_party_warp_and_persist(&mut s1, &data_warp, Some(&pool), 1, &mut warp_out).await;
    assert!(!warped, "party warp must return false on DB error");
    assert_eq!(s1.map_id, 12001, "RAM map_id must rollback on DB error");
    assert_eq!(s1.map_x, 100, "RAM map_x must rollback on DB error");
    assert!(warp_out.outgoing.is_empty(), "must not emit 14 07 / 0x0C on DB error");
    assert_eq!(registry_get(88010).unwrap().map_id, 12001, "registry must not update on DB error");
}

#[tokio::test]
async fn test_warp_event_count_and_walkstep_autochain_and_menu() {
    set_eve_events_enabled(true);
    let mut data = GameData::default();
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
    let mut scene = SceneEveData::default();
    scene.encounters.push(EveEncounterPlacement {
        id: 77,
        events: vec![10, 11],
        ..Default::default()
    });
    scene.npc_events.insert(
        10,
        NpcEventData {
            eve_no: 10,
            conditions: vec![EveCondition {
                condition_no: 1,
                condition_class: 0,
                and_num: 1,
                results: vec![make_mut_item(26041, 1)],
                ..Default::default()
            }],
            ..Default::default()
        },
    );
    scene.npc_events.insert(
        11,
        NpcEventData {
            eve_no: 11,
            conditions: vec![EveCondition {
                condition_no: 1,
                condition_class: 12,
                condition_parameter: 0,
                condition_parameter_style: 1,
                condition_ops: 0,
                condition_value: 0,
                and_num: 1,
                results: vec![make_talk_step(500)],
                ..Default::default()
            }],
            ..Default::default()
        },
    );
    scene.build_indices();
    data.scene_eve_data.insert(12001, scene.clone());

    // 1. WalkStep (16) AutoChain
    let completed = EventSession {
        map_id: 12001,
        eve_no: 10,
        npc_click_id: 77,
        trigger_kind: EventTriggerKind::WalkStep,
        chain_depth: 0,
        results: vec![make_mut_item(26041, 1)],
        current_index: 1,
        phase: EventPhase::Executing,
        last_surface_id: -1,
        last_choice_code: -1,
        battle_result: 0,
        matched_condition_no: 1,
    };
    let mut state = PlayerEventState::default();
    state.completed_eve_counts.insert(10, 1);
    let mut rng = ts_dream::battle::rng::DotNetRandom::new(1);
    match EveAutoChainEngine::try_auto_chain(&scene, &completed, &state, &mut rng) {
        AutoChainResult::Chained(next) => assert_eq!(next.eve_no, 11, "WalkStep must AutoChain to eve 11"),
        AutoChainResult::NoMatch => panic!("WalkStep AutoChain failed"),
    }

    // 2. Warp increments completed_eve_counts when state-changing
    let mut sess = Session::new();
    sess.id = 88020;
    sess.map_id = 12001;
    sess.current_event_session = Some(EventSession {
        map_id: 12001,
        eve_no: 20,
        npc_click_id: 1,
        trigger_kind: EventTriggerKind::ClickDoor,
        chain_depth: 0,
        results: vec![make_mut_item(26041, 1), make_warp_result(1)],
        current_index: 0,
        phase: EventPhase::Executing,
        last_surface_id: -1,
        last_choice_code: -1,
        battle_result: 0,
        matched_condition_no: 1,
    });
    let mut out = HandleOutcome::default();
    ts_dream::server::handlers::talk::execute_event_step_with_pool(&mut sess, &data, None, &mut out)
        .await;
    assert_eq!(sess.map_id, 12002);
    assert_eq!(
        sess.completed_eve_counts.get(&(12001, 20)),
        Some(&1),
        "Warp chain with state change must increment completed_eve_counts"
    );

    // 3. WalkStep 0x14 09 menu choice resolves by encounter_id (even with overlapping zones)
    let mut data_menu = GameData::default();
    let mut s_menu = SceneEveData::default();
    s_menu.encounters.push(EveEncounterPlacement {
        id: 1,
        start_x: 0,
        start_y: 0,
        end_x: 500,
        end_y: 500,
        events: vec![1],
        ..Default::default()
    });
    s_menu.encounters.push(EveEncounterPlacement {
        id: 2,
        start_x: 0,
        start_y: 0,
        end_x: 500,
        end_y: 500,
        events: vec![2],
        ..Default::default()
    });
    s_menu.npc_events.insert(
        2,
        NpcEventData {
            eve_no: 2,
            conditions: vec![EveCondition {
                condition_no: 1,
                condition_class: 10,
                condition_parameter: 900,
                condition_parameter_style: 30,
                and_num: 1,
                results: vec![make_money_result(1, 1000)],
                ..Default::default()
            }],
            ..Default::default()
        },
    );
    s_menu.build_indices();
    data_menu.scene_eve_data.insert(12001, s_menu);

    let mut conn = Conn::default();
    conn.session.id = 88021;
    conn.session.map_id = 12001;
    conn.session.map_x = 100;
    conn.session.map_y = 100;
    conn.session.current_event_session = Some(EventSession {
        map_id: 12001,
        eve_no: 2,
        npc_click_id: 2,
        trigger_kind: EventTriggerKind::WalkStep,
        chain_depth: 0,
        results: vec![EveResult {
            result_type: 6,
            result_mean_no: 900,
            ..Default::default()
        }],
        current_index: 0,
        phase: EventPhase::AwaitingChoice,
        last_surface_id: 900,
        last_choice_code: -1,
        battle_result: 0,
        matched_condition_no: 1,
    });
    let svc = BattleService::new(std::sync::Arc::new(data_menu.clone()));
    let env = ServerEnv::none();
    let _ = dispatch(&mut conn, &[0xF4, 0x44, 0x03, 0x00, 0x14, 0x09, 30], &data_menu, &svc, &env).await;
    assert_eq!(conn.session.gold, 1000, "WalkStep 0x14 09 must target encounter_id 2 directly");
}

#[test]
fn test_evaluator_emg_reversed_ops_truth_table() {
    let mut state = PlayerEventState::default();
    state.bag_items.insert(26041, 3);
    state.level = 15;
    state.reborn_count = 1;

    // cc=1: val=0, ops=2 (<) means 0 < qty -> qty > 0 -> true
    let has_item = EveCondition {
        condition_class: 1,
        condition_parameter: 26041,
        condition_ops: 2,
        condition_value: 0,
        ..Default::default()
    };
    assert!(evaluate(&has_item, &state));

    // cc=1: val=0, ops=5 (>=) means 0 >= qty -> qty == 0 -> false
    let no_item = EveCondition {
        condition_class: 1,
        condition_parameter: 26041,
        condition_ops: 5,
        condition_value: 0,
        ..Default::default()
    };
    assert!(!evaluate(&no_item, &state));

    // cc=7 param=0: val=10, ops=3 (<=) means 10 <= level -> level >= 10 -> true
    let min_level = EveCondition {
        condition_class: 7,
        condition_parameter: 0,
        condition_ops: 3,
        condition_value: 10,
        ..Default::default()
    };
    assert!(evaluate(&min_level, &state));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_battle_ended_offline_parked_warp_persistence() {
    use std::sync::Arc;
    use tokio::sync::RwLock;
    use ts_dream::data::loaders::{EveFightData, EveFightEnemy};
    use ts_dream::data::tables::Npc;

    let pool = setup_test_db().await;
    seed_char(&pool, 88030, b"OfflineHero", 12001).await;

    let mut data = GameData::default();
    data.warps.insert(
        (12001, 1),
        Warp {
            map1: 12001,
            warpid: 1,
            map2: 12002,
            x: 350,
            y: 450,
        },
    );
    data.npcs.insert(
        9001,
        Npc {
            id: 9001,
            hp: 1,
            lv: 1,
            agi: 1,
            ..Default::default()
        },
    );
    let mut scene = SceneEveData::default();
    scene.fight_datas.insert(
        1,
        EveFightData {
            eve_no: 1,
            fight_type: 1,
            left_enemies: vec![EveFightEnemy {
                npc_id: 9001,
                location_pos: 7,
                ..Default::default()
            }],
            ..Default::default()
        },
    );
    data.scene_eve_data.insert(12001, scene);

    let mut service = BattleService::new(Arc::new(data.clone())).with_pool(pool.clone());
    service.set_input_timeout(std::time::Duration::from_millis(80));

    let mut sess = Session::new();
    sess.id = 88030;
    sess.db_character_id = 88030;
    sess.map_id = 12001;
    sess.map_x = 100;
    sess.map_y = 100;
    sess.level = 50;
    sess.hp = 500;
    sess.hp_max = 500;
    sess.atk = 500;
    sess.agi = 200;
    sess.current_event_session = Some(EventSession {
        map_id: 12001,
        eve_no: 50,
        npc_click_id: 1,
        trigger_kind: EventTriggerKind::ClickNpc,
        chain_depth: 0,
        results: vec![
            EveResult {
                result_type: 3,
                parameter: 1,
                ..Default::default()
            },
            make_warp_result(1),
        ],
        current_index: 0,
        phase: EventPhase::AwaitingBattle,
        last_surface_id: -1,
        last_choice_code: -1,
        battle_result: 0,
        matched_condition_no: 1,
    });

    let sess_arc = Arc::new(RwLock::new(sess.clone()));
    let _rx = service.register(88030, Arc::clone(&sess_arc));
    let bid = service.start_eve_encounter(&mut sess, 1, 112);
    assert!(bid > 0, "must start eve encounter");
    {
        let mut s = sess_arc.write().await;
        s.battle_id = bid;
    }

    // Submit attack command (0x32 sub 0x01: row 3, col 2 -> target row 1, col 2, skill 10000)
    // and immediately disconnect (`unregister`) before battle finishes.
    let mut conn = Conn {
        session: sess_arc.read().await.clone(),
        ..Default::default()
    };
    let env = ServerEnv::none();
    let atk_frame = [
        0xF4, 0x44, 0x0B, 0x00, 0x32, 0x01, 0x03, 0x02, 0x01, 0x02, 0x10, 0x27, 0x00, 0x00, 0x00,
    ];
    let _ = dispatch(&mut conn, &atk_frame, &data, &service, &env).await;
    service.unregister(88030);

    // Wait for turn resolution + battle_ended + DB persistence.
    let mut persisted = (0i64, 0i64, 0i64);
    for _ in 0..30 {
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        persisted = sqlx::query_as(
            "SELECT mapid, mapx, mapy FROM characters WHERE playerid = 88030",
        )
        .fetch_one(&pool.read)
        .await
        .unwrap();
        if persisted.0 == 12002 {
            break;
        }
    }
    assert_eq!(
        persisted,
        (12002, 350, 450),
        "disconnected participant must have post-battle eve_warp coordinates persisted to SQLite"
    );
}
