//! Integration tests for Ticket 05:
//! - Pure Eve Planner (`DomainEffect`, `rt >= 7` never emitted via `0x14`)
//! - `Pre-Flight Probe All-or-Nothing` (`homdo` 25-slot full & `Quest Ledger` 200-slot full + `0x18 03`)
//! - `EventRunner` unified loop (`14 2C Lock` once across `AutoChain`, no lock on step-0 `StartBattle`,
//!   clean unlock+EndTalk before mid-talk `StartBattle`, `detect_duplicate_items` `p_style=0` negative)
//! - `result_class=2` `p_style` support without Bear save-gate & `quest_dont` only on completion/explicit mark
//! - E2E via `dispatcher::dispatch` + `setup_test_db()`:
//!   - `12001` Sĩ Triết (repeatable quest `10544` + `FollowPet 14131`, adjacent `homdo` slots preserved)
//!   - `12136` Giản Ung (`0x14 09` menu chain -> free skill `14001` + `08 01 6E` + `Quest 10049` +
//!     `quest_dont` bit `27` + `FinishChain` write-through persisted across DB reload)

mod common;

use common::test_db::setup_test_db;
use std::path::PathBuf;
use ts_dream::battle::service::BattleService;
use ts_dream::data::loader::GameData;
use ts_dream::data::loaders::{
    EveCondition, EveFightData, EveNpcPlacement, EveResult, NpcEventData, SceneEveData,
};
use ts_dream::db::modern::model::PetStorageType;
use ts_dream::db::modern::sqlite::SqliteRepositories;
use ts_dream::db::modern::traits::CharacterSeed;
use ts_dream::db::pool::DbPool;
use ts_dream::eve::auto_chain::{
    EveAutoChainEngine, EventPhase, EventSession, EventTriggerKind,
};
use ts_dream::eve::planner::{plan_results, DomainEffect};
use ts_dream::protocol::codecs::npc_talk::{NpcTalkCodec, TalkLockMode};
use ts_dream::protocol::codecs::quest_sync::QuestSyncCodec;
use ts_dream::server::dispatcher::{dispatch, HandleOutcome, ServerEnv};
use ts_dream::server::handlers::npc_event::set_eve_events_enabled;
use ts_dream::server::handlers::talk::execute_event_step;
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

fn op14_sub_frames(out: &HandleOutcome, sub: u8) -> Vec<String> {
    out.outgoing
        .iter()
        .filter_map(|f| {
            let bytes = hex::decode(&f.frame).ok()?;
            if bytes.len() >= 6 && bytes[4] == 0x14 && bytes[5] == sub {
                Some(f.frame.clone())
            } else {
                None
            }
        })
        .collect()
}

async fn seed_account_and_char(pool: &DbPool, account_id: i64, name: &[u8], map_id: u16, x: u16, y: u16) {
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
        element: 2,
        map_id: i64::from(map_id),
        map_x: i64::from(x),
        map_y: i64::from(y),
    };
    repos
        .characters()
        .create(account_id, name, &seed)
        .await
        .expect("create character");
}

#[test]
fn pure_planner_never_emits_0x14_for_rt_ge_7_and_handles_autochain_negative_pstyle0() {
    // 1. rt in {7, 8, 10} must map to DomainEffect::Unsupported and never emit 0x14 01..06
    let raw_results = vec![
        EveResult {
            result_type: 7,
            result_class: 7,
            parameter: 1,
            result_value: 100,
            ..Default::default()
        },
        EveResult {
            result_type: 8,
            result_class: 0,
            ..Default::default()
        },
        EveResult {
            result_type: 10,
            result_class: 1,
            ..Default::default()
        },
        EveResult {
            result_type: 0,
            result_class: 4,
            parameter: 14001,
            result_value: 1,
            ..Default::default()
        },
    ];
    let effects = plan_results(&raw_results);
    assert_eq!(
        effects[0],
        DomainEffect::Unsupported {
            result_type: 7,
            result_class: 7
        }
    );
    assert_eq!(
        effects[1],
        DomainEffect::Unsupported {
            result_type: 8,
            result_class: 0
        }
    );
    assert_eq!(
        effects[2],
        DomainEffect::Unsupported {
            result_type: 10,
            result_class: 1
        }
    );
    assert_eq!(
        effects[3],
        DomainEffect::LearnSkill {
            skill_id: 14001,
            target_level: 1
        }
    );

    // Executing them through EventRunner must NOT emit any 0x14 01..06 frame
    let data = GameData::default();
    let mut session = Session::new();
    session.id = 9001;
    session.current_event_session = Some(EventSession {
        map_id: 12001,
        eve_no: 1,
        npc_click_id: 1,
        trigger_kind: EventTriggerKind::ClickNpc,
        chain_depth: 0,
        results: raw_results[..3].to_vec(),
        current_index: 0,
        phase: EventPhase::Executing,
        last_surface_id: -1,
        last_choice_code: -1,
        battle_result: 0,
        matched_condition_no: 0,
    });
    let mut out = HandleOutcome::default();
    execute_event_step(&mut session, &data, &mut out);
    assert!(
        op14_sub_frames(&out, 0x01).is_empty(),
        "rt >= 7 must NEVER be emitted via 0x14 01"
    );

    // 2. detect_duplicate_items: p_style=0 negative result_value (take) is NOT duplicate give,
    // while p_style=0 positive result_value (give) IS detected as duplicate give.
    let take_session = EventSession {
        map_id: 1,
        eve_no: 1,
        npc_click_id: 1,
        trigger_kind: EventTriggerKind::ClickNpc,
        chain_depth: 0,
        results: vec![EveResult {
            result_type: 0,
            result_class: 1,
            parameter: 46001,
            parameter_style: 0,
            result_value: -3,
            ..Default::default()
        }],
        current_index: 0,
        phase: EventPhase::Completed,
        last_surface_id: -1,
        last_choice_code: -1,
        battle_result: 0,
        matched_condition_no: 1,
    };
    let give_pstyle0 = EventSession {
        results: vec![EveResult {
            result_type: 0,
            result_class: 1,
            parameter: 46001,
            parameter_style: 0,
            result_value: 2,
            ..Default::default()
        }],
        ..take_session.clone()
    };
    assert!(
        !EveAutoChainEngine::detect_duplicate_items(&give_pstyle0, &take_session),
        "p_style=0 negative result_value (item take) must NOT block subsequent item give"
    );
    assert!(
        EveAutoChainEngine::detect_duplicate_items(&give_pstyle0, &give_pstyle0),
        "p_style=0 positive result_value (item give) must trigger duplicate item guard"
    );
}

#[tokio::test]
async fn preflight_all_or_nothing_rejects_full_homdo_and_full_quest_ledger() {
    let mut data = GameData::default();
    let mut scene = SceneEveData::default();
    scene.npcs.insert(
        1,
        EveNpcPlacement {
            id: 1,
            npc_id: 1,
            x: 100,
            y: 100,
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
                and_num: 1,
                results: vec![
                    EveResult {
                        result_type: 0,
                        result_class: 5,
                        parameter_style: 1,
                        result_value: 2000, // +2000 gold
                        ..Default::default()
                    },
                    EveResult {
                        result_type: 0,
                        result_class: 2,
                        parameter: 10544, // +quest 10544
                        parameter_style: 1,
                        result_value: 1,
                        ..Default::default()
                    },
                    EveResult {
                        result_type: 0,
                        result_class: 1,
                        parameter: 46001, // +3 item 46001 (odd count)
                        parameter_style: 1,
                        result_value: 3,
                        ..Default::default()
                    },
                ],
                ..Default::default()
            }],
            ..Default::default()
        },
    );
    data.scene_eve_data.insert(7801, scene);

    let service = BattleService::default();
    let env = ServerEnv::none();
    set_eve_events_enabled(true);

    // 1. Negative: homdo 25/25 slots full -> reject entire batch (gold, quest, item all unchanged)
    let mut conn_full_bag = Conn::default();
    conn_full_bag.session.id = 77001;
    conn_full_bag.session.map_id = 7801;
    conn_full_bag.session.map_x = 100;
    conn_full_bag.session.map_y = 100;
    conn_full_bag.session.gold = 500;
    for slot in 1..=25u8 {
        conn_full_bag.session.homdo.push(InventoryItem {
            slot,
            id: 40000 + u16::from(slot),
            count: 3,
            ..Default::default()
        });
    }
    let out_bag = dispatch(&mut conn_full_bag, &click_npc_frame(1), &data, &service, &env).await;
    assert_eq!(conn_full_bag.session.gold, 500, "gold must NOT mutate on full homdo");
    assert!(
        conn_full_bag.session.quest_tasks.is_empty(),
        "quest_tasks must NOT mutate on full homdo"
    );
    assert_eq!(conn_full_bag.session.homdo.len(), 25);
    assert_eq!(conn_full_bag.session.homdo[0].count, 3, "adjacent slot intact");
    assert!(
        out_bag.outgoing.iter().any(|f| f.frame.contains("020102")),
        "full homdo must emit bag-full system toast"
    );

    // 2. Negative: Quest Ledger 200/200 slots full -> reject entire batch + emit S->C 0x18 03
    let mut conn_full_ql = Conn::default();
    conn_full_ql.session.id = 77002;
    conn_full_ql.session.map_id = 7801;
    conn_full_ql.session.map_x = 100;
    conn_full_ql.session.map_y = 100;
    conn_full_ql.session.gold = 500;
    for slot in 1..=200u8 {
        conn_full_ql
            .session
            .quest_tasks
            .insert(20000 + u16::from(slot), (slot, 1));
    }
    let out_ql = dispatch(&mut conn_full_ql, &click_npc_frame(1), &data, &service, &env).await;
    assert_eq!(conn_full_ql.session.gold, 500, "gold must NOT mutate on full Quest Ledger");
    assert!(conn_full_ql.session.homdo.is_empty(), "homdo must NOT mutate on full Quest Ledger");
    assert!(
        !conn_full_ql.session.quest_tasks.contains_key(&10544),
        "new quest must NOT be inserted when Quest Ledger is full"
    );
    assert!(
        out_ql
            .outgoing
            .iter()
            .any(|f| f.frame == QuestSyncCodec::build_quest_full_hex()),
        "full Quest Ledger must emit S->C 0x18 03"
    );
}

#[tokio::test]
async fn event_runner_step0_start_battle_emits_no_lock() {
    let mut data = GameData::default();
    let mut scene = SceneEveData::default();
    scene.npcs.insert(
        1,
        EveNpcPlacement {
            id: 1,
            npc_id: 1,
            x: 100,
            y: 100,
            events: vec![1],
            ..Default::default()
        },
    );
    scene.fight_datas.insert(
        5,
        EveFightData {
            eve_no: 5,
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
                and_num: 1,
                results: vec![EveResult {
                    result_type: 3,
                    result_mean_no: 5,
                    ..Default::default()
                }],
                ..Default::default()
            }],
            ..Default::default()
        },
    );
    data.scene_eve_data.insert(7802, scene);

    let service = BattleService::default();
    let env = ServerEnv::none();
    set_eve_events_enabled(true);

    let mut conn = Conn::default();
    conn.session.id = 77003;
    conn.session.map_id = 7802;
    conn.session.map_x = 100;
    conn.session.map_y = 100;
    let out = dispatch(&mut conn, &click_npc_frame(1), &data, &service, &env).await;
    assert_eq!(out.eve_battle, Some((5, 112)));
    assert!(
        op14_sub_frames(&out, 0x2C).is_empty(),
        "step-0 StartBattle must NOT emit 14 2C Lock"
    );
    assert!(
        !conn.session.talk_locked,
        "step-0 StartBattle must leave talk_locked false"
    );
}

#[tokio::test]
async fn e2e_12001_si_triet_repeatable_and_12136_gian_ung_skill_write_through_db() {
    let pool = setup_test_db().await;
    let data = GameData::load(&data_dir()).expect("GameData::load");
    let service = BattleService::default();
    let env = ServerEnv::with_pool(&pool);
    set_eve_events_enabled(true);
    let repos = SqliteRepositories::new(pool.clone());

    // =========================================================================
    // Part A: 12001 Sĩ Triết (NPC 24) — repeatable quest 10544 + FollowPet 14131
    // =========================================================================
    let npc24 = data
        .scene_eve_data
        .get(&12001)
        .and_then(|s| s.npcs.get(&24))
        .expect("12001 NPC 24");
    let char_si_triet = 88001i64;
    seed_account_and_char(&pool, char_si_triet, b"SiTriet", 12001, npc24.x as u16, npc24.y as u16).await;

    let mut conn = Conn::default();
    conn.session.id = char_si_triet as u32;
    assert!(repos.sessions().load(char_si_triet, &mut conn.session).await.unwrap());
    // Seed adjacent homdo slots with odd quantities to verify preservation
    conn.session.homdo.push(InventoryItem {
        slot: 1,
        id: 46001,
        count: 7,
        ..Default::default()
    });
    conn.session.homdo.push(InventoryItem {
        slot: 2,
        id: 46002,
        count: 13,
        ..Default::default()
    });

    // Seed active quest 10544 step 1 + FollowPet 14131 so eve=36 (step != 1) is skipped
    // and eve=40 cond=4 (Sĩ Triết turn-in) matches.
    conn.session.quest_tasks.insert(10544, (1, 1));
    conn.session.pets.push(PetState {
        stt: 1,
        storage_type: PetStorageType::Carried,
        id: 14131,
        ..Default::default()
    });
    let point_before = conn.session.point;

    // Click NPC 24 -> Talk 16536 (1 Lock emitted) -> Continue -> Quest 10544 removed (0x18 04) + Gold/Points
    let out1 = dispatch(&mut conn, &click_npc_frame(24), &data, &service, &env).await;
    assert_eq!(op14_sub_frames(&out1, 0x2C).len(), 1, "single Lock emitted on talk start");
    let out_turnin = dispatch(&mut conn, &CONTINUE_FRAME, &data, &service, &env).await;
    assert!(!conn.session.quest_tasks.contains_key(&10544), "10544 removed on turn-in");
    assert!(conn.session.quest_dont.is_empty(), "repeatable quest 10544 must not set quest_dont");
    assert!(
        conn.session.point > point_before,
        "Sĩ Triết turn-in must award points (rc=5 p_style=1 val=40)"
    );
    assert!(
        out_turnin
            .outgoing
            .iter()
            .any(|f| f.frame == QuestSyncCodec::build_item_clear_hex(10544)),
        "0x18 04 clear frame emitted"
    );
    assert!(
        out_turnin
            .outgoing
            .iter()
            .any(|f| f.frame == NpcTalkCodec::build_end_talk_hex()),
        "FinishChain emits EndTalk"
    );
    assert_eq!(conn.session.homdo[0].count, 7, "adjacent slot 1 preserved");
    assert_eq!(conn.session.homdo[1].count, 13, "adjacent slot 2 preserved");

    // =========================================================================
    // Part B: 12136 Giản Ung (NPC 1) — menu chain -> Skill 14001 + Quest 10049 + DB write-through
    // =========================================================================
    let npc1 = data
        .scene_eve_data
        .get(&12136)
        .and_then(|s| s.npcs.get(&1))
        .expect("12136 NPC 1");
    let char_gian_ung = 88002i64;
    seed_account_and_char(&pool, char_gian_ung, b"GianUng", 12136, npc1.x as u16, npc1.y as u16).await;

    let mut conn2 = Conn::default();
    conn2.session.id = char_gian_ung as u32;
    assert!(repos.sessions().load(char_gian_ung, &mut conn2.session).await.unwrap());
    conn2.session.skill_point = 0; // verify Skill 14001 is granted FREE of skill_point

    let out_start = dispatch(&mut conn2, &click_npc_frame(1), &data, &service, &env).await;
    let mut lock_count = op14_sub_frames(&out_start, 0x2C).len();
    for _ in 0..10 {
        if conn2
            .session
            .current_event_session
            .as_ref()
            .is_some_and(|e| e.phase == EventPhase::AwaitingChoice)
        {
            break;
        }
        let o = dispatch(&mut conn2, &CONTINUE_FRAME, &data, &service, &env).await;
        lock_count += op14_sub_frames(&o, 0x2C).len();
    }
    assert_eq!(lock_count, 1, "only 1 Lock emitted before Surface 1");

    let _ = dispatch(&mut conn2, &select_menu_frame(30), &data, &service, &env).await;
    let _ = dispatch(&mut conn2, &select_menu_frame(32), &data, &service, &env).await;
    let _ = dispatch(&mut conn2, &CONTINUE_FRAME, &data, &service, &env).await;
    let _ = dispatch(&mut conn2, &select_menu_frame(32), &data, &service, &env).await;
    let _ = dispatch(&mut conn2, &CONTINUE_FRAME, &data, &service, &env).await;

    let mut finish_frames = Vec::new();
    let o_sel = dispatch(&mut conn2, &select_menu_frame(31), &data, &service, &env).await;
    finish_frames.extend(o_sel.outgoing);
    for _ in 0..10 {
        if conn2.session.current_event_session.is_none() {
            break;
        }
        let o = dispatch(&mut conn2, &CONTINUE_FRAME, &data, &service, &env).await;
        finish_frames.extend(o.outgoing);
    }
    assert!(conn2.session.current_event_session.is_none(), "FinishChain completed");
    assert_eq!(conn2.session.skill_point, 0, "Skill 14001 must be free of skill_point");
    assert!(
        conn2.session.skills.iter().any(|(id, lv)| *id == 14001 && *lv >= 1),
        "Session must contain Skill 14001"
    );
    assert!(
        finish_frames.iter().any(|f| f.frame.contains("08016E01")),
        "08 01 6E skill sync frame must be emitted"
    );
    assert!(
        finish_frames.iter().any(|f| f.frame == NpcTalkCodec::build_talk_lock_hex(conn2.session.id, TalkLockMode::Unlock)),
        "FinishChain must emit Unlock once"
    );
    assert!(
        finish_frames.iter().any(|f| f.frame == NpcTalkCodec::build_end_talk_hex()),
        "FinishChain must emit EndTalk once"
    );

    // Reload from SQLite without calling auto_save — FinishChain write-through must have persisted!
    let mut reloaded = Session::new();
    reloaded.id = char_gian_ung as u32;
    assert!(repos.sessions().load(char_gian_ung, &mut reloaded).await.unwrap());
    assert!(
        reloaded.skills.iter().any(|(id, lv)| *id == 14001 && *lv >= 1),
        "Skill 14001 must persist via FinishChain write-through"
    );
    assert!(
        reloaded.quest_tasks.contains_key(&10049),
        "Quest 10049 must persist via FinishChain write-through"
    );
    assert!(
        reloaded.quest_dont.contains(&27),
        "Mark.dat bit_id 27 for Quest 10049 must persist via FinishChain write-through"
    );
    assert_eq!(
        reloaded.completed_eve_counts.get(&(12136, 2)),
        Some(&1),
        "Scene-scoped completed_eve_counts (12136, 2) must persist via FinishChain write-through"
    );
}

#[tokio::test]
async fn preflight_allows_negative_pstyle1_on_full_ledger_and_finish_chain_syncs_registry() {
    let mut data = GameData::default();
    let mut scene = SceneEveData::default();
    scene.npcs.insert(
        1,
        EveNpcPlacement {
            id: 1,
            npc_id: 1,
            x: 100,
            y: 100,
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
                and_num: 1,
                results: vec![
                    EveResult {
                        result_type: 0,
                        result_class: 2,
                        parameter: 10999, // not in quest_tasks
                        parameter_style: 1,
                        result_value: -1, // decrement no-op when absent
                        ..Default::default()
                    },
                    EveResult {
                        result_type: 0,
                        result_class: 5,
                        parameter_style: 1,
                        result_value: 2000,
                        ..Default::default()
                    },
                ],
                ..Default::default()
            }],
            ..Default::default()
        },
    );
    data.scene_eve_data.insert(7803, scene);

    let service = BattleService::default();
    let env = ServerEnv::none();
    set_eve_events_enabled(true);

    let mut conn = Conn::default();
    conn.session.id = 77099;
    conn.session.map_id = 7803;
    conn.session.map_x = 100;
    conn.session.map_y = 100;
    conn.session.gold = 100;
    for slot in 1..=200u8 {
        conn.session
            .quest_tasks
            .insert(20000 + u16::from(slot), (slot, 1));
    }
    ts_dream::server::session::registry_register_for_login(&conn.session);

    let out = dispatch(&mut conn, &click_npc_frame(1), &data, &service, &env).await;
    assert!(
        !out.outgoing
            .iter()
            .any(|f| f.frame == QuestSyncCodec::build_quest_full_hex()),
        "negative p_style=1 on absent quest must NOT trigger QuestLedgerFull"
    );
    assert_eq!(conn.session.gold, 2100, "gold reward must execute");

    let reg = ts_dream::server::session::lock_online_sessions()
        .get(&77099)
        .cloned()
        .expect("online registry entry");
    assert_eq!(reg.gold, 2100, "FinishChain must sync mutated gold to online registry");
    assert!(!reg.talk_locked, "FinishChain must sync unlocked talk state to registry");
    ts_dream::server::session::lock_online_sessions().remove(&77099);
}
