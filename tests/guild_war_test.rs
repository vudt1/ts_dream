//! Integration tests for Ticket 07: Guild Warfare, City Siege, Water War, and Dragon Boat Racing.
//!
//! Verifies:
//! - Guild relational SQLite persistence and schema models (guilds, guild_members, guild_cities)
//! - Guild creation, invite with Two-Phase State Lock, and leadership transfer
//! - City Siege 5-city ownership table (`S→C 29 0F`), announcements (`29 09`), and gate barriers (`0x03`)
//! - Water War 25-slot temporary boat inventory initialization (`17 64`) and cleanup on departure (`17 65`)
//! - Naval bombardment combat synchronization (`40 01..03`)
//! - Dragon Boat Racing token bucket rate limiter (5 packets/second ceiling) and alternating paddle inputs

use ts_dream::battle::service::BattleService;
use ts_dream::data::loader::GameData;
use ts_dream::db::domain::{create_guild, list_guilds, update_city_owner};
use ts_dream::server::dispatcher::{HandleOutcome, OpcodeCtx, ServerEnv};
use ts_dream::server::handlers::guild::{
    build_city_gate_barrier_raw, build_city_ownership_raw, build_city_siege_notice_raw,
    build_naval_damage_raw, build_naval_effect_raw, build_naval_status_raw,
    check_dragon_boat_rate_limit, city_gate_barrier_frame, city_ownership_frame,
    enter_water_war, handle_boat_race, handle_guild, leave_water_war, CITY_GIANG_LANG,
    CITY_KHUC_A, CITY_LAC_DUONG, CITY_THO_XUAN, CITY_TRUONG_AN, DRAGON_BOAT_MAX_TOKENS,
    PADDLE_LEFT, PADDLE_RIGHT,
};
use ts_dream::server::session::{lock_online_sessions, Conn, Session};

mod common;
use common::test_db::setup_test_db;

#[tokio::test]
async fn test_guild_sqlite_schema_and_domain() {
    let pool = setup_test_db().await;

    // 1. Initial city table should be pre-seeded with 5 cities
    let guilds_before = list_guilds(&pool).await.expect("list guilds");
    assert_eq!(guilds_before.len(), 0);

    // 2. Create guild in SQLite
    let guild_id = create_guild(&pool, "LongHoMon", 1001).await.expect("create guild");
    assert!(guild_id > 0);

    let guilds_after = list_guilds(&pool).await.expect("list guilds after");
    assert_eq!(guilds_after.len(), 1);
    assert_eq!(guilds_after[0].name, "LongHoMon");
    assert_eq!(guilds_after[0].leader_id, 1001);
    assert_eq!(guilds_after[0].member_count, 1);

    // 3. Assign city ownership
    update_city_owner(&pool, 1, Some(guild_id)).await.expect("update city owner");
}

#[tokio::test]
async fn test_guild_creation_and_invite_2phase_lock() {
    let pool = setup_test_db().await;
    let data = GameData::default();
    let service = BattleService::default();

    // Player 1 creates guild
    let mut conn1 = Conn::default();
    conn1.session.id = 2001;
    conn1.session.name = b"BangChu".to_vec();

    // Register Player 2 in online sessions
    let mut conn2 = Conn::default();
    conn2.session.id = 2002;
    conn2.session.name = b"ThanhVien".to_vec();
    {
        let mut online = lock_online_sessions();
        online.insert(2001, conn1.session.clone());
        online.insert(2002, conn2.session.clone());
    }

    // 1. Create guild via opcode 0x27 sub 0x01
    {
        let mut out = HandleOutcome::default();
        let payload = ts_dream::encoding::viscii_encode("DaiVietGuild");
        let mut ctx = OpcodeCtx {
            conn: &mut conn1,
            data: &data,
            service: &service,
            out: &mut out,
            opcode: 0x27,
            sub: 0x01,
            payload: &payload,
            decoded: &[],
            env: ServerEnv::with_pool(&pool),
        };

        handle_guild(&mut ctx).await;
        assert!(conn1.session.guild_id > 0, "Player 1 should now belong to guild");
        assert!(out.outgoing.iter().any(|f| f.frame.contains("F4440300270100")));
    }

    // 2. Player 1 invites Player 2 via opcode 0x27 sub 0x02
    {
        let mut out = HandleOutcome::default();
        let payload = 2002u32.to_le_bytes();
        let mut ctx = OpcodeCtx {
            conn: &mut conn1,
            data: &data,
            service: &service,
            out: &mut out,
            opcode: 0x27,
            sub: 0x02,
            payload: &payload,
            decoded: &[],
            env: ServerEnv::with_pool(&pool),
        };

        handle_guild(&mut ctx).await;
        // Verify invite frame sent
        assert!(out.outgoing.iter().any(|f| f.frame.starts_with("F444") && f.frame.contains("2702")));
    }

    // 3. Player 2 accepts invite via opcode 0x27 sub 0x03
    {
        conn2.session.pending_guild_invite = lock_online_sessions().get(&2002).unwrap().pending_guild_invite;
        let mut out = HandleOutcome::default();
        let payload = [1u8];
        let mut ctx = OpcodeCtx {
            conn: &mut conn2,
            data: &data,
            service: &service,
            out: &mut out,
            opcode: 0x27,
            sub: 0x03,
            payload: &payload,
            decoded: &[],
            env: ServerEnv::with_pool(&pool),
        };

        handle_guild(&mut ctx).await;
        assert_eq!(conn2.session.guild_id, conn1.session.guild_id, "Player 2 should join guild after consent");
        assert!(out.outgoing.iter().any(|f| f.frame.contains("270301")));
    }

    // Clean up
    {
        let mut online = lock_online_sessions();
        online.remove(&2001);
        online.remove(&2002);
    }
}

#[test]
fn test_city_siege_table_encoding_and_announcements() {
    // 5 cities ownership table: S->C 29 0F
    let cities = [
        (CITY_THO_XUAN, 101, "Nguy"),
        (CITY_TRUONG_AN, 102, "Thuc"),
        (CITY_GIANG_LANG, 103, "Ngo"),
        (CITY_KHUC_A, 0, ""),
        (CITY_LAC_DUONG, 104, "Han"),
    ];

    let raw = build_city_ownership_raw(&cities);
    assert_eq!(raw[0..2], [0xF4, 0x44]);
    assert_eq!(raw[4], 0x29);
    assert_eq!(raw[5], 0x0F);
    assert_eq!(raw[6], 5, "5 cities");

    let hex_str = city_ownership_frame(&cities);
    assert!(hex_str.starts_with("F444"));
    assert!(hex_str.contains("290F05"));

    // 13 warfare announcements: S->C 29 09
    for code in 1..=13 {
        let notice_raw = build_city_siege_notice_raw(code, CITY_THO_XUAN, Some("QuanDoanA"));
        assert_eq!(notice_raw[0..2], [0xF4, 0x44]);
        assert_eq!(notice_raw[4], 0x29);
        assert_eq!(notice_raw[5], 0x09);
        assert_eq!(notice_raw[6], code);
        assert_eq!(notice_raw[7], CITY_THO_XUAN);
    }

    // City gate collision barrier: 0x03
    let barrier_intact = build_city_gate_barrier_raw(CITY_THO_XUAN, 1, true);
    assert_eq!(barrier_intact, vec![0xF4, 0x44, 0x04, 0x00, 0x03, 0x01, 0x01, 0x01]);
    assert_eq!(city_gate_barrier_frame(CITY_THO_XUAN, 1, true), "F444040003010101");

    let barrier_broken = build_city_gate_barrier_raw(CITY_THO_XUAN, 1, false);
    assert_eq!(barrier_broken, vec![0xF4, 0x44, 0x04, 0x00, 0x03, 0x01, 0x01, 0x00]);
    assert_eq!(city_gate_barrier_frame(CITY_THO_XUAN, 1, false), "F444040003010100");
}

#[test]
fn test_water_war_boat_inventory_lifecycle_and_bombardment() {
    let mut session = Session::default();
    session.id = 3001;

    // 1. Enter Water War -> 25 slot inventory initialized & 17 64 frame emitted
    let init_frame = enter_water_war(&mut session);
    assert_eq!(session.boat_inventory.len(), 25);
    assert!(init_frame.starts_with("F444") && init_frame.contains("176419")); // 25 = 0x19

    // 2. Leave Water War -> boat inventory cleared & 17 65 frame emitted
    let clear_frame = leave_water_war(&mut session);
    assert_eq!(session.boat_inventory.len(), 0);
    assert_eq!(clear_frame, "F44402001765");

    // 3. Naval bombardment synchronization: S->C 40 01..03
    let damage_raw = build_naval_damage_raw(1, 5001, 250);
    assert_eq!(damage_raw[0..2], [0xF4, 0x44]);
    assert_eq!(damage_raw[4], 0x40);
    assert_eq!(damage_raw[5], 0x01);

    let effect_raw = build_naval_effect_raw(1, 5001);
    assert_eq!(effect_raw[0..2], [0xF4, 0x44]);
    assert_eq!(effect_raw[4], 0x40);
    assert_eq!(effect_raw[5], 0x02);

    let status_raw = build_naval_status_raw(5001, 1);
    assert_eq!(status_raw[0..2], [0xF4, 0x44]);
    assert_eq!(status_raw[4], 0x40);
    assert_eq!(status_raw[5], 0x03);
}

#[test]
fn test_dragon_boat_racing_rate_limiter_and_alternating_paddles() {
    let mut tokens = DRAGON_BOAT_MAX_TOKENS;
    let mut last_ms = 1_000_000u64;

    // 1. Rapid fire: first 5 packets within 100ms should succeed
    for i in 0..5 {
        let accepted = check_dragon_boat_rate_limit(&mut tokens, &mut last_ms, 1_000_000 + i * 10);
        assert!(accepted, "packet {} should be accepted within initial burst capacity", i + 1);
    }

    // 6th packet immediately after should be DROPPED (rate limited)
    let dropped = check_dragon_boat_rate_limit(&mut tokens, &mut last_ms, 1_000_050);
    assert!(!dropped, "6th packet within 50ms burst must be dropped by token bucket");

    // After 1 full second, tokens refill by 5 -> accepted again
    let accepted_after_1s = check_dragon_boat_rate_limit(&mut tokens, &mut last_ms, 1_001_100);
    assert!(accepted_after_1s, "packet after 1s refill must be accepted");

    // 2. Alternating Left / Right paddle flow on dragon boat map 60515
    let mut conn = Conn::default();
    conn.session.id = 7001;
    conn.session.map_id = 60515;
    conn.session.map_x = 100;
    conn.session.boat_paddle_tokens = 5.0;
    conn.session.boat_last_paddle = 0;

    let data = GameData::default();
    let service = BattleService::default();

    // Paddle Left (1)
    {
        let mut out = HandleOutcome::default();
        let payload = [PADDLE_LEFT];
        let mut ctx = OpcodeCtx {
            conn: &mut conn,
            data: &data,
            service: &service,
            out: &mut out,
            opcode: 0x46,
            sub: 0x01,
            payload: &payload,
            decoded: &[],
            env: ServerEnv::none(),
        };

        tokio::runtime::Runtime::new().unwrap().block_on(async {
            handle_boat_race(&mut ctx).await;
        });

        assert_eq!(conn.session.boat_last_paddle, PADDLE_LEFT);
        assert_eq!(conn.session.map_x, 105, "paddle advances boat position");
        assert_eq!(out.outgoing.len(), 1);
    }

    // Identical paddle Left (1) again -> rejected (no advance)
    {
        let mut out = HandleOutcome::default();
        let payload = [PADDLE_LEFT];
        let mut ctx = OpcodeCtx {
            conn: &mut conn,
            data: &data,
            service: &service,
            out: &mut out,
            opcode: 0x46,
            sub: 0x01,
            payload: &payload,
            decoded: &[],
            env: ServerEnv::none(),
        };

        tokio::runtime::Runtime::new().unwrap().block_on(async {
            handle_boat_race(&mut ctx).await;
        });

        assert_eq!(conn.session.map_x, 105, "identical consecutive paddle must not advance position");
        assert_eq!(out.outgoing.len(), 0);
    }

    // Alternating paddle Right (2) -> accepted and advances!
    {
        let mut out = HandleOutcome::default();
        let payload = [PADDLE_RIGHT];
        let mut ctx = OpcodeCtx {
            conn: &mut conn,
            data: &data,
            service: &service,
            out: &mut out,
            opcode: 0x46,
            sub: 0x01,
            payload: &payload,
            decoded: &[],
            env: ServerEnv::none(),
        };

        tokio::runtime::Runtime::new().unwrap().block_on(async {
            handle_boat_race(&mut ctx).await;
        });

        assert_eq!(conn.session.boat_last_paddle, PADDLE_RIGHT);
        assert_eq!(conn.session.map_x, 110, "alternating paddle advances boat position");
        assert_eq!(out.outgoing.len(), 1);
        assert_eq!(out.map_broadcast.len(), 1, "boat advance must broadcast to other players on map");
    }
}

#[tokio::test]
async fn test_water_war_opcode_3f_dispatch_and_inventory_lifecycle() {
    let mut conn = Conn::default();
    conn.session.id = 8001;
    let data = GameData::default();
    let service = BattleService::default();

    // 1. Enter Water War via Opcode 0x3F Sub 0x01
    {
        let mut out = HandleOutcome::default();
        let mut ctx = OpcodeCtx {
            conn: &mut conn,
            data: &data,
            service: &service,
            out: &mut out,
            opcode: 0x3F,
            sub: 0x01,
            payload: &[],
            decoded: &[],
            env: ServerEnv::none(),
        };

        ts_dream::server::handlers::guild::handle_water_war(&mut ctx).await;
        assert_eq!(conn.session.boat_inventory.len(), 25);
        assert!(out.outgoing.iter().any(|f| f.frame.starts_with("F444") && f.frame.contains("176419")));
    }

    // 2. Leave Water War via Opcode 0x3F Sub 0x02
    {
        let mut out = HandleOutcome::default();
        let mut ctx = OpcodeCtx {
            conn: &mut conn,
            data: &data,
            service: &service,
            out: &mut out,
            opcode: 0x3F,
            sub: 0x02,
            payload: &[],
            decoded: &[],
            env: ServerEnv::none(),
        };

        ts_dream::server::handlers::guild::handle_water_war(&mut ctx).await;
        assert_eq!(conn.session.boat_inventory.len(), 0);
        assert!(out.outgoing.iter().any(|f| f.frame == "F44402001765"));
    }
}

#[tokio::test]
async fn test_guild_leadership_transfer_and_kick_authorization() {
    let pool = setup_test_db().await;
    let data = GameData::default();
    let service = BattleService::default();

    // Player 1 creates guild
    let mut conn1 = Conn::default();
    conn1.session.id = 9101;
    conn1.session.name = b"LeaderA".to_vec();

    let mut conn2 = Conn::default();
    conn2.session.id = 9102;
    conn2.session.name = b"MemberB".to_vec();

    {
        let mut online = lock_online_sessions();
        online.insert(9101, conn1.session.clone());
        online.insert(9102, conn2.session.clone());
    }

    // 1. Player 1 creates guild
    {
        let mut out = HandleOutcome::default();
        let payload = ts_dream::encoding::viscii_encode("HoangKimGuild");
        let mut ctx = OpcodeCtx {
            conn: &mut conn1,
            data: &data,
            service: &service,
            out: &mut out,
            opcode: 0x27,
            sub: 0x01,
            payload: &payload,
            decoded: &[],
            env: ServerEnv::with_pool(&pool),
        };
        handle_guild(&mut ctx).await;
    }
    let guild_id = conn1.session.guild_id;
    assert!(guild_id > 0);

    // 2. Player 1 invites Player 2
    {
        let mut out = HandleOutcome::default();
        let payload = 9102u32.to_le_bytes();
        let mut ctx = OpcodeCtx {
            conn: &mut conn1,
            data: &data,
            service: &service,
            out: &mut out,
            opcode: 0x27,
            sub: 0x02,
            payload: &payload,
            decoded: &[],
            env: ServerEnv::with_pool(&pool),
        };
        handle_guild(&mut ctx).await;
    }

    // Player 2 accepts invite via 0x27 sub 0x03
    {
        conn2.session.pending_guild_invite = lock_online_sessions().get(&9102).unwrap().pending_guild_invite;
        let mut out = HandleOutcome::default();
        let payload = [1u8];
        let mut ctx = OpcodeCtx {
            conn: &mut conn2,
            data: &data,
            service: &service,
            out: &mut out,
            opcode: 0x27,
            sub: 0x03,
            payload: &payload,
            decoded: &[],
            env: ServerEnv::with_pool(&pool),
        };
        handle_guild(&mut ctx).await;
    }

    // Member 2 is now member in online sessions
    {
        let online = lock_online_sessions();
        assert_eq!(online.get(&9102).unwrap().guild_id, guild_id);
    }

    // 3. Unauthorized transfer attempt: Member 2 tries to transfer leadership to 9103
    {
        let mut out = HandleOutcome::default();
        conn2.session.guild_id = guild_id;
        let payload = 9103u32.to_le_bytes();
        let mut ctx = OpcodeCtx {
            conn: &mut conn2,
            data: &data,
            service: &service,
            out: &mut out,
            opcode: 0x27,
            sub: 0x05,
            payload: &payload,
            decoded: &[],
            env: ServerEnv::with_pool(&pool),
        };
        handle_guild(&mut ctx).await;
        // Should reject unauthorized transfer
        let expected_msg = ts_dream::server::spawn::sys_msg_frame("Chi co Quan Chu moi co quyen chuyen nhuong!");
        assert!(out.outgoing.iter().any(|f| f.frame == expected_msg));
    }

    // 4. Authorized transfer: Leader 1 transfers to Member 2
    {
        let mut out = HandleOutcome::default();
        let payload = 9102u32.to_le_bytes();
        let mut ctx = OpcodeCtx {
            conn: &mut conn1,
            data: &data,
            service: &service,
            out: &mut out,
            opcode: 0x27,
            sub: 0x05,
            payload: &payload,
            decoded: &[],
            env: ServerEnv::with_pool(&pool),
        };
        handle_guild(&mut ctx).await;
        let expected_success = ts_dream::server::spawn::sys_msg_frame("Chuyen nhuong chuc vu Quan Chu thanh cong!");
        assert!(out.outgoing.iter().any(|f| f.frame == expected_success));
    }

    // Verify DB: Player 2 is now leader
    let leader_in_db = sqlx::query_scalar::<_, i64>("SELECT leader_id FROM guilds WHERE guild_id = ?")
        .bind(guild_id as i64)
        .fetch_one(&pool.read)
        .await
        .unwrap();
    assert_eq!(leader_in_db, 9102);

    // Verify rank: Player 2 is rank 2, Player 1 is rank 0
    let rank1 = sqlx::query_scalar::<_, i32>("SELECT rank FROM guild_members WHERE char_id = ?")
        .bind(9101i64)
        .fetch_one(&pool.read)
        .await
        .unwrap();
    let rank2 = sqlx::query_scalar::<_, i32>("SELECT rank FROM guild_members WHERE char_id = ?")
        .bind(9102i64)
        .fetch_one(&pool.read)
        .await
        .unwrap();
    assert_eq!(rank1, 0, "old leader should be demoted to rank 0");
    assert_eq!(rank2, 2, "new leader should be rank 2");

    // Clean up
    {
        let mut online = lock_online_sessions();
        online.remove(&9101);
        online.remove(&9102);
    }
}

#[tokio::test]
async fn test_guild_consent_decline_and_uninvited_rejection() {
    let pool = setup_test_db().await;
    let data = GameData::default();
    let service = BattleService::default();

    let mut conn = Conn::default();
    conn.session.id = 8001;
    ts_dream::server::session::registry_register_for_login(&conn.session);

    // 1. Uninvited accept should reject
    {
        let mut out = HandleOutcome::default();
        let payload = [1u8];
        let mut ctx = OpcodeCtx {
            conn: &mut conn,
            data: &data,
            service: &service,
            out: &mut out,
            opcode: 0x27,
            sub: 0x03,
            payload: &payload,
            decoded: &[],
            env: ServerEnv::with_pool(&pool),
        };
        handle_guild(&mut ctx).await;
        let expected = ts_dream::server::spawn::sys_msg_frame("Khong co loi moi gia nhap quan doan");
        assert!(out.outgoing.iter().any(|f| f.frame == expected));
    }

    // 2. Decline flow (choice = 0)
    conn.session.pending_guild_invite = Some((8002, 10));
    {
        let mut out = HandleOutcome::default();
        let payload = [0u8];
        let mut ctx = OpcodeCtx {
            conn: &mut conn,
            data: &data,
            service: &service,
            out: &mut out,
            opcode: 0x27,
            sub: 0x03,
            payload: &payload,
            decoded: &[],
            env: ServerEnv::with_pool(&pool),
        };
        handle_guild(&mut ctx).await;
        assert_eq!(conn.session.guild_id, 0);
        assert_eq!(conn.session.pending_guild_invite, None);
        let expected = ts_dream::server::spawn::sys_msg_frame("Da tu choi loi moi quan doan");
        assert!(out.outgoing.iter().any(|f| f.frame == expected));
    }

    lock_online_sessions().remove(&8001);
}

#[tokio::test]
async fn test_guild_sub_opcode_aliases() {
    let pool = setup_test_db().await;
    let data = GameData::default();
    let service = BattleService::default();

    let mut conn1 = Conn::default();
    conn1.session.id = 8101;
    let mut conn2 = Conn::default();
    conn2.session.id = 8102;

    ts_dream::server::session::registry_register_for_login(&conn1.session);
    ts_dream::server::session::registry_register_for_login(&conn2.session);

    // Create guild
    let guild_id = create_guild(&pool, "AliasGuild", 8101).await.expect("create guild");
    conn1.session.guild_id = guild_id as u32;
    ts_dream::server::session::registry_update_if_online(&conn1.session);

    // Add player 2
    ts_dream::db::domain::add_guild_member(&pool, guild_id, 8102, 0).await.expect("add member");
    conn2.session.guild_id = guild_id as u32;
    ts_dream::server::session::registry_update_if_online(&conn2.session);

    // 1. Kick alias 0x07 (instead of 0x04)
    {
        let mut out = HandleOutcome::default();
        let payload = 8102u32.to_le_bytes();
        let mut ctx = OpcodeCtx {
            conn: &mut conn1,
            data: &data,
            service: &service,
            out: &mut out,
            opcode: 0x27,
            sub: 0x07, // Alias of 0x04
            payload: &payload,
            decoded: &[],
            env: ServerEnv::with_pool(&pool),
        };
        handle_guild(&mut ctx).await;
        let expected = ts_dream::server::spawn::sys_msg_frame("Da truc xuat thanh vien khoi quan doan");
        assert!(out.outgoing.iter().any(|f| f.frame == expected));
        assert_eq!(lock_online_sessions().get(&8102).unwrap().guild_id, 0);
    }

    // Re-add player 2 for transfer test
    ts_dream::db::domain::add_guild_member(&pool, guild_id, 8102, 0).await.expect("re-add member");
    {
        let mut online = lock_online_sessions();
        online.get_mut(&8102).unwrap().guild_id = guild_id as u32;
    }

    // 2. Transfer alias 0x0A (instead of 0x05)
    {
        let mut out = HandleOutcome::default();
        let payload = 8102u32.to_le_bytes();
        let mut ctx = OpcodeCtx {
            conn: &mut conn1,
            data: &data,
            service: &service,
            out: &mut out,
            opcode: 0x27,
            sub: 0x0A, // Alias of 0x05
            payload: &payload,
            decoded: &[],
            env: ServerEnv::with_pool(&pool),
        };
        handle_guild(&mut ctx).await;
        let expected = ts_dream::server::spawn::sys_msg_frame("Chuyen nhuong chuc vu Quan Chu thanh cong!");
        assert!(out.outgoing.iter().any(|f| f.frame == expected));
    }

    // 3. Badge upload alias 0x12 (instead of 0x06)
    {
        let mut out = HandleOutcome::default();
        let payload = vec![0xAB; 288];
        let mut ctx = OpcodeCtx {
            conn: &mut conn2, // new leader 8102
            data: &data,
            service: &service,
            out: &mut out,
            opcode: 0x27,
            sub: 0x12, // Alias of 0x06
            payload: &payload,
            decoded: &[],
            env: ServerEnv::with_pool(&pool),
        };
        handle_guild(&mut ctx).await;
        let expected = ts_dream::server::spawn::sys_msg_frame("Cap nhat huy hieu quan doan thanh cong!");
        assert!(out.outgoing.iter().any(|f| f.frame == expected));
    }

    lock_online_sessions().remove(&8101);
    lock_online_sessions().remove(&8102);
}

#[tokio::test]
async fn test_guild_negative_cases() {
    let pool = setup_test_db().await;
    let data = GameData::default();
    let service = BattleService::default();

    let mut conn = Conn::default();
    conn.session.id = 8201;
    conn.session.guild_id = 99;
    ts_dream::server::session::registry_register_for_login(&conn.session);

    // 1. Self-invite -> rejected
    {
        let mut out = HandleOutcome::default();
        let payload = 8201u32.to_le_bytes();
        let mut ctx = OpcodeCtx {
            conn: &mut conn,
            data: &data,
            service: &service,
            out: &mut out,
            opcode: 0x27,
            sub: 0x02,
            payload: &payload,
            decoded: &[],
            env: ServerEnv::with_pool(&pool),
        };
        handle_guild(&mut ctx).await;
        let expected = ts_dream::server::spawn::sys_msg_frame("Khong the tu moi chinh minh");
        assert!(out.outgoing.iter().any(|f| f.frame == expected));
    }

    // 2. Self-kick -> rejected
    {
        let mut out = HandleOutcome::default();
        let payload = 8201u32.to_le_bytes();
        let mut ctx = OpcodeCtx {
            conn: &mut conn,
            data: &data,
            service: &service,
            out: &mut out,
            opcode: 0x27,
            sub: 0x04,
            payload: &payload,
            decoded: &[],
            env: ServerEnv::with_pool(&pool),
        };
        handle_guild(&mut ctx).await;
        let expected = ts_dream::server::spawn::sys_msg_frame("Khong the tu truc xuat chinh minh");
        assert!(out.outgoing.iter().any(|f| f.frame == expected));
    }

    // 3. Self-transfer -> rejected
    {
        let mut out = HandleOutcome::default();
        let payload = 8201u32.to_le_bytes();
        let mut ctx = OpcodeCtx {
            conn: &mut conn,
            data: &data,
            service: &service,
            out: &mut out,
            opcode: 0x27,
            sub: 0x05,
            payload: &payload,
            decoded: &[],
            env: ServerEnv::with_pool(&pool),
        };
        handle_guild(&mut ctx).await;
        let expected = ts_dream::server::spawn::sys_msg_frame("Khong the chuyen nhuong cho chinh minh");
        assert!(out.outgoing.iter().any(|f| f.frame == expected));
    }

    // 4. Action without guild (guild_id == 0)
    conn.session.guild_id = 0;
    {
        let mut out = HandleOutcome::default();
        let payload = 8202u32.to_le_bytes();
        let mut ctx = OpcodeCtx {
            conn: &mut conn,
            data: &data,
            service: &service,
            out: &mut out,
            opcode: 0x27,
            sub: 0x02,
            payload: &payload,
            decoded: &[],
            env: ServerEnv::with_pool(&pool),
        };
        handle_guild(&mut ctx).await;
        let expected = ts_dream::server::spawn::sys_msg_frame("Ban chua co quan doan");
        assert!(out.outgoing.iter().any(|f| f.frame == expected));
    }

    lock_online_sessions().remove(&8201);
}

#[tokio::test]
async fn test_city_siege_dispatcher_routing() {
    let pool = setup_test_db().await;
    let data = GameData::default();
    let service = BattleService::default();
    let env = ServerEnv::with_pool(&pool);

    let mut conn = Conn::default();
    conn.session.id = 8301;
    ts_dream::server::session::registry_register_for_login(&conn.session);

    // 1. Dispatch 0x29 0x0F (City ownership table query)
    // Frame: F4 44 [len: 2B LE] 29 0F
    let packet_29_0f = vec![0xF4, 0x44, 0x02, 0x00, 0x29, 0x0F];
    let outcome = ts_dream::server::dispatcher::dispatch(&mut conn, &packet_29_0f, &data, &service, &env).await;
    assert!(outcome.outgoing.iter().any(|f| f.frame.starts_with("F444") && f.frame.contains("290F05")));

    // 2. Dispatch 0x29 0x09 (Notice)
    let packet_29_09 = vec![0xF4, 0x44, 0x04, 0x00, 0x29, 0x09, 0x01, 0x02];
    let outcome = ts_dream::server::dispatcher::dispatch(&mut conn, &packet_29_09, &data, &service, &env).await;
    assert!(outcome.outgoing.iter().any(|f| f.frame.starts_with("F444") && f.frame.contains("29090102")));

    lock_online_sessions().remove(&8301);
}

#[tokio::test]
async fn test_naval_combat_ammo_consumption_and_depletion() {
    let pool = setup_test_db().await;
    let data = GameData::default();
    let service = BattleService::default();

    let mut conn = Conn::default();
    conn.session.id = 8401;
    ts_dream::server::session::registry_register_for_login(&conn.session);

    // 1. Enter Water War -> provisions 10 ammo in slot 1 (id 46080)
    let init_frame = enter_water_war(&mut conn.session);
    assert_eq!(conn.session.boat_inventory.len(), 25);
    assert_eq!(conn.session.boat_inventory[0].id, 46080);
    assert_eq!(conn.session.boat_inventory[0].count, 10);
    assert!(init_frame.contains("176419"));

    // 2. Fire cannon 10 times -> ammo counts down to 0
    for i in 1..=10 {
        let mut out = HandleOutcome::default();
        // payload: action(1B) + target_id(4B LE) + param(1B)
        let mut payload = vec![0x01];
        payload.extend_from_slice(&8402u32.to_le_bytes());
        payload.push(1);

        let mut ctx = OpcodeCtx {
            conn: &mut conn,
            data: &data,
            service: &service,
            out: &mut out,
            opcode: 0x40,
            sub: 0x01,
            payload: &payload,
            decoded: &[],
            env: ServerEnv::with_pool(&pool),
        };
        ts_dream::server::handlers::guild::handle_naval_combat(&mut ctx).await;
        assert_eq!(conn.session.boat_inventory[0].count, 10 - i);
    }
    assert_eq!(conn.session.boat_inventory[0].id, 0);

    // 3. 11th shot should be rejected due to no ammo
    {
        let mut out = HandleOutcome::default();
        let mut payload = vec![0x01];
        payload.extend_from_slice(&8402u32.to_le_bytes());
        payload.push(1);

        let mut ctx = OpcodeCtx {
            conn: &mut conn,
            data: &data,
            service: &service,
            out: &mut out,
            opcode: 0x40,
            sub: 0x01,
            payload: &payload,
            decoded: &[],
            env: ServerEnv::with_pool(&pool),
        };
        ts_dream::server::handlers::guild::handle_naval_combat(&mut ctx).await;
        let expected = ts_dream::server::spawn::sys_msg_frame("Khong du dan duoc thuy chien!");
        assert!(out.outgoing.iter().any(|f| f.frame == expected));
    }

    lock_online_sessions().remove(&8401);
}
