//! Integration tests for Ticket 06: Competitive Arena Tournaments, Scoreboards,
//! and 7-Day Timed Title/Ring Revocation on Monday Reset.
//!
//! Verifies:
//! - Notice frame encoding (`S→C 25 03 [noticeCode: u8]`) for codes 0x01..0x0A across 5 modes
//! - Match status frame broadcasting (`S→C 14 28 [state: u8]`)
//! - Tournament scoreboard dynamic array encoding (`S→C 24 09`) and dialog open (`S→C 24 0A`)
//! - Weekly Monday 00:00 reset logic revoking 7-day timed championship items via `S→C 17 6F`
//! - Point exchange rejection when points < 50 via `S→C 17 6E 01`

use chrono::{TimeZone, Utc};
use ts_dream::data::loader::GameData;
use ts_dream::battle::service::BattleService;
use ts_dream::server::dispatcher::{HandleOutcome, OpcodeCtx, ServerEnv};
use ts_dream::server::handlers::arena::{
    build_match_status_raw, build_point_exchange_reject_raw, build_tournament_notice_raw,
    build_tournament_scoreboard_open_raw, build_tournament_scoreboard_raw, handle_arena,
    is_monday_reset_due, match_status_frame, point_exchange_reject_frame,
    process_weekly_arena_reset, timed_item_revoke_frame, tournament_notice_frame,
    tournament_scoreboard_frame, tournament_scoreboard_open_frame, ChampionTenure,
    ITEM_NHAN_DOAN_P, ITEM_NHAN_LOAN_DAU, MATCH_STATE_END, MATCH_STATE_START,
    NOTICE_CELESTIAL_PREPARE, NOTICE_CELESTIAL_START, NOTICE_MELEE_PREPARE, NOTICE_MELEE_START,
    NOTICE_SOLO_PK_PREPARE, NOTICE_SOLO_PK_START, NOTICE_SURVIVAL_PREPARE, NOTICE_SURVIVAL_START,
    NOTICE_TEAM_PK_PREPARE, NOTICE_TEAM_PK_START, POINT_EXCHANGE_REJECT_UNDER_50,
    REVOKE_NHAN_DOAN_P, REVOKE_NHAN_LOAN_DAU,
};
use ts_dream::server::session::{Conn, InventoryItem, Session};

#[test]
fn test_notice_frame_encoding_across_5_modes() {
    let notice_codes = [
        (NOTICE_SOLO_PK_PREPARE, 0x01),
        (NOTICE_SOLO_PK_START, 0x02),
        (NOTICE_TEAM_PK_PREPARE, 0x03),
        (NOTICE_TEAM_PK_START, 0x04),
        (NOTICE_MELEE_PREPARE, 0x05),
        (NOTICE_MELEE_START, 0x06),
        (NOTICE_CELESTIAL_PREPARE, 0x07),
        (NOTICE_CELESTIAL_START, 0x08),
        (NOTICE_SURVIVAL_PREPARE, 0x09),
        (NOTICE_SURVIVAL_START, 0x0A),
    ];

    for (code, expected_byte) in notice_codes {
        let raw = build_tournament_notice_raw(code);
        assert_eq!(raw[0..2], [0xF4, 0x44], "must have F4 44 header");
        assert_eq!(raw[2..4], [0x03, 0x00], "len must be 3");
        assert_eq!(raw[4], 0x25, "opcode must be 0x25");
        assert_eq!(raw[5], 0x03, "sub-opcode must be 0x03");
        assert_eq!(raw[6], expected_byte, "notice code must match");

        let hex_str = tournament_notice_frame(code);
        assert_eq!(hex_str, format!("F44403002503{:02X}", expected_byte));
    }
}

#[test]
fn test_match_status_broadcast_frame() {
    // 1 = start
    let raw_start = build_match_status_raw(MATCH_STATE_START);
    assert_eq!(raw_start, vec![0xF4, 0x44, 0x03, 0x00, 0x14, 0x28, 0x01]);
    assert_eq!(match_status_frame(MATCH_STATE_START), "F4440300142801");

    // 2 = end
    let raw_end = build_match_status_raw(MATCH_STATE_END);
    assert_eq!(raw_end, vec![0xF4, 0x44, 0x03, 0x00, 0x14, 0x28, 0x02]);
    assert_eq!(match_status_frame(MATCH_STATE_END), "F4440300142802");
}

#[test]
fn test_tournament_scoreboard_dynamic_array_payload() {
    // Mode = 1 (Solo PK), Flag = 0
    // 2 records, each exactly 7 bytes
    let record1: [u8; 7] = [0x01, 0x00, 0x10, 0x20, 0x30, 0x40, 0x50];
    let record2: [u8; 7] = [0x02, 0x01, 0x11, 0x21, 0x31, 0x41, 0x51];
    let records = [record1, record2];

    let raw = build_tournament_scoreboard_raw(1, 0, &records);
    // [0..2] F4 44
    // [2..4] len = 2 (opcode 24 09) + 1 (mode) + 1 (flag) + 2 (count) + 14 (records) = 20 = 0x14 0x00
    assert_eq!(raw[0..2], [0xF4, 0x44]);
    assert_eq!(raw[2..4], [0x14, 0x00]);
    assert_eq!(raw[4], 0x24);
    assert_eq!(raw[5], 0x09);
    assert_eq!(raw[6], 1, "mode");
    assert_eq!(raw[7], 0, "flag");
    assert_eq!(raw[8..10], [0x02, 0x00], "count = 2 records");
    assert_eq!(&raw[10..17], &record1);
    assert_eq!(&raw[17..24], &record2);

    let hex_str = tournament_scoreboard_frame(1, 0, &records);
    assert_eq!(hex_str.len(), 24 * 2);

    // Scoreboard open frame
    let open_raw = build_tournament_scoreboard_open_raw();
    assert_eq!(open_raw, vec![0xF4, 0x44, 0x02, 0x00, 0x24, 0x0A]);
    assert_eq!(tournament_scoreboard_open_frame(), "F4440200240A");
}

#[test]
fn test_point_exchange_under_50_rejection() {
    let raw = build_point_exchange_reject_raw(POINT_EXCHANGE_REJECT_UNDER_50);
    assert_eq!(raw, vec![0xF4, 0x44, 0x03, 0x00, 0x17, 0x6E, 0x01]);
    assert_eq!(point_exchange_reject_frame(POINT_EXCHANGE_REJECT_UNDER_50), "F4440300176E01");

    // Test through handle_arena opcode ctx
    let mut conn = Conn::default();
    conn.session.id = 5001;
    let data = GameData::default();
    let service = BattleService::default();
    let mut out = HandleOutcome::default();

    // Payload specifying 30 points (under 50)
    let payload = 30u32.to_le_bytes();
    let mut ctx = OpcodeCtx {
        conn: &mut conn,
        data: &data,
        service: &service,
        out: &mut out,
        opcode: 0x17,
        sub: 0x6E,
        payload: &payload,
        decoded: &[],
        env: ServerEnv::none(),
    };

    tokio::runtime::Runtime::new().unwrap().block_on(async {
        handle_arena(&mut ctx).await;
    });

    assert_eq!(out.outgoing.len(), 1);
    assert_eq!(out.outgoing[0].frame, "F4440300176E01");
}

#[test]
fn test_weekly_monday_reset_item_revocation() {
    let monday_00_05 = Utc.with_ymd_and_hms(2026, 11, 9, 0, 5, 0).unwrap();
    assert!(is_monday_reset_due(monday_00_05, None), "must trigger on Monday 00:xx");

    // Sunday test: should not trigger
    let sunday_23_59 = Utc.with_ymd_and_hms(2026, 11, 8, 23, 59, 0).unwrap();
    assert!(!is_monday_reset_due(sunday_23_59, None), "must not trigger on Sunday");

    let mut session = Session::default();
    session.id = 9001;
    // Player holds 3 championship rings/items:
    // 1. Nhẫn loạn đấu (slot 1) - awarded 8 days ago (expired)
    // 2. Nhẫn đoàn P (slot 2) - awarded 3 days ago, but ranking lost
    // 3. Adjacent item: 5 Đậu Dũng Khí (slot 3) - must remain intact!
    session.homdo.push(InventoryItem {
        slot: 1,
        id: ITEM_NHAN_LOAN_DAU,
        count: 1,
        ..Default::default()
    });
    session.homdo.push(InventoryItem {
        slot: 2,
        id: ITEM_NHAN_DOAN_P,
        count: 1,
        ..Default::default()
    });
    session.homdo.push(InventoryItem {
        slot: 3,
        id: 26051,
        count: 5,
        ..Default::default()
    });

    let records = vec![
        ChampionTenure {
            char_id: 9001,
            awarded_at: Utc.with_ymd_and_hms(2026, 11, 1, 0, 0, 0).unwrap(), // 8 days ago
            item_id: ITEM_NHAN_LOAN_DAU,
            still_ranked: true,
        },
        ChampionTenure {
            char_id: 9001,
            awarded_at: Utc.with_ymd_and_hms(2026, 11, 6, 0, 0, 0).unwrap(), // 3 days ago
            item_id: ITEM_NHAN_DOAN_P,
            still_ranked: false, // lost ranking!
        },
    ];

    let frames = process_weekly_arena_reset(&mut session, &records, monday_00_05);

    // Verify 2 revocation frames emitted
    assert_eq!(frames.len(), 2);
    // Nhẫn loạn đấu revocation code 0x02
    assert_eq!(frames[0], timed_item_revoke_frame(REVOKE_NHAN_LOAN_DAU));
    assert_eq!(frames[0], "F4440300176F02");

    // Nhẫn đoàn P revocation code 0x03
    assert_eq!(frames[1], timed_item_revoke_frame(REVOKE_NHAN_DOAN_P));
    assert_eq!(frames[1], "F4440300176F03");

    // Verify inventory integrity:
    // Slot 1 and slot 2 removed, adjacent slot 3 with count 5 intact!
    assert_eq!(session.homdo.len(), 1);
    assert_eq!(session.homdo[0].slot, 3);
    assert_eq!(session.homdo[0].id, 26051);
    assert_eq!(session.homdo[0].count, 5);
}

#[test]
fn test_weekly_monday_reset_equipped_trangbi_and_tuideo_revocation() {
    let monday_00_05 = Utc.with_ymd_and_hms(2026, 11, 9, 0, 5, 0).unwrap();

    let mut session = Session::default();
    session.id = 9002;

    // 1. Champion equipped ring in trangbi (slot 6 - accessory)
    session.trangbi.push(InventoryItem {
        slot: 6,
        id: ITEM_NHAN_LOAN_DAU,
        count: 1,
        ..Default::default()
    });

    // 2. Champion has another timed item in tuideo (auxiliary bag)
    session.tuideo.push(InventoryItem {
        slot: 1,
        id: ITEM_NHAN_DOAN_P,
        count: 1,
        ..Default::default()
    });

    let records = vec![
        ChampionTenure {
            char_id: 9002,
            awarded_at: Utc.with_ymd_and_hms(2026, 11, 1, 0, 0, 0).unwrap(), // expired 8 days
            item_id: ITEM_NHAN_LOAN_DAU,
            still_ranked: true,
        },
        ChampionTenure {
            char_id: 9002,
            awarded_at: Utc.with_ymd_and_hms(2026, 11, 8, 0, 0, 0).unwrap(), // 1 day ago but lost ranking
            item_id: ITEM_NHAN_DOAN_P,
            still_ranked: false,
        },
    ];

    let frames = process_weekly_arena_reset(&mut session, &records, monday_00_05);

    // Both items must be revoked!
    assert_eq!(frames.len(), 2);
    assert_eq!(frames[0], "F4440300176F02");
    assert_eq!(frames[1], "F4440300176F03");

    // Both trangbi and tuideo slots must be cleared
    assert_eq!(session.trangbi.len(), 0, "equipped ring in trangbi must be revoked");
    assert_eq!(session.tuideo.len(), 0, "ring in tuideo must be revoked");
}
