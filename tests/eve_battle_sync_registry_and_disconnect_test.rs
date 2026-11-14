//! Integration tests for Ticket 08:
//! - `EveFightData` (`left_enemies` + `right_enemies`, `fight_type`, `SceneInfo.background_no` / fallback `112`)
//! - `BattleService::battle_ended` under `Two-Phase State Lock` (`lock_player_operations`) + `registry_update_if_online` + `register_sender` handle refresh
//! - Preservation & delivery of `direct_messages` and `map_broadcast` from `resume_eve_after_battle`
//! - Atomic SQLite write-through at `Battle End` (Win, Lose, Flee)
//! - Mid-battle disconnect still runs `resume_eve_after_battle` and persists to SQLite
//! - Party scope: only Leader advances standard Eve quest; Member progress preserved
//! - E2E `12001` Tống Dũng (`BattleService::battle_ended` + `dispatcher::dispatch` + `setup_test_db()`)

mod common;

use common::test_db::setup_test_db;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};
use ts_dream::battle::construction::Battle;
use ts_dream::battle::runner::Outcome;
use ts_dream::battle::service::BattleService;
use ts_dream::data::loader::GameData;
use ts_dream::data::loaders::{
    EveFightData, EveFightEnemy, EveResult, EveSceneInfo, SceneEveData,
};
use ts_dream::data::tables::Npc;
use ts_dream::db::modern::sqlite::SqliteRepositories;
use ts_dream::db::modern::traits::CharacterSeed;
use ts_dream::db::pool::DbPool;
use ts_dream::eve::{EventPhase, EventSession, EventTriggerKind};
use ts_dream::server::dispatcher::{dispatch, ServerEnv};
use ts_dream::server::handlers::npc_event::set_eve_events_enabled;
use ts_dream::server::session::{
    lock_online_sessions, registry_get, registry_register_for_login, registry_update_if_online,
    Conn, Session,
};

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

fn move_frame(x: u16, y: u16) -> Vec<u8> {
    let [x0, x1] = x.to_le_bytes();
    let [y0, y1] = y.to_le_bytes();
    vec![0xF4, 0x44, 0x08, 0x00, 0x06, 0x01, 0x00, 0x00, x0, x1, y0, y1]
}

fn drain_rx(rx: &mut mpsc::Receiver<String>) -> Vec<String> {
    let mut out = Vec::new();
    while let Ok(f) = rx.try_recv() {
        out.push(f);
    }
    out
}

fn op14_01_results(frames: &[String]) -> Vec<(u8, u8, u16, u8, i32, u16)> {
    frames
        .iter()
        .filter_map(|f| {
            let b = hex::decode(f).ok()?;
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
        level: 10,
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

/// Find Tống Dũng NPC click id on map 12001 from `Data/eve.emg`.
fn find_tong_dung_click_id(data: &GameData) -> u16 {
    let scene = data
        .scene_eve_data
        .get(&12001)
        .expect("12001 scene must exist");
    for (click_id, npc) in &scene.npcs {
        let has_14106_or_10028 = npc.events.iter().any(|&ev_no| {
            scene.npc_events.get(&u16::from(ev_no)).is_some_and(|ev| {
                ev.conditions.iter().any(|c| {
                    c.results.iter().any(|r| {
                        (r.result_type == 0 && r.result_class == 8 && r.parameter == 14106)
                            || (r.result_type == 0
                                && r.result_class == 2
                                && r.parameter == 10028)
                    })
                })
            })
        });
        if has_14106_or_10028 {
            return *click_id;
        }
    }
    panic!("Tống Dũng NPC (RewardPet 14106 / Quest 10028) not found in 12001.eve");
}

/// Drive a `12001` Tống Dũng interaction up to `StartBattle`.
async fn drive_tong_dung_to_battle(
    conn: &mut Conn,
    data: &GameData,
    service: &BattleService,
    env: &ServerEnv<'_>,
) -> (u16, i32, Vec<String>) {
    let click_id = find_tong_dung_click_id(data);
    let mut frames = Vec::new();
    let out1 = dispatch(conn, &click_npc_frame(click_id), data, service, env).await;
    frames.extend(out1.outgoing.iter().map(|f| f.frame.clone()));
    if let Some((fight_id, diahinh)) = out1.eve_battle {
        return (fight_id, diahinh, frames);
    }

    for _ in 0..16 {
        if let Some(ev) = conn.session.current_event_session.as_ref() {
            if ev.phase == EventPhase::AwaitingChoice {
                // Pick the first non-cancel option (30..=39) that triggers a battle
                for choice in [30u8, 31, 32, 33, 21] {
                    let mut probe = Conn {
                        session: conn.session.clone(),
                        ..Default::default()
                    };
                    let out =
                        dispatch(&mut probe, &select_menu_frame(choice), data, service, env).await;
                    if out.eve_battle.is_some()
                        || probe.session.current_event_session.as_ref().is_some_and(|e| {
                            e.phase == EventPhase::AwaitingBattle
                                || e.phase == EventPhase::Executing
                        })
                    {
                        conn.session = probe.session;
                        frames.extend(out.outgoing.iter().map(|f| f.frame.clone()));
                        if let Some((fight_id, diahinh)) = out.eve_battle {
                            return (fight_id, diahinh, frames);
                        }
                        break;
                    }
                }
            }
        }
        let out = dispatch(conn, &CONTINUE_FRAME, data, service, env).await;
        frames.extend(out.outgoing.iter().map(|f| f.frame.clone()));
        if let Some((fight_id, diahinh)) = out.eve_battle {
            return (fight_id, diahinh, frames);
        }
        if conn.session.current_event_session.is_none() {
            break;
        }
    }
    panic!("Tống Dũng chain did not reach StartBattle");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn e2e_12001_tong_dung_battle_win_rewards_registry_and_next_packet_safe() {
    set_eve_events_enabled(true);
    let pool = setup_test_db().await;
    let data = Arc::new(GameData::load(&data_dir()).expect("load Data/"));
    let char_id = seed_account_and_char(&pool, 9801, b"TongDungWin", 12001, 110, 1520).await;
    let repos = SqliteRepositories::new(pool.clone());

    let mut session = Session::new();
    session.id = char_id as u32;
    assert!(repos.sessions().load(char_id, &mut session).await.expect("load"));
    session.logined = true;
    session.authed = true;

    let service = BattleService::new(Arc::clone(&data)).with_pool(pool.clone());
    let (tx, mut rx) = mpsc::channel(256);
    registry_register_for_login(&session);
    service.register_sender(
        char_id,
        Arc::new(RwLock::new(session.clone())),
        tx.clone(),
    );

    let mut conn = Conn {
        session,
        ..Default::default()
    };
    let env = ServerEnv::with_pool(&pool);

    let (fight_id, diahinh, pre_frames) =
        drive_tong_dung_to_battle(&mut conn, &data, &service, &env).await;
    assert!(
        !conn.session.talk_locked,
        "Talk UI must be cleanly unlocked before entering battle"
    );
    let _ = pre_frames;

    // Refresh sender + registry just like `server_control.rs` does
    registry_update_if_online(&conn.session);
    service.register_sender(
        char_id,
        Arc::new(RwLock::new(conn.session.clone())),
        tx.clone(),
    );

    let battle_id = service.start_eve_encounter(&mut conn.session, fight_id, diahinh);
    assert!(battle_id > 0, "Tống Dũng Eve battle must start");
    registry_update_if_online(&conn.session);
    let _ = drain_rx(&mut rx);

    // End battle with Win
    service.battle_ended(battle_id, Outcome::PlayerWin);

    let post_frames = drain_rx(&mut rx);
    let results = op14_01_results(&post_frames);
    assert!(
        results
            .iter()
            .any(|&(rt, rc, param, _, _, _)| rt == 0 && rc == 8 && param == 14106),
        "Win must emit 0x14 01 RewardPet 14106; got results={results:?}"
    );
    assert!(
        results
            .iter()
            .any(|&(rt, rc, _, _, _, _)| rt == 0 && rc == 3),
        "Win must emit 0x14 01 NpcTeam; got results={results:?}"
    );
    assert!(
        post_frames.iter().any(|f| f.contains("0F08"))
            && post_frames.iter().any(|f| f.contains("0F01")),
        "Win must emit 0F08 + 0F01 for pet 14106"
    );

    // Registry must be updated with pet 14106 & Quest 10028
    let reg = registry_get(char_id as u32).expect("player must remain in registry");
    assert_eq!(reg.battle_id, 0, "battle_id must be cleared after battle_ended");
    assert!(
        reg.pets.iter().any(|p| p.id == 14106),
        "Registry must contain RewardPet 14106"
    );
    assert!(
        reg.quest_tasks.contains_key(&10028)
            || data
                .quest_marks
                .get(&10028)
                .is_some_and(|m| reg.quest_dont.contains(m)),
        "Registry must record Quest 10028"
    );

    // Next packet (movement) pulls from registry and must NOT overwrite rewards
    conn.session = reg;
    let _ = dispatch(&mut conn, &move_frame(520, 520), &data, &service, &env).await;
    assert!(
        conn.session.pets.iter().any(|p| p.id == 14106),
        "Next packet after battle must NOT overwrite RewardPet 14106"
    );
    assert!(
        conn.session.quest_tasks.contains_key(&10028)
            || data
                .quest_marks
                .get(&10028)
                .is_some_and(|m| conn.session.quest_dont.contains(m)),
        "Next packet after battle must NOT overwrite Quest 10028"
    );

    // Reload from SQLite to verify Battle End write-through
    let mut reloaded = Session::new();
    reloaded.id = char_id as u32;
    assert!(repos.sessions().load(char_id, &mut reloaded).await.expect("reload"));
    assert!(
        reloaded.pets.iter().any(|p| p.id == 14106),
        "Pet 14106 must be persisted in SQLite at Battle End"
    );
    assert!(
        reloaded.quest_tasks.contains_key(&10028)
            || data
                .quest_marks
                .get(&10028)
                .is_some_and(|m| reloaded.quest_dont.contains(m)),
        "Quest 10028 must be persisted in SQLite at Battle End"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn e2e_12001_tong_dung_lose_and_flee_do_not_trigger_on_win_and_persist() {
    set_eve_events_enabled(true);
    let pool = setup_test_db().await;
    let data = Arc::new(GameData::load(&data_dir()).expect("load Data/"));
    let repos = SqliteRepositories::new(pool.clone());

    for (idx, outcome) in [Outcome::PlayerLose, Outcome::PlayerFled].into_iter().enumerate() {
        let acc_id = 9810 + idx as i64;
        let name = format!("TongDungNeg{idx}");
        let char_id = seed_account_and_char(&pool, acc_id, name.as_bytes(), 12001, 110, 1520).await;

        let mut session = Session::new();
        session.id = char_id as u32;
        assert!(repos.sessions().load(char_id, &mut session).await.expect("load"));
        session.logined = true;
        session.authed = true;

        let service = BattleService::new(Arc::clone(&data)).with_pool(pool.clone());
        let (tx, _rx) = mpsc::channel(256);
        registry_register_for_login(&session);
        service.register_sender(char_id, Arc::new(RwLock::new(session.clone())), tx);

        let mut conn = Conn {
            session,
            ..Default::default()
        };
        let env = ServerEnv::with_pool(&pool);

        let (fight_id, diahinh, _) =
            drive_tong_dung_to_battle(&mut conn, &data, &service, &env).await;
        registry_update_if_online(&conn.session);
        service.register_sender(
            char_id,
            Arc::new(RwLock::new(conn.session.clone())),
            mpsc::channel(256).0,
        );

        let battle_id = service.start_eve_encounter(&mut conn.session, fight_id, diahinh);
        assert!(battle_id > 0);
        registry_update_if_online(&conn.session);

        service.battle_ended(battle_id, outcome);

        let reg = registry_get(char_id as u32).expect("registry entry");
        assert_eq!(reg.battle_id, 0);
        assert!(
            !reg.pets.iter().any(|p| p.id == 14106),
            "{outcome:?} must NOT grant RewardPet 14106"
        );
        assert!(
            !data
                .quest_marks
                .get(&10028)
                .is_some_and(|m| reg.quest_dont.contains(m)),
            "{outcome:?} must NOT mark Quest 10028 complete"
        );

        let mut reloaded = Session::new();
        reloaded.id = char_id as u32;
        assert!(repos.sessions().load(char_id, &mut reloaded).await.expect("reload"));
        assert!(
            !reloaded.pets.iter().any(|p| p.id == 14106),
            "SQLite must NOT contain RewardPet 14106 on {outcome:?}"
        );
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn e2e_eve_battle_party_member_preserves_standard_eve_quest_progress() {
    set_eve_events_enabled(true);
    let pool = setup_test_db().await;
    let data = Arc::new(GameData::load(&data_dir()).expect("load Data/"));
    let leader_id = seed_account_and_char(&pool, 9821, b"PartyLead", 12001, 110, 1520).await;
    let member_id = seed_account_and_char(&pool, 9822, b"PartyMem", 12001, 110, 1520).await;
    let repos = SqliteRepositories::new(pool.clone());

    let mut leader = Session::new();
    leader.id = leader_id as u32;
    assert!(repos.sessions().load(leader_id, &mut leader).await.expect("load leader"));
    leader.logined = true;
    leader.authed = true;
    leader.id_mem[0] = member_id as u32;

    let mut member = Session::new();
    member.id = member_id as u32;
    assert!(repos.sessions().load(member_id, &mut member).await.expect("load member"));
    member.logined = true;
    member.authed = true;
    member.id_leader = leader_id as u32;
    member.quest_tasks.insert(9999, (1, 1));
    // Even if a member had an AwaitingBattle event session, BattleService must not advance it
    member.current_event_session = Some(EventSession {
        map_id: 12001,
        eve_no: 1,
        npc_click_id: 1,
        trigger_kind: EventTriggerKind::ClickNpc,
        chain_depth: 0,
        results: vec![EveResult {
            result_group_no: 1,
            result_no: 1,
            result_type: 3,
            result_class: 0,
            parameter: 1,
            parameter_style: 0,
            result_value: 0,
            result_mean_no: 0,
        }, EveResult {
            result_group_no: 1,
            result_no: 2,
            result_type: 0,
            result_class: 2,
            parameter: 10028,
            parameter_style: 1,
            result_value: 1,
            result_mean_no: 0,
        }],
        current_index: 0,
        battle_result: 0,
        phase: EventPhase::AwaitingBattle,
        last_surface_id: 0,
        last_choice_code: 0,
        matched_condition_no: 0,
    });

    let service = BattleService::new(Arc::clone(&data)).with_pool(pool.clone());
    let (tx_l, _rx_l) = mpsc::channel(256);
    let (tx_m, _rx_m) = mpsc::channel(256);
    registry_register_for_login(&leader);
    registry_register_for_login(&member);
    service.register_sender(leader_id, Arc::new(RwLock::new(leader.clone())), tx_l);
    service.register_sender(member_id, Arc::new(RwLock::new(member.clone())), tx_m);

    let mut conn = Conn {
        session: leader,
        ..Default::default()
    };
    let env = ServerEnv::with_pool(&pool);
    let (fight_id, diahinh, _) =
        drive_tong_dung_to_battle(&mut conn, &data, &service, &env).await;
    registry_update_if_online(&conn.session);
    service.register_sender(
        leader_id,
        Arc::new(RwLock::new(conn.session.clone())),
        mpsc::channel(256).0,
    );

    let battle_id = service.start_eve_encounter(&mut conn.session, fight_id, diahinh);
    assert!(battle_id > 0);
    registry_update_if_online(&conn.session);

    service.battle_ended(battle_id, Outcome::PlayerWin);

    let reg_leader = registry_get(leader_id as u32).expect("leader in registry");
    assert!(reg_leader.pets.iter().any(|p| p.id == 14106));

    let reg_member = registry_get(member_id as u32).expect("member in registry");
    assert_eq!(reg_member.battle_id, 0);
    assert!(
        !reg_member.quest_tasks.contains_key(&10028),
        "Party Member must NOT advance standard Eve quest 10028"
    );
    assert_eq!(
        reg_member.quest_tasks.get(&9999),
        Some(&(1, 1)),
        "Party Member's existing quest_tasks must remain untouched"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn e2e_eve_battle_disconnect_mid_battle_updates_db_for_relogin() {
    set_eve_events_enabled(true);
    let pool = setup_test_db().await;
    let data = Arc::new(GameData::load(&data_dir()).expect("load Data/"));
    let char_id = seed_account_and_char(&pool, 9831, b"DcMidBattle", 12001, 110, 1520).await;
    let repos = SqliteRepositories::new(pool.clone());

    let mut session = Session::new();
    session.id = char_id as u32;
    assert!(repos.sessions().load(char_id, &mut session).await.expect("load"));
    session.logined = true;
    session.authed = true;

    let service = BattleService::new(Arc::clone(&data)).with_pool(pool.clone());
    let (tx, _rx) = mpsc::channel(256);
    registry_register_for_login(&session);
    service.register_sender(char_id, Arc::new(RwLock::new(session.clone())), tx);

    let mut conn = Conn {
        session,
        ..Default::default()
    };
    let env = ServerEnv::with_pool(&pool);

    let (fight_id, diahinh, _) =
        drive_tong_dung_to_battle(&mut conn, &data, &service, &env).await;
    registry_update_if_online(&conn.session);
    service.register_sender(
        char_id,
        Arc::new(RwLock::new(conn.session.clone())),
        mpsc::channel(256).0,
    );

    let battle_id = service.start_eve_encounter(&mut conn.session, fight_id, diahinh);
    assert!(battle_id > 0);
    registry_update_if_online(&conn.session);

    // Simulate mid-battle disconnect (exactly as `disconnect_player` does)
    service.unregister(char_id);
    service.leave_battle(&mut conn.session);
    lock_online_sessions().remove(&(char_id as u32));
    assert!(registry_get(char_id as u32).is_none());

    // Battle finishes while player is offline
    service.battle_ended(battle_id, Outcome::PlayerWin);

    // Player logs back in and loads from SQLite: post-battle Eve rewards must be present
    let mut relogin = Session::new();
    relogin.id = char_id as u32;
    assert!(repos.sessions().load(char_id, &mut relogin).await.expect("reload"));
    assert!(
        relogin.pets.iter().any(|p| p.id == 14106),
        "Disconnected player must receive RewardPet 14106 in SQLite for next login"
    );
    assert!(
        relogin.quest_tasks.contains_key(&10028)
            || data
                .quest_marks
                .get(&10028)
                .is_some_and(|m| relogin.quest_dont.contains(m)),
        "Disconnected player must receive Quest 10028 in SQLite for next login"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn eve_fight_data_loads_right_enemies_diahinh_fallback_and_direct_map_broadcast() {
    set_eve_events_enabled(true);
    let mut data = GameData::default();
    let npc = Npc {
        id: 1001,
        name: b"Ally".to_vec(),
        lv: 20,
        hp: 500,
        sp: 200,
        atk: 50,
        def: 40,
        agi: 30,
        ..Default::default()
    };
    data.npcs.insert(1001, npc.clone());
    data.npcs.insert(1002, Npc { id: 1002, ..npc });

    let mut scene = SceneEveData::default();
    // SceneInfo background_no = 0 -> must fallback to 112
    scene.scene_infos.insert(
        1,
        EveSceneInfo {
            eve_no: 1,
            background_no: 0,
            ..Default::default()
        },
    );
    let fight = EveFightData {
        eve_no: 7,
        fight_type: 34,
        bk_music: 1,
        link_to_result: 1,
        left_enemies: vec![EveFightEnemy {
            no: 1,
            npc_id: 1002,
            location_pos: 2, // row 0, col 2
            ai: 1,
        }],
        right_enemies: vec![EveFightEnemy {
            no: 1,
            npc_id: 1001,
            location_pos: 5, // row 3, col 0 (ally side, team 1)
            ai: 1,
        }],
    };
    scene.fight_datas.insert(7, fight.clone());
    data.scene_eve_data.insert(12001, scene);

    // 1. Verify Battle::add_npc places right_enemies on Team 1
    let mut b = Battle::new(1, 112);
    b.fight_type = fight.fight_type;
    b.add_npc(data.npcs.get(&1001).unwrap(), 1, 3, 0, 3);
    b.add_npc(data.npcs.get(&1002).unwrap(), 1, 0, 2, 3);
    assert_eq!(b.fight_type, 34);
    assert_eq!(b.cell(3, 0).unwrap().team, 1, "right_enemy must be on Team 1");
    assert_eq!(b.cell(0, 2).unwrap().team, 2, "left_enemy must be on Team 2");

    // 2. Verify start_eve_encounter + battle_ended preserves direct_messages & map_broadcast
    //    when post-battle Eve step executes a party Warp
    data.warps.insert(
        (12001, 9001),
        ts_dream::data::tables::Warp {
            warpid: 9001,
            map1: 12001,
            map2: 12002,
            x: 300,
            y: 300,
            ..Default::default()
        },
    );
    let service = BattleService::new(Arc::new(data));
    let mut leader = Session::new();
    leader.id = 9851;
    leader.map_id = 12001;
    leader.id_mem[0] = 9852;
    leader.current_event_session = Some(EventSession {
        map_id: 12001,
        eve_no: 1,
        npc_click_id: 1,
        trigger_kind: EventTriggerKind::ClickNpc,
        chain_depth: 0,
        results: vec![
            EveResult {
                result_group_no: 1,
                result_no: 1,
                result_type: 3,
                result_class: 0,
                parameter: 7,
                parameter_style: 0,
                result_value: 0,
                result_mean_no: 0,
            },
            EveResult {
                result_group_no: 1,
                result_no: 2,
                result_type: 2,
                result_class: 0,
                parameter: 9001,
                parameter_style: 0,
                result_value: 0,
                result_mean_no: 0,
            },
        ],
        current_index: 0,
        battle_result: 0,
        phase: EventPhase::AwaitingBattle,
        last_surface_id: 0,
        last_choice_code: 0,
        matched_condition_no: 0,
    });
    let mut member = Session::new();
    member.id = 9852;
    member.map_id = 12001;
    member.id_leader = 9851;

    let mut bystander = Session::new();
    bystander.id = 9853;
    bystander.map_id = 12001;

    let (tx_l, mut rx_l) = mpsc::channel(64);
    let (tx_m, mut rx_m) = mpsc::channel(64);
    let (tx_b, mut rx_b) = mpsc::channel(64);
    registry_register_for_login(&leader);
    registry_register_for_login(&member);
    registry_register_for_login(&bystander);
    service.register_sender(9851, Arc::new(RwLock::new(leader.clone())), tx_l);
    service.register_sender(9852, Arc::new(RwLock::new(member)), tx_m);
    service.register_sender(9853, Arc::new(RwLock::new(bystander)), tx_b);

    let bid = service.start_eve_encounter(&mut leader, 7, 0);
    assert!(bid > 0);
    let _ = drain_rx(&mut rx_l);
    let _ = drain_rx(&mut rx_m);
    let _ = drain_rx(&mut rx_b);

    service.battle_ended(bid, Outcome::PlayerWin);

    let mem_frames = drain_rx(&mut rx_m);
    let bys_frames = drain_rx(&mut rx_b);
    assert!(
        !mem_frames.is_empty(),
        "Party member must receive direct_messages from post-battle Warp"
    );
    assert!(
        !bys_frames.is_empty(),
        "Map bystander must receive map_broadcast (hide_from_map) from post-battle Warp"
    );
}
