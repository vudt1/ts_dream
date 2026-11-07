//! Integration tests for Ticket 01: Character God & Ghost State, Map Appearance, and Real-Time Synchronization.
//!
//! Verifies:
//! - SQLite schema persistence and session hydration of `ghost` and `god`
//! - `player_appear` (`0x03`) byte layout at offsets 5 & 6
//! - Real-time synchronizer frame `S→C 18 08`
//! - Consumable Item 46180 ("Cải vận phù") exorcism and rejection when not ghosted
//! - NPC 1752 ("Bà Gọi Hồn") exorcism with gold fee gating
//! - Combat experience formula modifiers (1.5x God, 0.5x Ghost)

use ts_dream::battle::combat_formula::apply_god_ghost_exp;
use ts_dream::battle::service::BattleService;
use ts_dream::data::loader::GameData;
use ts_dream::db::modern::traits::CharacterSeed;
use ts_dream::db::modern::sqlite::SqliteRepositories;
use ts_dream::server::dispatcher::{HandleOutcome, OpcodeCtx, ServerEnv};
use ts_dream::server::handlers::inventory::handle_inventory;
use ts_dream::server::handlers::talk::handle_talk;
use ts_dream::server::session::{Conn, InventoryItem, Session};
use ts_dream::server::spawn;

mod common;
use common::test_db::setup_test_db;

#[test]
fn test_player_appear_god_ghost_offsets() {
    let mut session = Session::default();
    session.id = 1001;
    session.name = b"TestPlayer".to_vec();
    session.map_id = 12001;
    session.map_x = 400;
    session.map_y = 400;
    session.ghost = 1;
    session.god = 15;

    let frames = spawn::build_logined_sequence_session_ext(&session);
    let appear_hex = frames
        .iter()
        .find(|f| f.starts_with("F444") && f.len() > 12 && &f[8..10] == "03")
        .expect("must contain player_appear packet (0x03)");

    let raw = hex::decode(appear_hex).expect("valid hex");
    // Wire frame layout:
    // [0..2] F4 44
    // [2..4] len LE
    // [4] opcode = 0x03
    // [5..9] id LE32 (4 bytes)
    // [9] sex (1 byte)
    // [10] ghost (1 byte, offset 5 in body)
    // [11] god (1 byte, offset 6 in body)
    let payload = &raw[4..];
    assert_eq!(payload[0], 0x03, "opcode must be 0x03");
    assert_eq!(payload[6], 1, "ghost flag must be 1 (offset 5 in body after 4-byte ID)");
    assert_eq!(payload[7], 1, "god flag must be 1 (offset 6 in body after 4-byte ID)");

    // Test with zeroes
    session.ghost = 0;
    session.god = 0;
    let frames_clean = spawn::build_logined_sequence_session_ext(&session);
    let appear_clean_hex = frames_clean
        .iter()
        .find(|f| f.starts_with("F444") && f.len() > 12 && &f[8..10] == "03")
        .expect("must contain player_appear packet");
    let raw_clean = hex::decode(appear_clean_hex).expect("valid hex");
    let payload_clean = &raw_clean[4..];
    assert_eq!(payload_clean[6], 0, "ghost flag must be 0");
    assert_eq!(payload_clean[7], 0, "god flag must be 0");
}

#[test]
fn test_state_sync_frame_structure() {
    let char_id = 0x12345678;

    // Ghost off: type 1, value 0
    let ghost_off = spawn::state_sync_frame(char_id, 1, 0);
    assert!(ghost_off.starts_with("F44409001808"), "must be header F4 44 09 00 and opcode 18 08");
    let raw = hex::decode(&ghost_off).expect("valid hex");
    let payload = &raw[4..];
    assert_eq!(payload[0], 0x18);
    assert_eq!(payload[1], 0x08);
    let id_le = u32::from_le_bytes([payload[2], payload[3], payload[4], payload[5]]);
    assert_eq!(id_le, 0x12345678);
    let type_le = u16::from_le_bytes([payload[6], payload[7]]);
    assert_eq!(type_le, 1);
    assert_eq!(payload[8], 0);

    // God on: type 2, value 1
    let god_on = spawn::state_sync_frame(char_id, 2, 1);
    assert!(god_on.starts_with("F44409001808"));
    let raw_god = hex::decode(&god_on).expect("valid hex");
    let payload_god = &raw_god[4..];
    let type_god = u16::from_le_bytes([payload_god[6], payload_god[7]]);
    assert_eq!(type_god, 2);
    assert_eq!(payload_god[8], 1);
}

#[test]
fn test_combat_formula_god_ghost_exp() {
    // Normal base EXP
    assert_eq!(apply_god_ghost_exp(1000, 0, 0), 1000);

    // God of Fortune buff (1.5x)
    assert_eq!(apply_god_ghost_exp(1000, 10, 0), 1500);
    assert_eq!(apply_god_ghost_exp(333, 1, 0), 500);

    // Ghost debuff (0.5x)
    assert_eq!(apply_god_ghost_exp(1000, 0, 1), 500);
    assert_eq!(apply_god_ghost_exp(100, 0, 1), 50);

    // Both God and Ghost active (edge case: 1.5x then 0.5x = 0.75x)
    assert_eq!(apply_god_ghost_exp(1000, 5, 1), 750);
}

#[tokio::test]
async fn test_consumable_46180_cai_van_phu() {
    let data = GameData::default();
    let service = BattleService::default();

    // 1. Success case: ghost == 1
    {
        let mut conn = Conn::default();
        conn.session.id = 2001;
        conn.session.ghost = 1;
        conn.session.homdo.push(InventoryItem {
            slot: 1,
            id: 46180,
            count: 1,
            ..Default::default()
        });

        let mut out = HandleOutcome::default();
        let payload = [1, 0];
        let mut ctx = OpcodeCtx {
            conn: &mut conn,
            data: &data,
            service: &service,
            out: &mut out,
            opcode: 0x17,
            sub: 15,
            payload: &payload,
            decoded: &[],
            env: ServerEnv::none(),
        };

        handle_inventory(&mut ctx).await;
        assert_eq!(conn.session.ghost, 0, "ghost status must be cleared");
        assert!(
            conn.session.homdo.iter().all(|i| i.id != 46180 || i.count == 0),
            "item must be consumed"
        );

        let has_sync = out.outgoing.iter().any(|f| f.contains("1808") && f.contains("010000"));
        assert!(has_sync, "must emit 18 08 sync frame indicating ghost cleared");
    }

    // 2. Negative case: ghost == 0 -> rejected, item preserved
    {
        let mut conn = Conn::default();
        conn.session.id = 2002;
        conn.session.ghost = 0;
        conn.session.homdo.push(InventoryItem {
            slot: 1,
            id: 46180,
            count: 1,
            ..Default::default()
        });

        let mut out = HandleOutcome::default();
        let payload = [1, 0];
        let mut ctx = OpcodeCtx {
            conn: &mut conn,
            data: &data,
            service: &service,
            out: &mut out,
            opcode: 0x17,
            sub: 15,
            payload: &payload,
            decoded: &[],
            env: ServerEnv::none(),
        };

        handle_inventory(&mut ctx).await;
        assert_eq!(conn.session.ghost, 0);
        assert_eq!(conn.session.homdo[0].count, 1, "item must not be consumed");
        let has_toast = out.outgoing.iter().any(|f| f.contains("Ban khong bi ma theo") || f.contains("020102"));
        assert!(has_toast, "must send system message rejecting exorcism");
    }
}

#[tokio::test]
async fn test_npc_1752_exorcism_dialog() {
    let data = GameData::default();
    let service = BattleService::default();

    // 1. Success case: ghost == 1 and gold >= 500
    {
        let mut conn = Conn::default();
        conn.session.id = 3001;
        conn.session.ghost = 1;
        conn.session.gold = 1000;
        conn.session.map_id = 12001;
        conn.session.map_x = 400;
        conn.session.map_y = 400;

        let mut out = HandleOutcome::default();
        let payload = [0xD8, 0x06]; // NPC 1752 LE16
        let mut ctx = OpcodeCtx {
            conn: &mut conn,
            data: &data,
            service: &service,
            out: &mut out,
            opcode: 0x14,
            sub: 1,
            payload: &payload,
            decoded: &[],
            env: ServerEnv::none(),
        };

        handle_talk(&mut ctx).await;
        assert_eq!(conn.session.ghost, 0, "ghost must be dispelled");
        assert_eq!(conn.session.gold, 500, "500 gold must be deducted");

        let has_sync = out.outgoing.iter().any(|f| f.contains("1808") && f.contains("010000"));
        assert!(has_sync, "must emit 18 08 sync frame indicating ghost cleared");
    }

    // 2. Negative case: ghost == 1 but gold < 500 -> refused
    {
        let mut conn = Conn::default();
        conn.session.id = 3002;
        conn.session.ghost = 1;
        conn.session.gold = 200;
        conn.session.map_id = 12001;

        let mut out = HandleOutcome::default();
        let payload = [0xD8, 0x06];
        let mut ctx = OpcodeCtx {
            conn: &mut conn,
            data: &data,
            service: &service,
            out: &mut out,
            opcode: 0x14,
            sub: 1,
            payload: &payload,
            decoded: &[],
            env: ServerEnv::none(),
        };

        handle_talk(&mut ctx).await;
        assert_eq!(conn.session.ghost, 1, "ghost must remain");
        assert_eq!(conn.session.gold, 200, "gold must not be deducted");
        let has_err = out.outgoing.iter().any(|f| f.contains("Khong du tien") || f.contains("020102"));
        assert!(has_err, "must toast insufficient gold error");
    }

    // 3. Negative case: ghost == 0 -> refuses as not cursed
    {
        let mut conn = Conn::default();
        conn.session.id = 3003;
        conn.session.ghost = 0;
        conn.session.gold = 1000;
        conn.session.map_id = 12001;

        let mut out = HandleOutcome::default();
        let payload = [0xD8, 0x06];
        let mut ctx = OpcodeCtx {
            conn: &mut conn,
            data: &data,
            service: &service,
            out: &mut out,
            opcode: 0x14,
            sub: 1,
            payload: &payload,
            decoded: &[],
            env: ServerEnv::none(),
        };

        handle_talk(&mut ctx).await;
        assert_eq!(conn.session.ghost, 0);
        assert_eq!(conn.session.gold, 1000, "gold must not be deducted");
        let has_toast = out.outgoing.iter().any(|f| f.contains("Ban khong bi ma theo") || f.contains("020102"));
        assert!(has_toast, "must toast player is not ghosted");
    }
}

#[tokio::test]
async fn test_db_persistence_god_ghost() {
    let db = setup_test_db().await;
    let repos = SqliteRepositories::new(db.clone());

    let acc_id = ts_dream::db::accounts::create(&db, "quiz_tester", "pass")
        .await
        .expect("create account");

    let seed = CharacterSeed {
        level: 20,
        sex: 1,
        hair: 1,
        element: 1,
        map_id: 12001,
        map_x: 400,
        map_y: 400,
    };
    repos.characters().create(acc_id, b"GhostGodTester", &seed).await.expect("create character");

    let session = Session {
        id: acc_id as u32,
        db_character_id: acc_id,
        name: b"GhostGodTester".to_vec(),
        ghost: 1,
        god: 42,
        map_id: 12001,
        map_x: 400,
        map_y: 400,
        ..Default::default()
    };
    repos.sessions().save(&session).await.expect("save session");

    let mut loaded = Session {
        id: acc_id as u32,
        ..Default::default()
    };
    let found = repos.sessions().load(acc_id, &mut loaded).await.expect("load session");
    assert!(found, "session must exist");
    assert_eq!(loaded.ghost, 1, "hydrated session must have ghost=1");
    assert_eq!(loaded.god, 42, "hydrated session must have god=42");

    // Modify and save back
    loaded.ghost = 0;
    loaded.god = 41;
    repos.sessions().save(&loaded).await.expect("update session");

    let mut reloaded = Session {
        id: acc_id as u32,
        ..Default::default()
    };
    repos.sessions().load(acc_id, &mut reloaded).await.expect("reload session");
    assert_eq!(reloaded.ghost, 0, "persisted ghost must be updated to 0");
    assert_eq!(reloaded.god, 41, "persisted god must be updated to 41");
}

#[test]
fn test_quiz_modal_frame_emission_and_viscii_layout() {
    let question = "Ai la nguoi ket nghia dao vien?";
    let options = [
        "Quan Vu".to_string(),
        "Trieu Van".to_string(),
        "Ma Sieu".to_string(),
        "Hoang Trung".to_string(),
    ];

    let frame = ts_dream::server::events::build_quiz_modal_frame(question, &options);
    assert!(frame.starts_with("F444"), "must start with header F444");

    let raw = hex::decode(&frame).expect("valid hex");
    assert_eq!(raw[4], 0x10, "opcode must be 0x10");
    assert_eq!(raw[5], 0x01, "sub-opcode must be 0x01");

    let q_len = raw[6] as usize;
    let q_bytes = &raw[7..7 + q_len];
    assert_eq!(q_bytes, question.as_bytes());

    let mut offset = 7 + q_len;
    for (i, opt) in options.iter().enumerate().take(3) {
        let opt_len = raw[offset] as usize;
        offset += 1;
        let opt_bytes = &raw[offset..offset + opt_len];
        assert_eq!(opt_bytes, opt.as_bytes(), "option {} mismatch", i + 1);
        offset += opt_len;
    }
    // 4th option is the remaining payload
    let opt4_bytes = &raw[offset..];
    assert_eq!(opt4_bytes, options[3].as_bytes(), "option 4 mismatch");
}

#[tokio::test]
async fn test_anti_spectate_rejection_frame() {
    let data = GameData::default();
    let service = BattleService::default();

    // Register an active quiz for battle_id = 999
    let (_q, _frame) = service.events.register_quiz(999, 4001, None);
    assert!(service.is_north_star_battle(999));

    // Another player (id 4002) tries to spectate battle 999
    let mut conn = Conn::default();
    conn.session.id = 4002;
    conn.session.battle_id = 0;

    let mut out = HandleOutcome::default();
    // Opcode 0x0B sub 2 inner 4 (spectate), target = 999 LE32
    let mut payload = vec![4u8];
    payload.extend_from_slice(&999u32.to_le_bytes());

    let mut ctx = OpcodeCtx {
        conn: &mut conn,
        data: &data,
        service: &service,
        out: &mut out,
        opcode: 0x0B,
        sub: 2,
        payload: &payload,
        decoded: &[],
        env: ServerEnv::none(),
    };

    ts_dream::server::handlers::battle::handle_battle(&mut ctx);

    // Assert spectate was rejected with F4440300210103
    let rejected = out.outgoing.iter().any(|f| f.frame == "F4440300210103");
    assert!(
        rejected,
        "must emit anti-spectate rejection frame F4440300210103"
    );
    assert_eq!(conn.session.battle_id, 0, "spectator must not join battle");
}

#[tokio::test]
async fn test_quiz_correct_answer_awards_god_buff() {
    let data = GameData::default();
    let service = BattleService::default();

    // Setup active quiz for leader 5001, battle 101, correct answer = 1
    let custom_q = ts_dream::server::events::NorthStarQuestion {
        id: 10,
        question: "Cau hoi test".to_string(),
        options: ["Dap an dung".to_string(), "Sai 1".to_string(), "Sai 2".to_string(), "Sai 3".to_string()],
        correct: 1,
    };
    let (_q, _frame) = service.events.register_quiz(101, 5001, Some(custom_q));

    let mut conn = Conn::default();
    conn.session.id = 5001;
    conn.session.battle_id = 101;
    conn.session.god = 5;

    let mut out = HandleOutcome::default();
    // C->S 10 01 [answer_index: 1 LE32]
    let payload = 1u32.to_le_bytes();

    let mut ctx = OpcodeCtx {
        conn: &mut conn,
        data: &data,
        service: &service,
        out: &mut out,
        opcode: 0x10,
        sub: 1,
        payload: &payload,
        decoded: &[],
        env: ServerEnv::none(),
    };

    ts_dream::server::handlers::battle::handle_quiz_answer(&mut ctx).await;

    // Leader receives S->C 10 02 01
    assert!(out.outgoing.iter().any(|f| f.frame == "F4440300100201"), "must emit correct answer modal frame 10 02 01");
    // Leader receives +10 God buff (5 + 10 = 15)
    assert_eq!(conn.session.god, 15, "god buff must be incremented by 10");
    // Emits S->C 18 08 [playerId][02 00][01]
    assert!(out.outgoing.iter().any(|f| f.frame.contains("1808") && f.frame.contains("020001")), "must emit state sync frame for God buff");
    // Battle ended
    assert_eq!(conn.session.battle_id, 0, "must exit combat");
}

#[tokio::test]
async fn test_quiz_incorrect_answer_penalizes_ghost_and_hp1() {
    let data = GameData::default();
    let service = BattleService::default();

    // Setup active quiz for leader 5002, battle 102, correct answer = 1
    let custom_q = ts_dream::server::events::NorthStarQuestion {
        id: 11,
        question: "Cau hoi test".to_string(),
        options: ["Dap an dung".to_string(), "Sai 1".to_string(), "Sai 2".to_string(), "Sai 3".to_string()],
        correct: 1,
    };
    let (_q, _frame) = service.events.register_quiz(102, 5002, Some(custom_q));

    let mut conn = Conn::default();
    conn.session.id = 5002;
    conn.session.battle_id = 102;
    conn.session.hp = 1200;
    conn.session.ghost = 0;

    let mut out = HandleOutcome::default();
    // C->S 10 01 [answer_index: 3 LE32] -> Wrong!
    let payload = 3u32.to_le_bytes();

    let mut ctx = OpcodeCtx {
        conn: &mut conn,
        data: &data,
        service: &service,
        out: &mut out,
        opcode: 0x10,
        sub: 1,
        payload: &payload,
        decoded: &[],
        env: ServerEnv::none(),
    };

    ts_dream::server::handlers::battle::handle_quiz_answer(&mut ctx).await;

    // Leader receives S->C 10 02 02
    assert!(out.outgoing.iter().any(|f| f.frame == "F4440300100202"), "must emit incorrect answer modal frame 10 02 02");
    // Leader HP drops to 1
    assert_eq!(conn.session.hp, 1, "leader HP must drop to 1");
    // Afflicted with Ghost
    assert_eq!(conn.session.ghost, 1, "leader must receive ghost debuff");
    // Emits S->C 18 08 [playerId][01 00][01]
    assert!(out.outgoing.iter().any(|f| f.frame.contains("1808") && f.frame.contains("010001")), "must emit state sync frame for Ghost");
    // Battle ended
    assert_eq!(conn.session.battle_id, 0, "must exit combat");
}

#[tokio::test]
async fn test_quiz_party_leader_modal_and_member_isolation() {
    let data = GameData::default();
    let service = BattleService::default();

    let leader_id = 6001u32;
    let member_id = 6002u32;

    // Leader session
    let mut leader_session = Session::default();
    leader_session.id = leader_id;
    leader_session.hp = 1500;
    leader_session.id_leader = leader_id;
    leader_session.id_mem = [member_id, 0, 0, 0];
    leader_session.ghost = 0;
    leader_session.god = 0;

    // Member session
    let mut member_session = Session::default();
    member_session.id = member_id;
    member_session.hp = 800;
    member_session.id_leader = leader_id;
    member_session.ghost = 0;
    member_session.god = 0;

    // Start North Star battle
    let (battle_id, modal_frame) = service.start_north_star_battle(&mut leader_session, 1643, 112);
    assert_eq!(leader_session.battle_id, battle_id);
    assert!(modal_frame.starts_with("F444"), "modal frame generated for leader");

    // Leader submits incorrect answer
    let mut conn = Conn::default();
    conn.session = leader_session;

    let mut out = HandleOutcome::default();
    let payload = 4u32.to_le_bytes(); // Incorrect answer

    let mut ctx = OpcodeCtx {
        conn: &mut conn,
        data: &data,
        service: &service,
        out: &mut out,
        opcode: 0x10,
        sub: 1,
        payload: &payload,
        decoded: &[],
        env: ServerEnv::none(),
    };

    ts_dream::server::handlers::battle::handle_quiz_answer(&mut ctx).await;

    // Leader punished
    assert_eq!(conn.session.hp, 1);
    assert_eq!(conn.session.ghost, 1);

    // Member strictly untouched
    assert_eq!(member_session.hp, 800, "member HP must remain untouched");
    assert_eq!(member_session.ghost, 0, "member must NOT receive ghost debuff");
    assert_eq!(member_session.god, 0, "member god must remain 0");
}

#[tokio::test]
async fn test_quiz_45s_timeout_resolves_incorrect() {
    let service = BattleService::default();

    // Create a quiz with 0ms timeout to simulate immediate expiration
    let events = ts_dream::server::events::EventLifecycleManager::default()
        .with_quiz_timeout(std::time::Duration::from_millis(0));
    let service = service.with_events(std::sync::Arc::new(events));

    let (_q, _frame) = service.events.register_quiz(201, 7001, None);

    // Timeout check
    let res = service.events.check_quiz_timeout(201);
    assert!(res.is_some(), "expired quiz must resolve");
    let res = res.unwrap();
    assert_eq!(res.leader_id, 7001);
    assert!(!res.is_correct, "timeout must resolve as incorrect");
    assert_eq!(res.result_frame, "F4440300100202", "must emit 10 02 02 failure frame");
}

#[tokio::test]
async fn test_quiz_party_members_evacuated_on_answer() {
    let data = GameData::default();
    let service = BattleService::default();

    let leader_id = 7101u32;
    let member1_id = 7102u32;
    let member2_id = 7103u32;

    // Register members in online sessions
    {
        let mut online = ts_dream::server::session::lock_online_sessions();
        let mut m1 = Session::default();
        m1.id = member1_id;
        m1.id_leader = leader_id;
        m1.battle_id = 0;
        online.insert(member1_id, m1);

        let mut m2 = Session::default();
        m2.id = member2_id;
        m2.id_leader = leader_id;
        m2.battle_id = 0;
        online.insert(member2_id, m2);
    }

    let mut leader_session = Session::default();
    leader_session.id = leader_id;
    leader_session.id_leader = leader_id;
    leader_session.id_mem = [member1_id, member2_id, 0, 0];
    leader_session.battle_id = 0;

    let (battle_id, _modal) = service.start_north_star_battle(&mut leader_session, 1643, 112);
    assert_eq!(leader_session.battle_id, battle_id);

    // Verify online sessions have battle_id set for members
    {
        let online = ts_dream::server::session::lock_online_sessions();
        assert_eq!(online.get(&member1_id).unwrap().battle_id, battle_id);
        assert_eq!(online.get(&member2_id).unwrap().battle_id, battle_id);
    }

    // Leader submits correct answer
    let mut conn = Conn::default();
    conn.session = leader_session;

    let mut out = HandleOutcome::default();
    let payload = 1u32.to_le_bytes(); // Assume question 1 correct

    let mut ctx = OpcodeCtx {
        conn: &mut conn,
        data: &data,
        service: &service,
        out: &mut out,
        opcode: 0x10,
        sub: 1,
        payload: &payload,
        decoded: &[],
        env: ServerEnv::none(),
    };

    ts_dream::server::handlers::battle::handle_quiz_answer(&mut ctx).await;

    // Leader and both members must be back to battle_id = 0
    assert_eq!(conn.session.battle_id, 0, "leader must exit battle");
    {
        let online = ts_dream::server::session::lock_online_sessions();
        assert_eq!(online.get(&member1_id).unwrap().battle_id, 0, "member 1 must exit battle");
        assert_eq!(online.get(&member2_id).unwrap().battle_id, 0, "member 2 must exit battle");
    }
    assert!(!service.events.is_north_star_battle(battle_id), "quiz session must be cleaned up");
}

#[tokio::test]
async fn test_quiz_party_members_evacuated_on_leader_disconnect() {
    let app = std::sync::Arc::new(tokio::sync::RwLock::new(ts_dream::state::AppState::new(1)));
    app.write().await.running = true;

    let control = std::sync::Arc::new(ts_dream::web::server_control::ServerControl::new(
        19299,
        app.clone(),
        None,
        None,
    ));

    let leader_id = 7201u32;
    let member_id = 7202u32;

    // Set up sessions
    let mut leader_session = Session::default();
    leader_session.id = leader_id;
    leader_session.authed = true;
    leader_session.id_leader = leader_id;
    leader_session.id_mem = [member_id, 0, 0, 0];

    let mut member_session = Session::default();
    member_session.id = member_id;
    member_session.authed = true;
    member_session.id_leader = leader_id;

    {
        let mut online = ts_dream::server::session::lock_online_sessions();
        online.insert(member_id, member_session);
    }

    let (battle_id, _modal) = control.battle_service.start_north_star_battle(&mut leader_session, 1643, 112);
    {
        let mut online = ts_dream::server::session::lock_online_sessions();
        online.insert(leader_id, leader_session);
    }

    assert!(control.events.is_north_star_battle(battle_id));

    // Leader disconnects
    control.disconnect_player(leader_id).await;

    // Member must have battle_id reset to 0
    {
        let online = ts_dream::server::session::lock_online_sessions();
        assert_eq!(
            online.get(&member_id).unwrap().battle_id,
            0,
            "member must be cleanly evicted from quiz battle on leader disconnect"
        );
    }
    assert!(
        !control.events.is_north_star_battle(battle_id),
        "quiz battle must be aborted on leader disconnect"
    );
}


