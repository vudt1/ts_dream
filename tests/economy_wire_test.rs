//! Comprehensive Economy Integration Tests (Tickets 01 - 07).
//!
//! - Strict compliance: In-memory SQLite (`sqlite::memory:?cache=shared`), never touches `DB/ts_dream.db`.
//! - No `#[cfg(test)]` in `src/`.
//! - Tests wire packet byte structures against `aLogin.exe` protocol specifications:
//!   1. Currency model & post-login bank sync (`1A 04`, `1D 04`, shop_point preservation, GOLD_CAP clamp).
//!   2. NPC shop buy flow (`1B 01`) & error codes (`01`, `02`, `04`).
//!   3. NPC shop starter bundle fallback for NPC 7 Map 9999.
//!   4. NPC shop sell flow (`1B 02`) & error codes (`02`, `03`, `06`).
//!   5. Bank deposit (`1D 02`) & withdrawal (`1D 01`) with atomic DB updates and `1D 03` error frame.
//!   6. Storage transfer count fix: verifies actual count (hex `{:02X}`) in `1E 05`, `17 09`, `17 68`, NOT `"32"`.
//!   7. Player stall catalog frame (17B header + 16B records) & atomic shop purchase.
//!   8. P2P item trade (`19 01`) vs pet trade (`19 0A`) separation and `17 3B` form close on cancellation.

use ts_dream::battle::service::BattleService;
use ts_dream::data::loader::GameData;
use ts_dream::data::loaders::item::ItemDef;
use ts_dream::db::modern::sqlite::SqliteRepositories;
use ts_dream::db::pool::{bootstrap, DbPool};
use ts_dream::protocol::{
    encoder, GOLD_CAP, NPC_SHOP_BUY_BAG_FULL, NPC_SHOP_BUY_DISRUPTED,
    NPC_SHOP_BUY_INSUFFICIENT_GOLD, NPC_SHOP_SELL_GOLD_FULL_ALT, NPC_SHOP_SELL_ITEM_NOT_FOUND,
    NPC_SHOP_SELL_QTY_NOT_ENOUGH,
};
use ts_dream::server::dispatcher::{HandleOutcome, OpcodeCtx, ServerEnv};
use ts_dream::server::handlers::shops::{
    complete_shop_buy, gold_frame, handle_npc_shop, handle_player_shop, player_shop_catalog_frame,
    point_frame, ShopBuyError,
};
use ts_dream::server::handlers::talk::handle_talk;
use ts_dream::server::handlers::trade_storage::{
    cancel_timed_out_trade, expire_trade_if_timed_out, handle_bank_gold, handle_storage_transfer,
    handle_trade, is_trade_timed_out, send_bank_ui_unlock, TRADE_TIMEOUT_MS,
};
use ts_dream::server::session::{
    lock_online_sessions, Conn, InventoryItem, PetState, Session, ShopItem,
};
use ts_dream::server::spawn::build_logined_sequence_session;

async fn setup_test_db() -> DbPool {
    let pool = bootstrap("sqlite::memory:?cache=shared", None)
        .await
        .expect("bootstrap in-memory SQLite");
    let schema = include_str!("../migrations/0001_init.sql");
    sqlx::raw_sql(schema)
        .execute(&pool.write)
        .await
        .expect("execute 0001_init.sql migration");
    pool
}

fn create_test_conn(player_id: u32, db_char_id: i64) -> Conn {
    let mut conn = Conn::default();
    conn.session.id = player_id;
    conn.session.db_character_id = db_char_id;
    conn.session.idtalking = 1;
    conn.session.map_id = 12201;
    conn
}

// =========================================================================
// Test 1: Currency Wire Sync Login (Ticket 01, 07)
// =========================================================================
#[test]
fn test_currency_wire_sync_login() {
    let mut session = Session::default();
    session.id = 1001;
    session.gold = 500_000;
    session.bank_gold = 1_200_000;
    session.shop_point = 88_888;

    let frames = build_logined_sequence_session(&session);

    // 1. Verify 1A 04 frame contains gold (6 bytes) and 1A 03 frame contains shop_point (6 bytes)
    let expected_gold_frame = format!("F44406001A04{}", encoder::le32(500_000));
    assert!(
        frames.iter().any(|f| f == &expected_gold_frame),
        "spawn frames must contain 1A 04 with gold (6 bytes): expected {}",
        expected_gold_frame
    );

    let expected_point_frame = format!("F44406001A03{}", encoder::le32(88_888));
    assert!(
        frames.iter().any(|f| f == &expected_point_frame),
        "spawn frames must contain 1A 03 with shop_point (6 bytes): expected {}",
        expected_point_frame
    );

    // 2. Verify 1D 04 frame contains bank_gold
    let expected_bank_frame = format!("F44406001D04{}", encoder::le32(1_200_000));
    assert!(
        frames.iter().any(|f| f == &expected_bank_frame),
        "spawn frames must contain 1D 04 bank gold sync frame: expected {}",
        expected_bank_frame
    );

    // 3. Test GOLD_CAP clamp on gold and bank_gold
    let mut capped_session = Session::default();
    capped_session.id = 1002;
    capped_session.gold = 15_000_000; // exceeds GOLD_CAP 9_999_999
    capped_session.bank_gold = 25_000_000; // exceeds GOLD_CAP 9_999_999
    capped_session.shop_point = 1_500_000_000; // shop points up to 2B

    let capped_frames = build_logined_sequence_session(&capped_session);

    let expected_capped_gold = format!("F44406001A04{}", encoder::le32(GOLD_CAP));
    assert!(
        capped_frames.iter().any(|f| f == &expected_capped_gold),
        "gold must be clamped to GOLD_CAP: expected {}",
        expected_capped_gold
    );

    let expected_capped_point = format!("F44406001A03{}", encoder::le32(1_500_000_000));
    assert!(
        capped_frames.iter().any(|f| f == &expected_capped_point),
        "shop point sync frame must contain 1_500_000_000: expected {}",
        expected_capped_point
    );

    let expected_capped_bank = format!("F44406001D04{}", encoder::le32(GOLD_CAP));
    assert!(
        capped_frames.iter().any(|f| f == &expected_capped_bank),
        "bank_gold must be clamped to GOLD_CAP: expected {}",
        expected_capped_bank
    );

    // 4. Test gold_frame helper
    let gf = gold_frame(capped_session.gold);
    assert_eq!(gf, expected_capped_gold);
    let pf = point_frame(capped_session.shop_point);
    assert_eq!(pf, expected_capped_point);

    // 5. Verify unconditional 1D 04 sync frame when bank_gold == 0 (Ticket 02 / B4 fix)
    let mut zero_bank_session = Session::default();
    zero_bank_session.id = 1003;
    zero_bank_session.bank_gold = 0;
    let zero_frames = build_logined_sequence_session(&zero_bank_session);
    assert!(
        zero_frames.iter().any(|f| f == "F44406001D0400000000"),
        "1D 04 bank sync frame must be sent unconditionally with 0 when bank_gold == 0: expected F44406001D0400000000"
    );
}

// =========================================================================
// Test 2: NPC Shop Buy Flow & Result Codes (Ticket 01)
// =========================================================================
#[tokio::test]
async fn test_npc_shop_buy_flow_and_errors() {
    let data = GameData::default();
    let service = BattleService::default();
    let env = ServerEnv::none();

    // Case A: Successful purchase
    // Map 1, NPC 12201, index 0 resolves to item 10001 (price 10 gold).
    let mut conn = create_test_conn(2001, 2001);
    conn.session.gold = 1_000;
    conn.session.shop_point = 50;

    let payload = vec![0x00, 0x03, 0x00]; // index 0, count 3 (cost = 30 gold)
    let mut out = HandleOutcome::default();
    {
        let mut ctx = OpcodeCtx {
            conn: &mut conn,
            data: &data,
            service: &service,
            out: &mut out,
            opcode: 0x1B,
            sub: 1, // Buy
            payload: &payload,
            decoded: &[],
            env,
        };
        handle_npc_shop(&mut ctx).await;
    }

    // Verify wire sequence: 1A 02 (deduct 30), 1A 04 (balance), 17 06 (add item), 1B 01 00 (silent success)
    let frames: Vec<String> = out.outgoing.into_iter().map(|f| f.frame).collect();
    assert_eq!(conn.session.gold, 970);
    assert_eq!(conn.session.homdo.len(), 1);
    assert_eq!(conn.session.homdo[0].id, 10001);
    assert_eq!(conn.session.homdo[0].count, 3);

    let expected_deduct = format!("F44406001A02{}", encoder::le32(30));
    assert!(
        frames.iter().any(|f| f == &expected_deduct),
        "Must send 1A 02 to deduct gold"
    );

    let expected_sync = format!("F44406001A04{}", encoder::le32(970));
    assert!(
        frames.iter().any(|f| f == &expected_sync),
        "Must send 1A 04 to sync balance"
    );

    assert!(
        frames.iter().any(|f| f.starts_with("F4440E001706")),
        "Must send 17 06 to add item"
    );

    assert!(
        frames.iter().any(|f| f == "F44403001B0100"),
        "Must send 1B 01 00 on success"
    );

    // Case B: Insufficient gold
    let mut conn_poor = create_test_conn(2002, 2002);
    conn_poor.session.gold = 5; // cannot afford 30 gold
    let mut out_poor = HandleOutcome::default();
    {
        let mut ctx = OpcodeCtx {
            conn: &mut conn_poor,
            data: &data,
            service: &service,
            out: &mut out_poor,
            opcode: 0x1B,
            sub: 1,
            payload: &payload,
            decoded: &[],
            env,
        };
        handle_npc_shop(&mut ctx).await;
    }
    let frames_poor: Vec<String> = out_poor.outgoing.into_iter().map(|f| f.frame).collect();
    assert_eq!(conn_poor.session.gold, 5);
    assert!(conn_poor.session.homdo.is_empty());
    let expected_poor_err = format!("F44403001B01{:02X}", NPC_SHOP_BUY_INSUFFICIENT_GOLD);
    assert!(
        frames_poor.iter().any(|f| f == &expected_poor_err),
        "Must return 1B 01 01 on insufficient gold"
    );

    // Case C: Bag full (all 25 slots occupied)
    let mut conn_full = create_test_conn(2003, 2003);
    conn_full.session.gold = 10_000;
    for slot in 1..=25 {
        conn_full.session.homdo.push(InventoryItem {
            slot,
            id: 20000 + slot as u16,
            count: 1,
            ..Default::default()
        });
    }
    let mut out_full = HandleOutcome::default();
    {
        let mut ctx = OpcodeCtx {
            conn: &mut conn_full,
            data: &data,
            service: &service,
            out: &mut out_full,
            opcode: 0x1B,
            sub: 1,
            payload: &payload,
            decoded: &[],
            env,
        };
        handle_npc_shop(&mut ctx).await;
    }
    let frames_full: Vec<String> = out_full.outgoing.into_iter().map(|f| f.frame).collect();
    let expected_full_err = format!("F44403001B01{:02X}", NPC_SHOP_BUY_BAG_FULL);
    assert!(
        frames_full.iter().any(|f| f == &expected_full_err),
        "Must return 1B 01 02 on full bag"
    );

    // Case D: Invalid item index (unresolved)
    let mut conn_inv = create_test_conn(2004, 2004);
    conn_inv.session.gold = 10_000;
    let inv_payload = vec![0xFF, 0x01, 0x00]; // index 255 does not exist
    let mut out_inv = HandleOutcome::default();
    {
        let mut ctx = OpcodeCtx {
            conn: &mut conn_inv,
            data: &data,
            service: &service,
            out: &mut out_inv,
            opcode: 0x1B,
            sub: 1,
            payload: &inv_payload,
            decoded: &[],
            env,
        };
        handle_npc_shop(&mut ctx).await;
    }
    let frames_inv: Vec<String> = out_inv.outgoing.into_iter().map(|f| f.frame).collect();
    let expected_inv_err = format!("F44403001B01{:02X}", NPC_SHOP_BUY_DISRUPTED);
    assert!(
        frames_inv.iter().any(|f| f == &expected_inv_err),
        "Must return 1B 01 04 on unresolved item"
    );

    // Case E: Buy count == 0 (Ticket 01 / C2 fix) -> rejected with 1B 01 04 (not forced to 1)
    let mut conn_zero = create_test_conn(2005, 2005);
    conn_zero.session.gold = 10_000;
    let zero_payload = vec![0x00, 0x00, 0x00]; // index 0, count 0
    let mut out_zero = HandleOutcome::default();
    {
        let mut ctx = OpcodeCtx {
            conn: &mut conn_zero,
            data: &data,
            service: &service,
            out: &mut out_zero,
            opcode: 0x1B,
            sub: 1,
            payload: &zero_payload,
            decoded: &[],
            env,
        };
        handle_npc_shop(&mut ctx).await;
    }
    let frames_zero: Vec<String> = out_zero.outgoing.into_iter().map(|f| f.frame).collect();
    assert!(
        frames_zero.iter().any(|f| f == "F44403001B0104"),
        "Must return literal F44403001B0104 when count == 0"
    );
    assert_eq!(conn_zero.session.gold, 10_000, "Gold must remain untouched when count == 0");
    assert!(conn_zero.session.homdo.is_empty(), "No item granted when count == 0");
}

// =========================================================================
// Test 3: NPC 7 Map 9999 Starter Bundle Fallback (Ticket 01 constraint)
// =========================================================================
#[tokio::test]
async fn test_npc_shop_starter_bundle_npc7_map9999() {
    let data = GameData::default();
    let service = BattleService::default();
    let env = ServerEnv::none();

    let mut conn = create_test_conn(2005, 2005);
    conn.session.map_id = 9999;
    conn.session.idtalking = 7;
    conn.session.gold = 0;

    let payload = vec![0x00, 0x01, 0x00];
    let mut out = HandleOutcome::default();
    {
        let mut ctx = OpcodeCtx {
            conn: &mut conn,
            data: &data,
            service: &service,
            out: &mut out,
            opcode: 0x1B,
            sub: 1,
            payload: &payload,
            decoded: &[],
            env,
        };
        handle_npc_shop(&mut ctx).await;
    }

    let frames: Vec<String> = out.outgoing.into_iter().map(|f| f.frame).collect();
    // Verify starter items were added to homdo
    assert!(
        !conn.session.homdo.is_empty(),
        "Starter bundle must populate items into homdo"
    );
    // Verify 1B 01 00 was sent
    assert!(
        frames.iter().any(|f| f == "F44403001B0100"),
        "Must send 1B 01 00 on starter bundle grant"
    );
}

// =========================================================================
// Test 4: NPC Shop Sell Flow & Result Codes (Ticket 01)
// =========================================================================
#[tokio::test]
async fn test_npc_shop_sell_flow_and_errors() {
    let mut data = GameData::default();
    // Register item 10001 with sell_price = 25
    data.item_defs.insert(
        10001,
        ItemDef {
            id: 10001,
            sell_price: 25,
            price: 50,
            ..Default::default()
        },
    );

    let service = BattleService::default();
    let env = ServerEnv::none();

    // Case A: Successful sell of 2 out of 5 items
    let mut conn = create_test_conn(2006, 2006);
    conn.session.gold = 100;
    conn.session.shop_point = 10;
    conn.session.homdo.push(InventoryItem {
        slot: 1,
        id: 10001,
        count: 5,
        ..Default::default()
    });

    let payload = vec![0x01, 0x02]; // slot 1, qty 2 (revenue = 2 * 25 = 50)
    let mut out = HandleOutcome::default();
    {
        let mut ctx = OpcodeCtx {
            conn: &mut conn,
            data: &data,
            service: &service,
            out: &mut out,
            opcode: 0x1B,
            sub: 2, // Sell
            payload: &payload,
            decoded: &[],
            env,
        };
        handle_npc_shop(&mut ctx).await;
    }

    let frames: Vec<String> = out.outgoing.into_iter().map(|f| f.frame).collect();
    assert_eq!(conn.session.gold, 150);
    assert_eq!(conn.session.homdo[0].count, 3); // 5 - 2 = 3

    let expected_credit = format!("F44406001A01{}", encoder::le32(50));
    assert!(
        frames.iter().any(|f| f == &expected_credit),
        "Must send 1A 01 to credit gold"
    );

    let expected_sync = format!("F44406001A04{}", encoder::le32(150));
    assert!(
        frames.iter().any(|f| f == &expected_sync),
        "Must send 1A 04 to sync balance"
    );

    let expected_remove = "F444040017090102"; // 17 09 slot 1 qty 2
    assert!(
        frames.iter().any(|f| f == expected_remove),
        "Must send 17 09 01 02 to remove item from slot"
    );

    assert!(
        frames.iter().any(|f| f == "F44403001B0200"),
        "Must send 1B 02 00 on sell success"
    );

    // Case B: Item not found (empty slot)
    let empty_payload = vec![0x05, 0x01]; // slot 5 does not exist
    let mut out_empty = HandleOutcome::default();
    {
        let mut ctx = OpcodeCtx {
            conn: &mut conn,
            data: &data,
            service: &service,
            out: &mut out_empty,
            opcode: 0x1B,
            sub: 2,
            payload: &empty_payload,
            decoded: &[],
            env,
        };
        handle_npc_shop(&mut ctx).await;
    }
    let frames_empty: Vec<String> = out_empty.outgoing.into_iter().map(|f| f.frame).collect();
    let expected_not_found = format!("F44403001B02{:02X}", NPC_SHOP_SELL_ITEM_NOT_FOUND);
    assert!(
        frames_empty.iter().any(|f| f == &expected_not_found),
        "Must return 1B 02 02 when item not found"
    );

    // Case C: Qty not enough (trying to sell 10 when count is 3)
    let over_payload = vec![0x01, 0x0A];
    let mut out_over = HandleOutcome::default();
    {
        let mut ctx = OpcodeCtx {
            conn: &mut conn,
            data: &data,
            service: &service,
            out: &mut out_over,
            opcode: 0x1B,
            sub: 2,
            payload: &over_payload,
            decoded: &[],
            env,
        };
        handle_npc_shop(&mut ctx).await;
    }
    let frames_over: Vec<String> = out_over.outgoing.into_iter().map(|f| f.frame).collect();
    let expected_qty_err = format!("F44403001B02{:02X}", NPC_SHOP_SELL_QTY_NOT_ENOUGH);
    assert!(
        frames_over.iter().any(|f| f == &expected_qty_err),
        "Must return 1B 02 03 when qty requested exceeds slot count"
    );

    // Case D: Gold cap reached (revenue would exceed GOLD_CAP 9_999_999)
    conn.session.gold = GOLD_CAP - 10;
    let cap_payload = vec![0x01, 0x01]; // slot 1 count 1 -> revenue 25, total = GOLD_CAP + 15
    let mut out_cap = HandleOutcome::default();
    {
        let mut ctx = OpcodeCtx {
            conn: &mut conn,
            data: &data,
            service: &service,
            out: &mut out_cap,
            opcode: 0x1B,
            sub: 2,
            payload: &cap_payload,
            decoded: &[],
            env,
        };
        handle_npc_shop(&mut ctx).await;
    }
    let frames_cap: Vec<String> = out_cap.outgoing.into_iter().map(|f| f.frame).collect();
    let expected_cap_err = format!("F44403001B02{:02X}", NPC_SHOP_SELL_GOLD_FULL_ALT);
    assert!(
        frames_cap.iter().any(|f| f == &expected_cap_err),
        "Must return 1B 02 06 when player gold would exceed GOLD_CAP"
    );
}

// =========================================================================
// Test 2: Bank Deposit & Withdrawal With UI Unlock (Ticket 01, 07)
// =========================================================================
#[tokio::test]
async fn test_bank_deposit_withdraw_with_ui_unlock() {
    let pool = setup_test_db().await;
    let repos = SqliteRepositories::new(pool.clone());
    let data = GameData::default();
    let service = BattleService::default();
    let env = ServerEnv {
        pool: Some(&pool),
        repos: Some(&repos),
        hub: None,
        sender: None,
    };

    // Create an account & character in in-memory DB
    let acc_id = ts_dream::db::accounts::create(&pool, "pass1", "pass2")
        .await
        .expect("create account");
    let char_id = acc_id;
    sqlx::query(
        "INSERT INTO characters (playerid, name) VALUES (?, ?)"
    )
    .bind(char_id)
    .bind(b"TestBankChar".to_vec())
    .execute(&pool.write)
    .await
    .expect("insert character");

    sqlx::query(
        "INSERT INTO character_money (playerid, gold, bankgold, shoppoint) VALUES (?, ?, ?, ?)"
    )
    .bind(char_id)
    .bind(50_000i64)
    .bind(100_000i64)
    .bind(0i64)
    .execute(&pool.write)
    .await
    .expect("insert money");

    let mut conn = create_test_conn(char_id as u32, char_id);
    conn.session.gold = 50_000;
    conn.session.bank_gold = 100_000;

    // 1. Bank Deposit: 1D 02 <20_000: u32 LE>
    let mut deposit_payload = Vec::new();
    deposit_payload.extend_from_slice(&20_000u32.to_le_bytes());
    let mut out_dep = HandleOutcome::default();
    {
        let mut ctx = OpcodeCtx {
            conn: &mut conn,
            data: &data,
            service: &service,
            out: &mut out_dep,
            opcode: 0x1D,
            sub: ts_dream::protocol::BANK_SUB_DEPOSIT,
            payload: &deposit_payload,
            decoded: &[],
            env,
        };
        handle_bank_gold(&mut ctx).await;
    }

    assert_eq!(conn.session.gold, 30_000);
    assert_eq!(conn.session.bank_gold, 120_000);

    let dep_frames: Vec<String> = out_dep.outgoing.into_iter().map(|f| f.frame).collect();
    let expected_dep_bank = format!("F44406001D01{}", encoder::le32(20_000));
    let expected_dep_gold = format!("F44406001A02{}", encoder::le32(20_000));
    assert!(
        dep_frames.iter().any(|f| f == &expected_dep_bank),
        "Deposit must reply 1D 01 to credit bank"
    );
    assert!(
        dep_frames.iter().any(|f| f == &expected_dep_gold),
        "Deposit must reply 1A 02 to debit pocket gold"
    );
    assert!(
        dep_frames.iter().any(|f| f == "F44402001D07"),
        "Deposit must reply 1D 07 to unlock UI deposit button"
    );

    // Verify DB persistence
    let row: (i64, i64) = sqlx::query_as("SELECT gold, bankgold FROM character_money WHERE playerid = ?")
        .bind(char_id)
        .fetch_one(&pool.read)
        .await
        .expect("query character_money");
    assert_eq!(row.0, 30_000);
    assert_eq!(row.1, 120_000);

    // 2. Bank Withdrawal: 1D 01 <50_000: u32 LE>
    let mut withdraw_payload = Vec::new();
    withdraw_payload.extend_from_slice(&50_000u32.to_le_bytes());
    let mut out_wdr = HandleOutcome::default();
    {
        let mut ctx = OpcodeCtx {
            conn: &mut conn,
            data: &data,
            service: &service,
            out: &mut out_wdr,
            opcode: 0x1D,
            sub: ts_dream::protocol::BANK_SUB_WITHDRAW,
            payload: &withdraw_payload,
            decoded: &[],
            env,
        };
        handle_bank_gold(&mut ctx).await;
    }

    assert_eq!(conn.session.gold, 80_000);
    assert_eq!(conn.session.bank_gold, 70_000);

    let wdr_frames: Vec<String> = out_wdr.outgoing.into_iter().map(|f| f.frame).collect();
    let expected_wdr_bank = format!("F44406001D02{}", encoder::le32(50_000));
    let expected_wdr_gold = format!("F44406001A01{}", encoder::le32(50_000));
    assert!(
        wdr_frames.iter().any(|f| f == &expected_wdr_bank),
        "Withdrawal must reply 1D 02 to debit bank"
    );
    assert!(
        wdr_frames.iter().any(|f| f == &expected_wdr_gold),
        "Withdrawal must reply 1A 01 to credit pocket gold"
    );
    assert!(
        wdr_frames.iter().any(|f| f == "F44402001D08"),
        "Withdrawal must reply 1D 08 to unlock UI withdraw button"
    );

    // 3. Over-withdrawal: try withdrawing 100_000 when bank only has 70_000
    let mut over_wdr_payload = Vec::new();
    over_wdr_payload.extend_from_slice(&100_000u32.to_le_bytes());
    let mut out_over = HandleOutcome::default();
    {
        let mut ctx = OpcodeCtx {
            conn: &mut conn,
            data: &data,
            service: &service,
            out: &mut out_over,
            opcode: 0x1D,
            sub: 1,
            payload: &over_wdr_payload,
            decoded: &[],
            env,
        };
        handle_bank_gold(&mut ctx).await;
    }

    assert_eq!(conn.session.gold, 80_000);
    assert_eq!(conn.session.bank_gold, 70_000);
    let over_frames: Vec<String> = out_over.outgoing.into_iter().map(|f| f.frame).collect();
    assert!(
        over_frames.iter().any(|f| f == "F44402001D03"),
        "Must reply 1D 03 on insufficient bank balance"
    );
    assert!(
        over_frames.iter().any(|f| f == "F44402001D08"),
        "Must reply 1D 08 to unlock UI withdraw button even on failure"
    );

    // 4. Over-deposit exceeding GOLD_CAP
    let mut over_dep_payload = Vec::new();
    over_dep_payload.extend_from_slice(&(GOLD_CAP - 50_000).to_le_bytes());
    let mut out_over_dep = HandleOutcome::default();
    {
        let mut ctx = OpcodeCtx {
            conn: &mut conn,
            data: &data,
            service: &service,
            out: &mut out_over_dep,
            opcode: 0x1D,
            sub: 2,
            payload: &over_dep_payload,
            decoded: &[],
            env,
        };
        handle_bank_gold(&mut ctx).await;
    }
    let over_dep_frames: Vec<String> = out_over_dep.outgoing.into_iter().map(|f| f.frame).collect();
    assert!(
        over_dep_frames.iter().any(|f| f == "F44402001D03"),
        "Must reply 1D 03 on deposit exceeding GOLD_CAP"
    );
    assert!(
        over_dep_frames.iter().any(|f| f == "F44402001D07"),
        "Must reply 1D 07 to unlock UI deposit button even on failure"
    );
}

// =========================================================================
// Test 6: Storage Transfer Wire Formatting (Count Fix, Ticket 04)
// =========================================================================
#[tokio::test]
async fn test_storage_warehouse_item_transfers_count_fix() {
    let data = GameData::default();
    let service = BattleService::default();
    let env = ServerEnv::none();

    let mut conn = create_test_conn(4001, 4001);
    // Put item in bag (slot 2) with count 15 (hex 0x0F)
    conn.session.homdo.push(InventoryItem {
        slot: 2,
        id: 26001,
        count: 15,
        doben: 100,
        ..Default::default()
    });

    // 1. Deposit to Tiền Trang: 1E 02 <slot 2>
    let mut out_dep = HandleOutcome::default();
    {
        let mut ctx = OpcodeCtx {
            conn: &mut conn,
            data: &data,
            service: &service,
            out: &mut out_dep,
            opcode: 0x1E,
            sub: 2,
            payload: &[0x02],
            decoded: &[],
            env,
        };
        handle_storage_transfer(&mut ctx).await;
    }

    let dep_frames: Vec<String> = out_dep.outgoing.into_iter().map(|f| f.frame).collect();
    // Must be 17 09 02 0F, NOT 17 09 02 32!
    assert!(
        dep_frames.iter().any(|f| f == "F44404001709020F"),
        "17 09 remove frame must carry exact item count 15 (0x0F), NOT hardcoded 32: found {:?}",
        dep_frames
    );
    assert!(
        dep_frames.iter().any(|f| f == "F4440D001E0291650F6400640000000000"),
        "Must send 1E 02 (13B frame F4440D001E02...) to add item to bank storage: found {:?}",
        dep_frames
    );
    assert!(conn.session.homdo.is_empty());
    assert_eq!(conn.session.tientrang.len(), 1);
    assert_eq!(conn.session.tientrang[0].count, 15);

    // 2. Withdraw from Tiền Trang: 1E 01 <slot 1>
    let tientrang_slot = conn.session.tientrang[0].slot;
    let tientrang_payload = [tientrang_slot];
    let mut out_wdr = HandleOutcome::default();
    {
        let mut ctx = OpcodeCtx {
            conn: &mut conn,
            data: &data,
            service: &service,
            out: &mut out_wdr,
            opcode: 0x1E,
            sub: 1,
            payload: &tientrang_payload,
            decoded: &[],
            env,
        };
        handle_storage_transfer(&mut ctx).await;
    }

    let wdr_frames: Vec<String> = out_wdr.outgoing.into_iter().map(|f| f.frame).collect();
    // Must be 1E 05 01 0F (<slot 1> <qty 15>)
    assert!(
        wdr_frames.iter().any(|f| f == "F44404001E05010F"),
        "1E 05 remove frame must carry exact slot 1 and item count 15 (0x0F): found {:?}",
        wdr_frames
    );
    assert!(
        wdr_frames.iter().any(|f| f.starts_with("F4440E001708")),
        "Must send 17 08 to add item to bag"
    );
    assert!(
        wdr_frames.iter().any(|f| f == "F44402001732"),
        "Must send 17 32 to refresh bag"
    );
    assert_eq!(conn.session.homdo.len(), 1);
    assert!(conn.session.tientrang.is_empty());

    // 3. Luulang Transfer with special companion pet (41187)
    conn.session.pets.push(PetState {
        id: 41187,
        stt: 1,
        ..Default::default()
    });

    // Move from homdo slot 1 to luulang: 17 51 <slot 1>
    conn.session.homdo[0].count = 42; // hex 0x2A
    let homdo_slot = conn.session.homdo[0].slot;
    let homdo_payload = [homdo_slot];
    let mut out_ll_dep = HandleOutcome::default();
    {
        let mut ctx = OpcodeCtx {
            conn: &mut conn,
            data: &data,
            service: &service,
            out: &mut out_ll_dep,
            opcode: 0x17,
            sub: 51,
            payload: &homdo_payload,
            decoded: &[],
            env,
        };
        handle_storage_transfer(&mut ctx).await;
    }

    let ll_dep_frames: Vec<String> = out_ll_dep.outgoing.into_iter().map(|f| f.frame).collect();
    assert!(
        ll_dep_frames.iter().any(|f| f == "F44404001709012A"),
        "17 09 frame for Luulang must carry exact item count 42 (0x2A), NOT hardcoded 32"
    );
    assert!(
        ll_dep_frames.iter().any(|f| f == "F4440E0017670191652A6400640000000000"),
        "17 67 update frame for Luulang must be 14-byte frame F4440E001767...: found {:?}",
        ll_dep_frames
    );
    assert_eq!(conn.session.luulang.len(), 1);
    assert_eq!(conn.session.luulang[0].count, 42);

    // Withdraw from luulang to homdo: 17 52 <slot 1>
    let luulang_slot = conn.session.luulang[0].slot;
    let luulang_payload = [luulang_slot];
    let mut out_ll_wdr = HandleOutcome::default();
    {
        let mut ctx = OpcodeCtx {
            conn: &mut conn,
            data: &data,
            service: &service,
            out: &mut out_ll_wdr,
            opcode: 0x17,
            sub: 52,
            payload: &luulang_payload,
            decoded: &[],
            env,
        };
        handle_storage_transfer(&mut ctx).await;
    }

    let ll_wdr_frames: Vec<String> = out_ll_wdr.outgoing.into_iter().map(|f| f.frame).collect();
    assert!(
        ll_wdr_frames.iter().any(|f| f == "F4440300176801"),
        "17 68 frame for Luulang must be 3-byte F44403001768<slot>: found {:?}",
        ll_wdr_frames
    );
    assert_eq!(conn.session.homdo.len(), 1);
    assert!(conn.session.luulang.is_empty());
}

// =========================================================================
// Test 7: Player Stall Catalog & Purchase (Ticket 05)
// =========================================================================
#[test]
fn test_player_stall_catalog_and_complete_buy() {
    let mut seller = Session::default();
    seller.id = 5001;
    seller.gold = 10_000;
    seller.homdo.push(InventoryItem {
        slot: 1,
        id: 10001,
        count: 5,
        long_val: 1,
        giatri_long: 5,
        khang: 0,
        texp: 100,
        ..Default::default()
    });
    seller.homdo.push(InventoryItem {
        slot: 2,
        id: 10002,
        count: 10,
        long_val: 2,
        giatri_long: 10,
        khang: 1,
        texp: 200,
        ..Default::default()
    });
    seller.shop.active = true;
    seller.shop.items = vec![
        ShopItem {
            slot: 1,
            item_id: 10001,
            count: 5,
            price: 500,
        },
        ShopItem {
            slot: 2,
            item_id: 10002,
            count: 10,
            price: 1_200,
        },
    ];

    // 1. Verify Catalog frame wire structure (17B header + 16B records)
    let catalog = player_shop_catalog_frame(&seller);
    // Header is 17 bytes: 17 21 (2B) + seller_id 5001 (4B LE: "89130000") + 12 zero bytes ("00" * 12)
    let expected_header = "172189130000000000000000000000000000";
    assert!(
        catalog.contains(expected_header),
        "Catalog frame must contain exact 17-byte header with seller id and 12 zeros"
    );

    // Record 1 (slot 01, item 10001, count 5, long 1, price 500, biased giatri 5+100=105 [0x69], khang 0, pad 0, texp 100)
    let expected_record1 = "0111270501F401000069000064000000";
    assert!(
        catalog.contains(expected_record1),
        "Catalog frame must contain literal record 1 with biased giatri 0x69"
    );

    // Record 2 (slot 02, item 10002, count 10, long 2, price 1200, biased giatri 10+100=110 [0x6E], khang 1, pad 0, texp 200)
    let expected_record2 = "0212270A02B00400006E0100C8000000";
    assert!(
        catalog.contains(expected_record2),
        "Catalog frame must contain literal record 2 with biased giatri 0x6E"
    );

    // 2. Complete Shop Buy (Ticket 05 / A1 & E2 fix)
    let mut buyer = Session::default();
    buyer.id = 5002;
    buyer.gold = 5_000;
    buyer.shop_point = 100;

    // Slot 0 is invalid (1-based indexing required by client UI echo)
    assert_eq!(
        complete_shop_buy(&mut buyer, &mut seller, 0, 2),
        Err(ShopBuyError::InvalidIndex),
        "Slot 0 must be rejected under 1-based indexing"
    );

    // Slot 1 purchases 2 of 1st item (10001), NOT 2nd item (10002) - Critical Bug A1 Fix!
    let res = complete_shop_buy(&mut buyer, &mut seller, 1, 2).expect("buy success");
    assert_eq!(res.total, 1_000);
    assert_eq!(res.count, 2);
    assert_eq!(res.item.count, 2, "Returned purchase item count must equal moved count (2), not original seller stack");
    assert_eq!(buyer.gold, 4_000);
    assert_eq!(seller.gold, 11_000);
    assert_eq!(seller.shop.items[0].count, 3); // 5 - 2 = 3
    assert_eq!(buyer.homdo.len(), 1);
    assert_eq!(buyer.homdo[0].id, 10001, "Slot 1 must buy item 10001, not 10002");
    assert_eq!(buyer.homdo[0].count, 2);

    // 2b. Purchase remaining 3 of slot 1 -> listing must be completely removed from seller.shop.items
    let res_remaining = complete_shop_buy(&mut buyer, &mut seller, 1, 3).expect("buy remaining success");
    assert_eq!(res_remaining.count, 3);
    assert_eq!(res_remaining.item.count, 3);
    assert_eq!(seller.shop.items.len(), 1, "Exhausted listing must be removed from shop.items");
    assert_eq!(seller.shop.items[0].item_id, 10002, "Only item 10002 should remain");

    // 3. Buyer has insufficient gold for slot 1 (now 10002) (cost = 5 * 1200 = 6000 > 2500)
    let poor_result = complete_shop_buy(&mut buyer, &mut seller, 1, 5);
    assert_eq!(poor_result, Err(ShopBuyError::NotEnoughGold { total: 6_000 }));

    // 4. Seller gold would exceed GOLD_CAP -> rejected with SellerGoldFull
    seller.gold = GOLD_CAP - 100;
    buyer.gold = 10_000;
    let full_result = complete_shop_buy(&mut buyer, &mut seller, 1, 1); // price 1200 > 100
    assert_eq!(full_result, Err(ShopBuyError::SellerGoldFull));
    assert_eq!(seller.gold, GOLD_CAP - 100); // untouched

    // 5. Out of bounds slot returns InvalidIndex
    assert_eq!(
        complete_shop_buy(&mut buyer, &mut seller, 99, 1),
        Err(ShopBuyError::InvalidIndex)
    );
}

// =========================================================================
// Test 8: P2P Item Trade vs Pet Trade Separation & Cancel (Ticket 06)
// =========================================================================
#[tokio::test]
async fn test_p2p_trade_and_pet_trade_separation_and_cancel() {
    let data = GameData::default();
    let service = BattleService::default();
    let env = ServerEnv::none();

    let p1_id = 6001u32;
    let p2_id = 6002u32;

    // Put partner p2 into online session registry
    {
        let mut sessions = lock_online_sessions();
        let mut s2 = Session::default();
        s2.id = p2_id;
        sessions.insert(p2_id, s2);
    }

    // 1. Test 19 01 (Item Trade Invite): sub == 1
    let mut conn1 = create_test_conn(p1_id, p1_id as i64);
    let mut p2_bytes = Vec::new();
    p2_bytes.extend_from_slice(&p2_id.to_le_bytes());

    let mut out_item_trade = HandleOutcome::default();
    {
        let mut ctx = OpcodeCtx {
            conn: &mut conn1,
            data: &data,
            service: &service,
            out: &mut out_item_trade,
            opcode: 0x19,
            sub: 1, // Item Trade
            payload: &p2_bytes,
            decoded: &[],
            env,
        };
        handle_trade(&mut ctx).await;
    }

    let item_trade_frames: Vec<String> = out_item_trade.outgoing.into_iter().map(|f| f.frame).collect();
    let expected_item_trade_invite = format!("F44406001901{}", encoder::le32(p2_id));
    assert!(
        item_trade_frames.iter().any(|f| f == &expected_item_trade_invite),
        "Sub 1 must emit 19 01 item trade invite frame"
    );
    assert!(conn1.session.trade.active);
    assert_eq!(conn1.session.trade.partner_id, p2_id);

    // 2. Test Trade Cancellation (action == 2): Sub 3
    let mut out_cancel = HandleOutcome::default();
    {
        let mut ctx = OpcodeCtx {
            conn: &mut conn1,
            data: &data,
            service: &service,
            out: &mut out_cancel,
            opcode: 0x19,
            sub: 3,
            payload: &[0x00], // action == 0 (Cancel)
            decoded: &[],
            env,
        };
        handle_trade(&mut ctx).await;
    }

    let cancel_frames: Vec<String> = out_cancel.outgoing.into_iter().map(|f| f.frame).collect();
    assert!(
        cancel_frames.iter().any(|f| f == "F44402001904"),
        "Trade cancel must emit 19 04 to close trade form"
    );
    assert!(
        !cancel_frames.iter().any(|f| f == "F4440200173B"),
        "Trade cancel must NOT emit 17 3B"
    );
    assert!(!conn1.session.trade.active);

    // 3. Test 19 0A (Pet Trade Invite): sub == 10
    // Reset partner p2 in online registry
    {
        let mut sessions = lock_online_sessions();
        let mut s2 = Session::default();
        s2.id = p2_id;
        sessions.insert(p2_id, s2);
    }

    let mut out_pet_trade = HandleOutcome::default();
    {
        let mut ctx = OpcodeCtx {
            conn: &mut conn1,
            data: &data,
            service: &service,
            out: &mut out_pet_trade,
            opcode: 0x19,
            sub: 10, // Pet Trade
            payload: &p2_bytes,
            decoded: &[],
            env,
        };
        handle_trade(&mut ctx).await;
    }

    let pet_trade_frames: Vec<String> = out_pet_trade.outgoing.into_iter().map(|f| f.frame).collect();
    let expected_pet_trade_invite = format!("F4440600190A{}", encoder::le32(p2_id));
    assert!(
        pet_trade_frames.iter().any(|f| f == &expected_pet_trade_invite),
        "Sub 10 must emit 19 0A pet trade invite frame"
    );
    assert!(conn1.session.trade.active);

    // 4. Test Pet Trade Cancellation (action == 0): Sub 12
    let mut out_pet_cancel = HandleOutcome::default();
    {
        let mut ctx = OpcodeCtx {
            conn: &mut conn1,
            data: &data,
            service: &service,
            out: &mut out_pet_cancel,
            opcode: 0x19,
            sub: 12,
            payload: &[0x00], // action == 0 (Cancel)
            decoded: &[],
            env,
        };
        handle_trade(&mut ctx).await;
    }

    let pet_cancel_frames: Vec<String> = out_pet_cancel.outgoing.into_iter().map(|f| f.frame).collect();
    assert!(
        pet_cancel_frames.iter().any(|f| f == "F44402001904"),
        "Pet trade cancel must emit 19 04 to close trade form"
    );
    assert!(
        !pet_cancel_frames.iter().any(|f| f == "F4440200173B"),
        "Pet trade cancel must NOT emit 17 3B"
    );
    assert!(!conn1.session.trade.active);

    // 5. Test Trade Toast Responses: Offline (19 02 01) & Busy (19 02 02) (Ticket 06 / B3 fix)
    let offline_id = 9999u32;
    let offline_payload = offline_id.to_le_bytes();
    let mut out_offline = HandleOutcome::default();
    {
        let mut ctx = OpcodeCtx {
            conn: &mut conn1,
            data: &data,
            service: &service,
            out: &mut out_offline,
            opcode: 0x19,
            sub: 1,
            payload: &offline_payload,
            decoded: &[],
            env,
        };
        handle_trade(&mut ctx).await;
    }
    let offline_frames: Vec<String> = out_offline.outgoing.into_iter().map(|f| f.frame).collect();
    assert!(
        offline_frames.iter().any(|f| f == "F4440300190201"),
        "Must emit literal 19 02 01 when trade partner is offline"
    );

    // Test partner busy
    {
        let mut sessions = lock_online_sessions();
        let mut s2 = Session::default();
        s2.id = p2_id;
        s2.trade.active = true;
        sessions.insert(p2_id, s2);
    }
    let mut out_busy = HandleOutcome::default();
    {
        let mut ctx = OpcodeCtx {
            conn: &mut conn1,
            data: &data,
            service: &service,
            out: &mut out_busy,
            opcode: 0x19,
            sub: 1,
            payload: &p2_bytes,
            decoded: &[],
            env,
        };
        handle_trade(&mut ctx).await;
    }
    let busy_frames: Vec<String> = out_busy.outgoing.into_iter().map(|f| f.frame).collect();
    assert!(
        busy_frames.iter().any(|f| f == "F4440300190202"),
        "Must emit literal 19 02 02 when trade partner is busy"
    );

    // 6. Test Trade Timeout: 17 3B on expired trade (Ticket 06 / B2 fix)
    let mut s_timeout1 = Session::default();
    s_timeout1.id = 7001;
    s_timeout1.trade = ts_dream::server::session::TradeState {
        active: true,
        partner_id: 7002,
        started_at_ms: 1_000,
        ..Default::default()
    };
    let mut s_timeout2 = Session::default();
    s_timeout2.id = 7002;
    s_timeout2.trade = ts_dream::server::session::TradeState {
        active: true,
        partner_id: 7001,
        started_at_ms: 1_000,
        ..Default::default()
    };
    let future_time = 1_000 + TRADE_TIMEOUT_MS + 5_000;
    assert!(is_trade_timed_out(&s_timeout1.trade, future_time));
    assert!(expire_trade_if_timed_out(&mut s_timeout1, &mut s_timeout2, future_time));
    assert!(!s_timeout1.trade.active);
    assert!(!s_timeout2.trade.active);

    // Cancel timed out trade asynchronously produces 17 3B
    let mut conn_timeout = create_test_conn(7001, 7001);
    conn_timeout.session.trade = ts_dream::server::session::TradeState {
        active: true,
        partner_id: 7002,
        started_at_ms: 1_000,
        ..Default::default()
    };
    let mut out_to = HandleOutcome::default();
    let cancelled = cancel_timed_out_trade(&mut conn_timeout.session, &mut out_to, None).await;
    assert!(cancelled);
    assert!(!conn_timeout.session.trade.active);
    let to_frames: Vec<String> = out_to.outgoing.into_iter().map(|f| f.frame).collect();
    assert!(
        to_frames.iter().any(|f| f == "F44402001904"),
        "Timed-out trade must emit 19 04 to close trade form"
    );
    assert!(
        to_frames.iter().any(|f| f == "F4440300190203"),
        "Timed-out trade must emit 19 02 03 toast"
    );
    assert!(
        !to_frames.iter().any(|f| f == "F4440200173B"),
        "Timed-out trade must NOT emit 17 3B"
    );
}

// =========================================================================
// Test 9: Storage Persistence for TienTrang & LuuLang (Ticket 04 / B1 fix)
// =========================================================================
#[tokio::test]
async fn test_storage_warehouse_database_persistence() {
    let pool = setup_test_db().await;
    let repos = SqliteRepositories::new(pool.clone());
    let data = GameData::default();
    let service = BattleService::default();
    let env = ServerEnv {
        pool: Some(&pool),
        repos: Some(&repos),
        hub: None,
        sender: None,
    };

    let acc_id = ts_dream::db::accounts::create(&pool, "storeacc", "pass")
        .await
        .expect("create account");
    let char_id = acc_id;
    sqlx::query("INSERT INTO characters (playerid, name) VALUES (?, ?)")
        .bind(char_id)
        .bind(b"StoragePersistChar".to_vec())
        .execute(&pool.write)
        .await
        .expect("insert character");

    let mut conn = create_test_conn(char_id as u32, char_id);
    // Give player Ba Đậu Yêu Balo pet (id 41187) so LuuLang transfers are authorized
    conn.session.pets.push(PetState {
        id: 41187,
        stt: 1,
        ..Default::default()
    });

    // Put item 26001 (count 15) in homdo slot 1, and item 27001 (count 7) in slot 2
    conn.session.homdo.push(InventoryItem {
        slot: 1,
        id: 26001,
        count: 15,
        doben: 100,
        ..Default::default()
    });
    conn.session.homdo.push(InventoryItem {
        slot: 2,
        id: 27001,
        count: 7,
        doben: 80,
        ..Default::default()
    });

    // 1. Deposit slot 1 to Tiền Trang (storagetype = 2) via 1E 02 <slot 1>
    let mut out_dep = HandleOutcome::default();
    {
        let mut ctx = OpcodeCtx {
            conn: &mut conn,
            data: &data,
            service: &service,
            out: &mut out_dep,
            opcode: 0x1E,
            sub: 2, // STORAGE_SUB_DEPOSIT
            payload: &[0x01],
            decoded: &[],
            env,
        };
        handle_storage_transfer(&mut ctx).await;
    }
    assert_eq!(conn.session.tientrang.len(), 1);
    assert_eq!(conn.session.tientrang[0].id, 26001);
    assert_eq!(conn.session.tientrang[0].count, 15);

    // Verify row directly in SQLite DB: storagetype == 2 (TienTrang)
    let tientrang_row: (i64, i64, i64) = sqlx::query_as(
        "SELECT itemid, quantity, storagetype FROM inventories WHERE playerid = ? AND storagetype = 2"
    )
    .bind(char_id)
    .fetch_one(&pool.read)
    .await
    .expect("fetch tientrang row from SQLite");
    assert_eq!(tientrang_row.0, 26001);
    assert_eq!(tientrang_row.1, 15);
    assert_eq!(tientrang_row.2, 2, "storagetype must be 2 for TienTrang");

    // 2. Deposit slot 2 to Lưu Lãng (storagetype = 16) via 17 51 <slot 2>
    let mut out_ll = HandleOutcome::default();
    {
        let mut ctx = OpcodeCtx {
            conn: &mut conn,
            data: &data,
            service: &service,
            out: &mut out_ll,
            opcode: 0x17,
            sub: 51, // ITEM_SUB_WAREHOUSE_DEPOSIT
            payload: &[0x02],
            decoded: &[],
            env,
        };
        handle_storage_transfer(&mut ctx).await;
    }
    assert_eq!(conn.session.luulang.len(), 1);
    assert_eq!(conn.session.luulang[0].id, 27001);
    assert_eq!(conn.session.luulang[0].count, 7);

    // Verify row directly in SQLite DB: storagetype == 16 (LuuLang)
    let luulang_row: (i64, i64, i64) = sqlx::query_as(
        "SELECT itemid, quantity, storagetype FROM inventories WHERE playerid = ? AND storagetype = 16"
    )
    .bind(char_id)
    .fetch_one(&pool.read)
    .await
    .expect("fetch luulang row from SQLite");
    assert_eq!(luulang_row.0, 27001);
    assert_eq!(luulang_row.1, 7);
    assert_eq!(luulang_row.2, 16, "storagetype must be 16 for LuuLang");

    // 3. Withdraw from TienTrang back to HomDo via 1E 01
    let tt_slot = conn.session.tientrang[0].slot;
    let mut out_wdr_tt = HandleOutcome::default();
    {
        let mut ctx = OpcodeCtx {
            conn: &mut conn,
            data: &data,
            service: &service,
            out: &mut out_wdr_tt,
            opcode: 0x1E,
            sub: 1, // STORAGE_SUB_WITHDRAW
            payload: &[tt_slot],
            decoded: &[],
            env,
        };
        handle_storage_transfer(&mut ctx).await;
    }
    assert!(conn.session.tientrang.is_empty());
    let tt_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM inventories WHERE playerid = ? AND storagetype = 2"
    )
    .bind(char_id)
    .fetch_one(&pool.read)
    .await
    .expect("count tientrang rows");
    assert_eq!(tt_count, 0, "TienTrang should have 0 rows after withdrawal");
}

// =========================================================================
// Test 3: Player Stall Open & Catalog Buy (Ticket 02, 07)
// =========================================================================
#[tokio::test]
async fn test_player_stall_open_and_catalog_buy() {
    let data = GameData::default();
    let service = BattleService::default();
    let env = ServerEnv::none();

    let seller_id = 5501u32;
    let buyer_id = 5502u32;
    let mut seller_conn = create_test_conn(seller_id, seller_id as i64);
    seller_conn.session.gold = 10_000;
    seller_conn.session.homdo.push(InventoryItem {
        slot: 1,
        id: 10001,
        count: 10,
        ..Default::default()
    });

    // 10 slots * 10 bytes = 100 bytes item data
    let mut payload = Vec::new();
    payload.push(1u8); // bagSlot 1
    payload.push(5u8); // qty 5
    payload.extend_from_slice(&500u32.to_le_bytes()); // price 500
    payload.extend_from_slice(&100u32.to_le_bytes()); // vitality 100
    payload.extend_from_slice(&[0u8; 90]); // remaining 9 empty slots
    let stall_name = b"My Stall";
    payload.push(stall_name.len() as u8);
    payload.extend_from_slice(stall_name);
    payload.push(1u8); // image

    let mut out_open = HandleOutcome::default();
    {
        let mut ctx = OpcodeCtx {
            conn: &mut seller_conn,
            data: &data,
            service: &service,
            out: &mut out_open,
            opcode: 0x17,
            sub: 30, // STALL_OPEN
            payload: &payload,
            decoded: &[],
            env,
        };
        handle_player_shop(&mut ctx).await;
    }
    let open_frames: Vec<String> = out_open.outgoing.into_iter().map(|f| f.frame).collect();
    assert!(open_frames.iter().any(|f| f == "F4440300171E01"), "S->C response must be 3-byte F4440300171E01");
    assert!(seller_conn.session.shop.active);
    assert_eq!(seller_conn.session.shop.items.len(), 1);

    // Put seller into online registry
    {
        let mut sessions = lock_online_sessions();
        sessions.insert(seller_id, seller_conn.session.clone());
    }

    // Buyer views catalog: 17 32 <seller_id: 4B LE>
    let mut buyer_conn = create_test_conn(buyer_id, buyer_id as i64);
    buyer_conn.session.gold = 50_000;
    let mut out_view = HandleOutcome::default();
    {
        let mut ctx = OpcodeCtx {
            conn: &mut buyer_conn,
            data: &data,
            service: &service,
            out: &mut out_view,
            opcode: 0x17,
            sub: 32, // STALL_VIEW
            payload: &seller_id.to_le_bytes(),
            decoded: &[],
            env,
        };
        handle_player_shop(&mut ctx).await;
    }
    let view_frames: Vec<String> = out_view.outgoing.into_iter().map(|f| f.frame).collect();
    assert!(view_frames.iter().any(|f| f.contains("1721")), "Must send 17 21 catalog frame");

    // Buyer buys from stall: 17 33 <seller_id: 4B LE> <shop_index: 1> <count: 5>
    let mut buy_payload = Vec::new();
    buy_payload.extend_from_slice(&seller_id.to_le_bytes());
    buy_payload.push(1u8); // 1-based index
    buy_payload.push(5u8); // count 5
    let mut out_buy = HandleOutcome::default();
    {
        let mut ctx = OpcodeCtx {
            conn: &mut buyer_conn,
            data: &data,
            service: &service,
            out: &mut out_buy,
            opcode: 0x17,
            sub: 33, // STALL_BUY
            payload: &buy_payload,
            decoded: &[],
            env,
        };
        handle_player_shop(&mut ctx).await;
    }
    assert_eq!(buyer_conn.session.gold, 50_000 - 2_500);
}

// =========================================================================
// Test 4: P2P Trade Two-Phase Atomic Flow (Ticket 03, 07)
// =========================================================================
#[tokio::test]
async fn test_p2p_trade_two_phase_atomic_flow() {
    let data = GameData::default();
    let service = BattleService::default();
    let env = ServerEnv::none();

    let p1_id = 7101u32;
    let p2_id = 7102u32;

    let mut conn1 = create_test_conn(p1_id, p1_id as i64);
    conn1.session.gold = 10_000;
    conn1.session.homdo.push(InventoryItem {
        slot: 1,
        id: 10001,
        count: 5,
        ..Default::default()
    });

    let mut conn2 = create_test_conn(p2_id, p2_id as i64);
    conn2.session.gold = 20_000;
    conn2.session.homdo.push(InventoryItem {
        slot: 1,
        id: 20001,
        count: 2,
        ..Default::default()
    });

    conn1.session.trade.active = true;
    conn1.session.trade.partner_id = p2_id;
    conn2.session.trade.active = true;
    conn2.session.trade.partner_id = p1_id;

    {
        let mut sessions = lock_online_sessions();
        sessions.insert(p2_id, conn2.session.clone());
    }

    let mut offer1_payload = Vec::new();
    offer1_payload.extend_from_slice(&1_000u32.to_le_bytes());
    offer1_payload.push(1); offer1_payload.push(5);
    offer1_payload.extend_from_slice(&[0u8; 8]);
    let mut out_offer1 = HandleOutcome::default();
    {
        let mut ctx = OpcodeCtx {
            conn: &mut conn1,
            data: &data,
            service: &service,
            out: &mut out_offer1,
            opcode: 0x19,
            sub: 2,
            payload: &offer1_payload,
            decoded: &[],
            env,
        };
        handle_trade(&mut ctx).await;
    }
    assert_eq!(conn1.session.trade.gold, 1_000);
    assert_eq!(conn1.session.trade.items.len(), 1);

    {
        let mut sessions = lock_online_sessions();
        sessions.insert(p1_id, conn1.session.clone());
    }

    let mut offer2_payload = Vec::new();
    offer2_payload.extend_from_slice(&2_000u32.to_le_bytes());
    offer2_payload.push(1); offer2_payload.push(2);
    offer2_payload.extend_from_slice(&[0u8; 8]);
    let mut out_offer2 = HandleOutcome::default();
    {
        let mut ctx = OpcodeCtx {
            conn: &mut conn2,
            data: &data,
            service: &service,
            out: &mut out_offer2,
            opcode: 0x19,
            sub: 2,
            payload: &offer2_payload,
            decoded: &[],
            env,
        };
        handle_trade(&mut ctx).await;
    }
    assert_eq!(conn2.session.trade.gold, 2_000);
    assert_eq!(conn2.session.trade.items.len(), 1);

    {
        let mut sessions = lock_online_sessions();
        sessions.insert(p2_id, conn2.session.clone());
    }
    let mut out_conf1 = HandleOutcome::default();
    {
        let mut ctx = OpcodeCtx {
            conn: &mut conn1,
            data: &data,
            service: &service,
            out: &mut out_conf1,
            opcode: 0x19,
            sub: 3,
            payload: &[0x01],
            decoded: &[],
            env,
        };
        handle_trade(&mut ctx).await;
    }
    assert!(conn1.session.trade.accepted);

    {
        let mut sessions = lock_online_sessions();
        sessions.insert(p1_id, conn1.session.clone());
    }

    let mut out_conf2 = HandleOutcome::default();
    {
        let mut ctx = OpcodeCtx {
            conn: &mut conn2,
            data: &data,
            service: &service,
            out: &mut out_conf2,
            opcode: 0x19,
            sub: 3,
            payload: &[0x01],
            decoded: &[],
            env,
        };
        handle_trade(&mut ctx).await;
    }
    let conf2_frames: Vec<String> = out_conf2.outgoing.into_iter().map(|f| f.frame).collect();
    assert!(conf2_frames.iter().any(|f| f == "F4440300190204"), "Must send 19 02 04 success toast");
    assert!(conf2_frames.iter().any(|f| f == "F44402001904"), "Must send 19 04 close form");
    assert!(!conn2.session.trade.active);
    assert_eq!(conn2.session.gold, 20_000 - 2_000 + 1_000);
    assert!(conn2.session.homdo.iter().any(|i| i.id == 10001 && i.count == 5));
}

// =========================================================================
// Test 5: P2P Trade Cancel Wire Safety (Ticket 03, 07)
// =========================================================================
#[tokio::test]
async fn test_p2p_trade_cancel_wire_safety() {
    let data = GameData::default();
    let service = BattleService::default();
    let env = ServerEnv::none();

    let p1_id = 7201u32;
    let p2_id = 7202u32;

    let mut conn1 = create_test_conn(p1_id, p1_id as i64);
    conn1.session.trade.active = true;
    conn1.session.trade.partner_id = p2_id;

    let mut s2 = Session::default();
    s2.id = p2_id;
    s2.trade.active = true;
    s2.trade.partner_id = p1_id;
    {
        let mut sessions = lock_online_sessions();
        sessions.insert(p2_id, s2);
    }

    let mut out_cancel = HandleOutcome::default();
    {
        let mut ctx = OpcodeCtx {
            conn: &mut conn1,
            data: &data,
            service: &service,
            out: &mut out_cancel,
            opcode: 0x19,
            sub: 3,
            payload: &[0x00], // cancel
            decoded: &[],
            env,
        };
        handle_trade(&mut ctx).await;
    }
    let frames: Vec<String> = out_cancel.outgoing.into_iter().map(|f| f.frame).collect();
    assert!(frames.iter().any(|f| f == "F44402001904"), "Cancel must emit 19 04 to close trade form");
    assert!(!frames.iter().any(|f| f.contains("173B")), "Cancel must NEVER emit 17 3B");
    assert!(!conn1.session.trade.active);
}

// =========================================================================
// Test 6: NPC Shop Bulk Buy All-Or-Nothing (Ticket 04, 07)
// =========================================================================
#[tokio::test]
async fn test_npc_shop_bulk_buy_all_or_nothing() {
    let data = GameData::default();
    let service = BattleService::default();
    let env = ServerEnv::none();

    // 1. Buy 100 items with 2 empty slots -> success (2 frames 17 06)
    let mut conn = create_test_conn(3001, 3001);
    conn.session.gold = 500_000;
    for slot in 1..=23 {
        conn.session.homdo.push(InventoryItem {
            slot,
            id: 10001,
            count: 50,
            ..Default::default()
        });
    }
    let payload = vec![0x00, 100, 0];
    let mut out = HandleOutcome::default();
    {
        let mut ctx = OpcodeCtx {
            conn: &mut conn,
            data: &data,
            service: &service,
            out: &mut out,
            opcode: 0x1B,
            sub: 1,
            payload: &payload,
            decoded: &[],
            env,
        };
        handle_npc_shop(&mut ctx).await;
    }
    let frames: Vec<String> = out.outgoing.into_iter().map(|f| f.frame).collect();
    assert!(frames.iter().any(|f| f == "F44403001B0100"));
    let added_frames: Vec<_> = frames.iter().filter(|f| f.contains("1706")).collect();
    assert_eq!(added_frames.len(), 2, "Must send exactly 2 frames of 17 06 for 100 items (2x50)");

    // 2. Buy 100 items with only 1 empty slot -> reject 1B 01 02, gold untouched
    let mut conn_tight = create_test_conn(3002, 3002);
    conn_tight.session.gold = 500_000;
    for slot in 1..=24 {
        conn_tight.session.homdo.push(InventoryItem {
            slot,
            id: 10001,
            count: 50,
            ..Default::default()
        });
    }
    let mut out_tight = HandleOutcome::default();
    {
        let mut ctx = OpcodeCtx {
            conn: &mut conn_tight,
            data: &data,
            service: &service,
            out: &mut out_tight,
            opcode: 0x1B,
            sub: 1,
            payload: &payload,
            decoded: &[],
            env,
        };
        handle_npc_shop(&mut ctx).await;
    }
    let tight_frames: Vec<String> = out_tight.outgoing.into_iter().map(|f| f.frame).collect();
    assert!(tight_frames.iter().any(|f| f == "F44403001B0102"), "Must return 1B 01 02 (BAG_FULL)");
    assert_eq!(conn_tight.session.gold, 500_000, "Gold must be untouched 100% on reject");

    // 3. Buy 1250 items with 25 empty slots -> success (25 frames 17 06)
    let mut conn_empty = create_test_conn(3003, 3003);
    conn_empty.session.gold = 500_000;
    let payload_1250 = vec![0x00, (1250u16 & 0xFF) as u8, (1250u16 >> 8) as u8];
    let mut out_1250 = HandleOutcome::default();
    {
        let mut ctx = OpcodeCtx {
            conn: &mut conn_empty,
            data: &data,
            service: &service,
            out: &mut out_1250,
            opcode: 0x1B,
            sub: 1,
            payload: &payload_1250,
            decoded: &[],
            env,
        };
        handle_npc_shop(&mut ctx).await;
    }
    let frames_1250: Vec<String> = out_1250.outgoing.into_iter().map(|f| f.frame).collect();
    assert!(frames_1250.iter().any(|f| f == "F44403001B0100"));
    let added_1250: Vec<_> = frames_1250.iter().filter(|f| f.contains("1706")).collect();
    assert_eq!(added_1250.len(), 25, "Must send exactly 25 frames of 17 06 for 1250 items");
}

// =========================================================================
// Test 7: NPC Dialogue Menu Shop Segregation (Ticket 04, 07)
// =========================================================================
#[tokio::test]
async fn test_npc_dialogue_menu_shop_segregation() {
    let data = GameData::default();
    let service = BattleService::default();
    let env = ServerEnv::none();

    let mut conn = create_test_conn(4001, 4001);
    conn.session.idtalking = 1;
    conn.session.idnpctalking = 15002;
    conn.session.select_menu = 30;

    let mut out = HandleOutcome::default();
    {
        let mut ctx = OpcodeCtx {
            conn: &mut conn,
            data: &data,
            service: &service,
            out: &mut out,
            opcode: 0x14,
            sub: 6,
            payload: &[],
            decoded: &[],
            env,
        };
        handle_talk(&mut ctx).await;
    }
    let frames: Vec<String> = out.outgoing.into_iter().map(|f| f.frame).collect();
    assert!(frames.iter().any(|f| f.contains("1401")), "Menu 30 must send 14 01 buy frame");
    assert!(!frames.iter().any(|f| f.contains("1B03")), "Menu 30 must NEVER send 1B 03 sell frame");
}

// =========================================================================
// Test 8: Five Containers Wire and Capacity (Ticket 06, 07)
// =========================================================================
#[tokio::test]
async fn test_five_containers_wire_and_capacity() {
    let data = GameData::default();
    let service = BattleService::default();
    let env = ServerEnv::none();

    // 1. Luulang capacity is exactly 10 slots
    let mut conn = create_test_conn(5001, 5001);
    conn.session.pets.push(PetState { id: 41187, ..Default::default() });
    for slot in 1..=10 {
        conn.session.luulang.push(InventoryItem {
            slot,
            id: 20000 + slot as u16,
            count: 1,
            ..Default::default()
        });
    }
    conn.session.homdo.push(InventoryItem {
        slot: 1,
        id: 30001,
        count: 1,
        ..Default::default()
    });
    let mut out_ll = HandleOutcome::default();
    {
        let mut ctx = OpcodeCtx {
            conn: &mut conn,
            data: &data,
            service: &service,
            out: &mut out_ll,
            opcode: 0x17,
            sub: 51,
            payload: &[0x01],
            decoded: &[],
            env,
        };
        handle_storage_transfer(&mut ctx).await;
    }
    assert_eq!(conn.session.luulang.len(), 10, "Luulang cannot exceed 10 slots");
    assert_eq!(conn.session.homdo.len(), 1, "Homdo item remains when luulang is full");

    let mut out_ll_wdr = HandleOutcome::default();
    {
        let mut ctx = OpcodeCtx {
            conn: &mut conn,
            data: &data,
            service: &service,
            out: &mut out_ll_wdr,
            opcode: 0x17,
            sub: 52,
            payload: &[0x01],
            decoded: &[],
            env,
        };
        handle_storage_transfer(&mut ctx).await;
    }
    let ll_wdr_frames: Vec<String> = out_ll_wdr.outgoing.into_iter().map(|f| f.frame).collect();
    assert!(ll_wdr_frames.iter().any(|f| f == "F4440300176801"), "Withdraw from luulang must send F4440300176801");

    // 2. Tientrang dump preceded by 1E 08
    let mut session = Session::default();
    session.tientrang.push(InventoryItem {
        slot: 1,
        id: 10001,
        count: 5,
        ..Default::default()
    });
    let login_frames = build_logined_sequence_session(&session);
    let clear_idx = login_frames.iter().position(|f| f == "F44402001E08").expect("1E 08 clear frame present");
    let dump_idx = login_frames.iter().position(|f| f.contains("1E01")).expect("1E 01 dump frame present");
    assert_eq!(clear_idx + 1, dump_idx, "1E 08 mass clear must immediately precede 1E 01 dump");

    conn.session.tientrang.push(InventoryItem {
        slot: 1,
        id: 10001,
        count: 5,
        ..Default::default()
    });
    let mut out_tt_wdr = HandleOutcome::default();
    {
        let mut ctx = OpcodeCtx {
            conn: &mut conn,
            data: &data,
            service: &service,
            out: &mut out_tt_wdr,
            opcode: 0x1E,
            sub: 1,
            payload: &[0x01],
            decoded: &[],
            env,
        };
        handle_storage_transfer(&mut ctx).await;
    }
    let tt_wdr_frames: Vec<String> = out_tt_wdr.outgoing.into_iter().map(|f| f.frame).collect();
    assert!(tt_wdr_frames.iter().any(|f| f == "F44404001E050105"), "Withdraw from tientrang must emit 1E 05");

    // 3. Trangbi: dump_trangbi produces 10 bytes/record without slot byte
    session.trangbi.push(InventoryItem {
        slot: 1,
        id: 11001,
        count: 1,
        long_val: 2,
        giatri_long: 3,
        khang: 4,
        texp: 50,
        ..Default::default()
    });
    let trangbi_dump = session.dump_trangbi();
    assert!(trangbi_dump.starts_with("F4440C00170B"));
}

// =========================================================================
// Test 9: Zero 020B Red Message Toast (Ticket 05, 07)
// =========================================================================
#[tokio::test]
async fn test_zero_020b_red_message_toast() {
    let data = GameData::default();
    let service = BattleService::default();
    let env = ServerEnv::none();

    let mut conn = create_test_conn(6001, 6001);
    conn.session.gold = 0;

    let mut out1 = HandleOutcome::default();
    {
        let mut ctx = OpcodeCtx {
            conn: &mut conn,
            data: &data,
            service: &service,
            out: &mut out1,
            opcode: 0x1B,
            sub: 1,
            payload: &[0x00, 0x05, 0x00],
            decoded: &[],
            env,
        };
        handle_npc_shop(&mut ctx).await;
    }
    for f in &out1.outgoing {
        assert!(!f.frame.contains("020B"), "NPC buy error must not contain 020B: {}", f.frame);
    }

    let mut out2 = HandleOutcome::default();
    {
        let mut ctx = OpcodeCtx {
            conn: &mut conn,
            data: &data,
            service: &service,
            out: &mut out2,
            opcode: 0x17,
            sub: 33,
            payload: &[0x99, 0x99, 0x00, 0x00, 0x01, 0x01],
            decoded: &[],
            env,
        };
        handle_player_shop(&mut ctx).await;
    }
    for f in &out2.outgoing {
        assert!(!f.frame.contains("020B"), "Stall buy error must not contain 020B: {}", f.frame);
    }

    conn.session.gold = GOLD_CAP;
    let mut out3 = HandleOutcome::default();
    {
        let mut ctx = OpcodeCtx {
            conn: &mut conn,
            data: &data,
            service: &service,
            out: &mut out3,
            opcode: 0x1D,
            sub: 1,
            payload: &1000u32.to_le_bytes(),
            decoded: &[],
            env,
        };
        handle_bank_gold(&mut ctx).await;
    }
    for f in &out3.outgoing {
        assert!(!f.frame.contains("020B"), "Bank overflow must not contain 020B: {}", f.frame);
    }

    // 4. sys_msg_frame must emit channel 020102 and never 020B
    let sys_msg = ts_dream::server::spawn::sys_msg_frame("Thong bao he thong");
    assert!(!sys_msg.contains("020B"), "sys_msg_frame must not contain 020B: {}", sys_msg);
    assert!(sys_msg.contains("020102"), "sys_msg_frame must use channel 020102: {}", sys_msg);
}

#[tokio::test]
async fn test_player_stall_open_rejects_exceeding_gold_cap() {
    let data = GameData::default();
    let service = BattleService::default();
    let env = ServerEnv::none();

    let mut conn = create_test_conn(9901, 9901);
    conn.session.homdo.push(InventoryItem {
        slot: 1,
        id: 10001,
        count: 10,
        ..Default::default()
    });

    let mut payload = vec![0u8; 100];
    payload[0] = 1; // slot 1
    payload[1] = 1; // qty 1
    // Price exceeds GOLD_CAP (10_000_000 > 9_999_999)
    payload[2..6].copy_from_slice(&10_000_000u32.to_le_bytes());
    payload.push(6); // name_len
    payload.extend_from_slice(b"MyShop");
    payload.push(0); // image

    let mut out = HandleOutcome::default();
    {
        let mut ctx = OpcodeCtx {
            conn: &mut conn,
            data: &data,
            service: &service,
            out: &mut out,
            opcode: 0x17,
            sub: 30,
            payload: &payload,
            decoded: &[],
            env,
        };
        handle_player_shop(&mut ctx).await;
    }

    assert_eq!(out.outgoing.len(), 1);
    assert_eq!(out.outgoing[0].frame, "F4440300171E02", "Must reject price exceeding GOLD_CAP with 17 1E 02");
    assert!(!conn.session.shop.active);
}

#[tokio::test]
async fn test_storage_partial_quantity_and_no_collateral_transfer() {
    let data = GameData::default();
    let service = BattleService::default();
    let env = ServerEnv::none();

    let mut conn = create_test_conn(8801, 8801);
    // HomDo has item in slot 1 (count 15) AND item in slot 5 (count 1)
    conn.session.homdo.push(InventoryItem {
        slot: 1,
        id: 26001,
        count: 15,
        doben: 100,
        ..Default::default()
    });
    conn.session.homdo.push(InventoryItem {
        slot: 5,
        id: 55555, // Collateral item that must NOT be touched
        count: 1,
        doben: 100,
        ..Default::default()
    });

    // 1. Deposit 5 units from slot 1: payload is [slot=1, qty=5]
    let dep_payload = [0x01, 0x05];
    let mut out_dep = HandleOutcome::default();
    {
        let mut ctx = OpcodeCtx {
            conn: &mut conn,
            data: &data,
            service: &service,
            out: &mut out_dep,
            opcode: 0x1E,
            sub: 2, // STORAGE_SUB_DEPOSIT
            payload: &dep_payload,
            decoded: &[],
            env,
        };
        handle_storage_transfer(&mut ctx).await;
    }

    let dep_frames: Vec<String> = out_dep.outgoing.into_iter().map(|f| f.frame).collect();
    // Verify: slot 1 had 5 deducted (F444040017090105)
    assert!(
        dep_frames.iter().any(|f| f == "F444040017090105"),
        "Must send 17 09 01 05 for 5 units deposited: found {:?}",
        dep_frames
    );
    // Verify: 1E 02 frame emitted for 5 units (count = 0x05)
    assert!(
        dep_frames.iter().any(|f| f == "F4440D001E029165056400640000000000"),
        "Must send 1E 02 with count 5: found {:?}",
        dep_frames
    );
    // Verify: slot 5 item was NOT touched at all!
    assert!(
        !dep_frames.iter().any(|f| f.contains("170905")),
        "Collateral slot 5 must NOT have 17 09 emitted: found {:?}",
        dep_frames
    );
    let homdo_slot1 = conn.session.homdo.iter().find(|i| i.slot == 1).expect("slot 1 exists");
    assert_eq!(homdo_slot1.count, 10, "Homdo slot 1 should retain 10 units (15 - 5)");
    let homdo_slot5 = conn.session.homdo.iter().find(|i| i.slot == 5).expect("slot 5 untouched");
    assert_eq!(homdo_slot5.id, 55555, "Slot 5 collateral item must remain intact");

    assert_eq!(conn.session.tientrang.len(), 1);
    assert_eq!(conn.session.tientrang[0].count, 5, "TienTrang should have received 5 units");

    // 2. Withdraw 2 units from TienTrang: payload is [tientrang_slot=1, qty=2]
    let tt_slot = conn.session.tientrang[0].slot;
    let wdr_payload = [tt_slot, 0x02];
    let mut out_wdr = HandleOutcome::default();
    {
        let mut ctx = OpcodeCtx {
            conn: &mut conn,
            data: &data,
            service: &service,
            out: &mut out_wdr,
            opcode: 0x1E,
            sub: 1, // STORAGE_SUB_WITHDRAW
            payload: &wdr_payload,
            decoded: &[],
            env,
        };
        handle_storage_transfer(&mut ctx).await;
    }

    let wdr_frames: Vec<String> = out_wdr.outgoing.into_iter().map(|f| f.frame).collect();
    // Verify: 1E 05 emitted for slot 1 with qty 2: F44404001E050102
    assert!(
        wdr_frames.iter().any(|f| f == "F44404001E050102"),
        "Must send 1E 05 01 02: found {:?}",
        wdr_frames
    );
    // TienTrang should retain 3 units (5 - 2)
    assert_eq!(conn.session.tientrang.len(), 1);
    assert_eq!(conn.session.tientrang[0].count, 3);
}

// =========================================================================
// Test 21: P2P Trade Invite Error Wire Parity (Ticket 09)
// =========================================================================
#[tokio::test]
async fn test_p2p_trade_invite_error_wire_parity() {
    let data = GameData::default();
    let service = BattleService::default();
    let env = ServerEnv::none();

    let p1_id = 7301u32;
    let offline_p2_id = 7302u32;
    let busy_p3_id = 7303u32;

    // Ensure neither is in online sessions initially
    {
        let mut sessions = lock_online_sessions();
        sessions.remove(&offline_p2_id);
        sessions.remove(&busy_p3_id);
    }

    let mut conn1 = create_test_conn(p1_id, p1_id as i64);

    // 1. Invite offline player -> S->C 19 02 01
    let mut out_offline = HandleOutcome::default();
    {
        let mut ctx = OpcodeCtx {
            conn: &mut conn1,
            data: &data,
            service: &service,
            out: &mut out_offline,
            opcode: 0x19,
            sub: ts_dream::protocol::TRADE_SUB_INVITE_CS, // 0x01
            payload: &offline_p2_id.to_le_bytes(),
            decoded: &[],
            env,
        };
        handle_trade(&mut ctx).await;
    }
    let offline_frames: Vec<String> = out_offline.outgoing.into_iter().map(|f| f.frame).collect();
    assert!(
        offline_frames.iter().any(|f| f == "F4440300190201"),
        "Inviting offline target must emit 19 02 01 toast, got: {:?}",
        offline_frames
    );
    assert!(!conn1.session.trade.active);

    // 2. Put busy_p3 online with active trade -> S->C 19 02 02
    {
        let mut sessions = lock_online_sessions();
        let mut s3 = Session::default();
        s3.id = busy_p3_id;
        s3.trade.active = true;
        s3.trade.partner_id = 9999;
        sessions.insert(busy_p3_id, s3);
    }

    let mut out_busy = HandleOutcome::default();
    {
        let mut ctx = OpcodeCtx {
            conn: &mut conn1,
            data: &data,
            service: &service,
            out: &mut out_busy,
            opcode: 0x19,
            sub: ts_dream::protocol::TRADE_SUB_INVITE_CS,
            payload: &busy_p3_id.to_le_bytes(),
            decoded: &[],
            env,
        };
        handle_trade(&mut ctx).await;
    }
    let busy_frames: Vec<String> = out_busy.outgoing.into_iter().map(|f| f.frame).collect();
    assert!(
        busy_frames.iter().any(|f| f == "F4440300190202"),
        "Inviting busy target must emit 19 02 02 toast, got: {:?}",
        busy_frames
    );
    assert!(!conn1.session.trade.active);

    // 3. Inviting self without being in registry -> S->C 19 02 02
    {
        let mut sessions = lock_online_sessions();
        sessions.remove(&p1_id);
    }
    let mut out_self = HandleOutcome::default();
    {
        let mut ctx = OpcodeCtx {
            conn: &mut conn1,
            data: &data,
            service: &service,
            out: &mut out_self,
            opcode: 0x19,
            sub: ts_dream::protocol::TRADE_SUB_INVITE_CS,
            payload: &p1_id.to_le_bytes(),
            decoded: &[],
            env,
        };
        handle_trade(&mut ctx).await;
    }
    let self_frames: Vec<String> = out_self.outgoing.into_iter().map(|f| f.frame).collect();
    assert!(
        self_frames.iter().any(|f| f == "F4440300190202"),
        "Inviting self must emit 19 02 02 toast even if not in registry, got: {:?}",
        self_frames
    );

    // 4. Inviting someone while sender is already busy in trade -> S->C 19 02 02
    conn1.session.trade.active = true;
    let mut out_sender_busy = HandleOutcome::default();
    {
        let mut ctx = OpcodeCtx {
            conn: &mut conn1,
            data: &data,
            service: &service,
            out: &mut out_sender_busy,
            opcode: 0x19,
            sub: ts_dream::protocol::TRADE_SUB_INVITE_CS,
            payload: &offline_p2_id.to_le_bytes(),
            decoded: &[],
            env,
        };
        handle_trade(&mut ctx).await;
    }
    let sender_busy_frames: Vec<String> = out_sender_busy.outgoing.into_iter().map(|f| f.frame).collect();
    assert!(
        sender_busy_frames.iter().any(|f| f == "F4440300190202"),
        "Inviting while sender is already busy in trade must emit 19 02 02 toast, got: {:?}",
        sender_busy_frames
    );
    conn1.session.trade.active = false;
}

// =========================================================================
// Test 22: P2P Trade Timeout Emits 19 04 and 19 02 03 Toast to Both Players (Ticket 09)
// =========================================================================
#[tokio::test]
async fn test_p2p_trade_timeout_emits_close_and_toast_both_players() {
    let p1_id = 7401u32;
    let p2_id = 7402u32;

    let app = std::sync::Arc::new(tokio::sync::RwLock::new(ts_dream::state::AppState::new(100)));
    let control = ts_dream::web::server_control::ServerControl::new(19250, app, None, None);

    let (p2_tx, mut p2_rx) = tokio::sync::mpsc::channel(16);
    let registered = control.login_register(p2_id, &p2_tx).await;
    assert!(registered, "Must register p2 in control");

    let mut conn1 = create_test_conn(p1_id, p1_id as i64);
    conn1.session.trade = ts_dream::server::session::TradeState {
        active: true,
        partner_id: p2_id,
        started_at_ms: 1_000, // Long expired
        ..Default::default()
    };

    let mut s2 = Session::default();
    s2.id = p2_id;
    s2.trade = ts_dream::server::session::TradeState {
        active: true,
        partner_id: p1_id,
        started_at_ms: 1_000,
        ..Default::default()
    };
    {
        let mut sessions = lock_online_sessions();
        sessions.insert(p2_id, s2);
    }

    let mut out1 = HandleOutcome::default();
    let cancelled = cancel_timed_out_trade(&mut conn1.session, &mut out1, Some(&control)).await;
    assert!(cancelled, "Must cancel timed out trade");
    assert!(!conn1.session.trade.active, "p1 trade must be inactive");

    // Check frames emitted to p1 (local)
    let p1_frames: Vec<String> = out1.outgoing.into_iter().map(|f| f.frame).collect();
    assert!(
        p1_frames.iter().any(|f| f == "F44402001904"),
        "p1 must receive 19 04 close trade form, got: {:?}",
        p1_frames
    );
    assert!(
        p1_frames.iter().any(|f| f == "F4440300190203"),
        "p1 must receive 19 02 03 cancel toast, got: {:?}",
        p1_frames
    );

    // Check frames emitted to p2 via hub
    let mut p2_frames = Vec::new();
    while let Ok(frame) = p2_rx.try_recv() {
        p2_frames.push(frame);
    }
    assert!(
        p2_frames.iter().any(|f| f == "F44402001904"),
        "p2 must receive 19 04 close trade form via hub, got: {:?}",
        p2_frames
    );
    assert!(
        p2_frames.iter().any(|f| f == "F4440300190203"),
        "p2 must receive 19 02 03 cancel toast via hub, got: {:?}",
        p2_frames
    );

    // Check registry state for p2
    let s2_after = lock_online_sessions().get(&p2_id).cloned().expect("p2 in registry");
    assert!(!s2_after.trade.active, "p2 trade must be reset in registry");
}

// =========================================================================
// Test 23: Natural 2-Party P2P Trade Flow & Offline Partner Recovery
// =========================================================================
#[tokio::test]
async fn test_p2p_trade_natural_flow_and_partner_offline_recovery() {
    let data = GameData::default();
    let service = BattleService::default();
    let env = ServerEnv::none();

    let p1_id = 7501u32;
    let p2_id = 7502u32;

    let mut conn1 = create_test_conn(p1_id, p1_id as i64);
    conn1.session.gold = 5_000;
    conn1.session.homdo.push(InventoryItem {
        slot: 1,
        id: 11111,
        count: 3,
        ..Default::default()
    });

    let mut conn2 = create_test_conn(p2_id, p2_id as i64);
    conn2.session.gold = 10_000;
    conn2.session.homdo.push(InventoryItem {
        slot: 1,
        id: 22222,
        count: 1,
        ..Default::default()
    });

    // Register both players as online initially
    {
        let mut sessions = lock_online_sessions();
        sessions.insert(p1_id, conn1.session.clone());
        sessions.insert(p2_id, conn2.session.clone());
    }

    // 1. P1 invites P2 via 19 01
    let mut out1_invite = HandleOutcome::default();
    {
        let mut ctx = OpcodeCtx {
            conn: &mut conn1,
            data: &data,
            service: &service,
            out: &mut out1_invite,
            opcode: 0x19,
            sub: ts_dream::protocol::TRADE_SUB_INVITE_CS,
            payload: &p2_id.to_le_bytes(),
            decoded: &[],
            env,
        };
        handle_trade(&mut ctx).await;
    }
    assert!(conn1.session.trade.active, "P1 trade should be active");
    assert_eq!(conn1.session.trade.partner_id, p2_id);

    // Sync conn2's session from registry where P1 set P2's trade state
    conn2.session = lock_online_sessions().get(&p2_id).cloned().unwrap();
    assert!(conn2.session.trade.active, "P2 trade should be active in registry");
    assert_eq!(conn2.session.trade.partner_id, p1_id);

    // 2. P1 offers 500 gold and item 11111 (slot 1, qty 2)
    let mut offer1_payload = Vec::new();
    offer1_payload.extend_from_slice(&500u32.to_le_bytes());
    offer1_payload.push(1);
    offer1_payload.push(2);
    offer1_payload.extend_from_slice(&[0u8; 8]);
    let mut out1_offer = HandleOutcome::default();
    {
        let mut ctx = OpcodeCtx {
            conn: &mut conn1,
            data: &data,
            service: &service,
            out: &mut out1_offer,
            opcode: 0x19,
            sub: 2,
            payload: &offer1_payload,
            decoded: &[],
            env,
        };
        handle_trade(&mut ctx).await;
    }

    // 3. P2 offers 1,000 gold and item 22222 (slot 1, qty 1)
    let mut offer2_payload = Vec::new();
    offer2_payload.extend_from_slice(&1_000u32.to_le_bytes());
    offer2_payload.push(1);
    offer2_payload.push(1);
    offer2_payload.extend_from_slice(&[0u8; 8]);
    let mut out2_offer = HandleOutcome::default();
    {
        let mut ctx = OpcodeCtx {
            conn: &mut conn2,
            data: &data,
            service: &service,
            out: &mut out2_offer,
            opcode: 0x19,
            sub: 2,
            payload: &offer2_payload,
            decoded: &[],
            env,
        };
        handle_trade(&mut ctx).await;
    }

    // 4. P1 confirms (action == 1) - Note: NO manual sessions.insert!
    let mut out1_confirm = HandleOutcome::default();
    {
        let mut ctx = OpcodeCtx {
            conn: &mut conn1,
            data: &data,
            service: &service,
            out: &mut out1_confirm,
            opcode: 0x19,
            sub: 3,
            payload: &[0x01],
            decoded: &[],
            env,
        };
        handle_trade(&mut ctx).await;
    }
    assert!(conn1.session.trade.accepted, "P1 should be accepted");

    // 5. P2 confirms (action == 1) - Should settle automatically!
    let mut out2_confirm = HandleOutcome::default();
    {
        let mut ctx = OpcodeCtx {
            conn: &mut conn2,
            data: &data,
            service: &service,
            out: &mut out2_confirm,
            opcode: 0x19,
            sub: 3,
            payload: &[0x01],
            decoded: &[],
            env,
        };
        handle_trade(&mut ctx).await;
    }
    let conf2_frames: Vec<String> = out2_confirm.outgoing.into_iter().map(|f| f.frame).collect();
    assert!(conf2_frames.iter().any(|f| f == "F4440300190204"), "Must send 19 02 04 success toast");
    assert!(conf2_frames.iter().any(|f| f == "F44402001904"), "Must send 19 04 close form");
    assert!(!conn2.session.trade.active, "P2 trade state must be reset");

    // Check balances on P2: started with 10k, offered 1k, received 500 = 9,500
    assert_eq!(conn2.session.gold, 9_500);
    // P2 received item 11111 (count 2)
    assert!(conn2.session.homdo.iter().any(|i| i.id == 11111 && i.count == 2));

    // Verify P1 in registry settled as well
    let s1_after = lock_online_sessions().get(&p1_id).cloned().expect("p1 in registry");
    assert!(!s1_after.trade.active, "P1 trade state in registry must be reset");
    assert_eq!(s1_after.gold, 5_000 - 500 + 1_000); // 5,500
    assert!(s1_after.homdo.iter().any(|i| i.id == 22222 && i.count == 1));

    // 6. Test Recovery when Partner vanishes midway through trade:
    let now_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0);
    let mut conn_solo = create_test_conn(7601, 7601);
    conn_solo.session.trade = ts_dream::server::session::TradeState {
        active: true,
        partner_id: 9999, // offline
        started_at_ms: now_ms, // current time (not timed out)
        ..Default::default()
    };
    let mut out_vanished = HandleOutcome::default();
    {
        let mut ctx = OpcodeCtx {
            conn: &mut conn_solo,
            data: &data,
            service: &service,
            out: &mut out_vanished,
            opcode: 0x19,
            sub: 3,
            payload: &[0x01], // Confirm attempt
            decoded: &[],
            env,
        };
        handle_trade(&mut ctx).await;
    }
    let vanished_frames: Vec<String> = out_vanished.outgoing.into_iter().map(|f| f.frame).collect();
    assert!(
        vanished_frames.iter().any(|f| f == "F44402001904"),
        "Must emit 19 04 to close trade form when partner vanished, got: {:?}",
        vanished_frames
    );
    assert!(
        vanished_frames.iter().any(|f| f == "F4440300190201"),
        "Must emit 19 02 01 toast (offline) when partner vanished, got: {:?}",
        vanished_frames
    );
    assert!(!conn_solo.session.trade.active, "Solo session trade must be deactivated");
}

#[tokio::test]
async fn test_bank_withdraw_overflow_vs_insufficient_balance() {
    let data = GameData::default();
    let service = BattleService::default();
    let env = ServerEnv::none();

    let mut conn = create_test_conn(8881, 8881);

    // Case 1: Insufficient bank balance.
    // Even if player's gold is already at GOLD_CAP, when bank balance is 0 or less than amount requested,
    // the failure is due to insufficient bank balance, NOT inventory overflow.
    // Must receive 1D 03 and 1D 08, and MUST NOT receive 1A 06!
    conn.session.gold = GOLD_CAP;
    conn.session.bank_gold = 0;
    let mut out_insufficient = HandleOutcome::default();
    {
        let mut ctx = OpcodeCtx {
            conn: &mut conn,
            data: &data,
            service: &service,
            out: &mut out_insufficient,
            opcode: 0x1D,
            sub: ts_dream::protocol::BANK_SUB_WITHDRAW,
            payload: &50_000u32.to_le_bytes(),
            decoded: &[],
            env,
        };
        handle_bank_gold(&mut ctx).await;
    }
    let frames_insufficient: Vec<String> = out_insufficient.outgoing.into_iter().map(|f| f.frame).collect();
    assert!(
        frames_insufficient.iter().any(|f| f == "F44402001D03"),
        "Insufficient bank balance must emit 1D 03: {:?}",
        frames_insufficient
    );
    assert!(
        frames_insufficient.iter().any(|f| f == "F44402001D08"),
        "Insufficient bank balance must unlock withdraw UI via 1D 08: {:?}",
        frames_insufficient
    );
    assert!(
        !frames_insufficient.iter().any(|f| f == "F44402001A06"),
        "Insufficient bank balance MUST NOT emit 1A 06 overflow error: {:?}",
        frames_insufficient
    );

    // Case 2: Bank has sufficient gold, BUT withdrawing would cause player inventory gold to exceed GOLD_CAP.
    // e.g. player has 9_900_000, bank has 200_000, withdraw 200_000 -> 9_900_000 + 200_000 = 10_100_000 > GOLD_CAP.
    // Must receive 1D 03, 1D 08, AND 1A 06!
    conn.session.gold = 9_900_000;
    conn.session.bank_gold = 200_000;
    let mut out_overflow = HandleOutcome::default();
    {
        let mut ctx = OpcodeCtx {
            conn: &mut conn,
            data: &data,
            service: &service,
            out: &mut out_overflow,
            opcode: 0x1D,
            sub: ts_dream::protocol::BANK_SUB_WITHDRAW,
            payload: &200_000u32.to_le_bytes(),
            decoded: &[],
            env,
        };
        handle_bank_gold(&mut ctx).await;
    }
    let frames_overflow: Vec<String> = out_overflow.outgoing.into_iter().map(|f| f.frame).collect();
    assert!(
        frames_overflow.iter().any(|f| f == "F44402001D03"),
        "Inventory overflow must emit 1D 03: {:?}",
        frames_overflow
    );
    assert!(
        frames_overflow.iter().any(|f| f == "F44402001D08"),
        "Inventory overflow must unlock withdraw UI via 1D 08: {:?}",
        frames_overflow
    );
    assert!(
        frames_overflow.iter().any(|f| f == "F44402001A06"),
        "Inventory overflow MUST emit 1A 06 overflow error when bank has enough gold: {:?}",
        frames_overflow
    );

    // Also verify send_bank_ui_unlock helper directly
    let mut out_unlock_wdr = HandleOutcome::default();
    send_bank_ui_unlock(
        ts_dream::protocol::BANK_SUB_WITHDRAW,
        &mut out_unlock_wdr,
    );
    assert_eq!(out_unlock_wdr.outgoing.len(), 1);
    assert_eq!(out_unlock_wdr.outgoing[0].frame, "F44402001D08");

    let mut out_unlock_dep = HandleOutcome::default();
    send_bank_ui_unlock(
        ts_dream::protocol::BANK_SUB_DEPOSIT,
        &mut out_unlock_dep,
    );
    assert_eq!(out_unlock_dep.outgoing.len(), 1);
    assert_eq!(out_unlock_dep.outgoing[0].frame, "F44402001D07");

    // Invalid subcode for send_bank_ui_unlock must emit nothing
    let mut out_unlock_invalid = HandleOutcome::default();
    send_bank_ui_unlock(99, &mut out_unlock_invalid);
    assert_eq!(out_unlock_invalid.outgoing.len(), 0);

    // Case 3: Bank deposit overflow (bank_gold + amount > GOLD_CAP).
    // Player has 500_000, bank has 9_800_000, attempts to deposit 300_000.
    // Must receive 1D 03 and 1D 07, but MUST NOT receive 1A 06 (pocket overflow).
    conn.session.gold = 500_000;
    conn.session.bank_gold = 9_800_000;
    let mut out_dep_overflow = HandleOutcome::default();
    {
        let mut ctx = OpcodeCtx {
            conn: &mut conn,
            data: &data,
            service: &service,
            out: &mut out_dep_overflow,
            opcode: 0x1D,
            sub: ts_dream::protocol::BANK_SUB_DEPOSIT,
            payload: &300_000u32.to_le_bytes(),
            decoded: &[],
            env,
        };
        handle_bank_gold(&mut ctx).await;
    }
    let frames_dep_overflow: Vec<String> = out_dep_overflow.outgoing.into_iter().map(|f| f.frame).collect();
    assert!(
        frames_dep_overflow.iter().any(|f| f == "F44402001D03"),
        "Bank deposit overflow must emit 1D 03: {:?}",
        frames_dep_overflow
    );
    assert!(
        frames_dep_overflow.iter().any(|f| f == "F44402001D07"),
        "Bank deposit overflow must unlock deposit UI via 1D 07: {:?}",
        frames_dep_overflow
    );
    assert!(
        !frames_dep_overflow.iter().any(|f| f == "F44402001A06"),
        "Bank deposit overflow MUST NOT emit 1A 06 pocket error: {:?}",
        frames_dep_overflow
    );

    // Case 4: Amount == 0
    let mut out_zero = HandleOutcome::default();
    {
        let mut ctx = OpcodeCtx {
            conn: &mut conn,
            data: &data,
            service: &service,
            out: &mut out_zero,
            opcode: 0x1D,
            sub: ts_dream::protocol::BANK_SUB_WITHDRAW,
            payload: &0u32.to_le_bytes(),
            decoded: &[],
            env,
        };
        handle_bank_gold(&mut ctx).await;
    }
    let frames_zero: Vec<String> = out_zero.outgoing.into_iter().map(|f| f.frame).collect();
    assert!(frames_zero.iter().any(|f| f == "F44402001D03"));
    assert!(frames_zero.iter().any(|f| f == "F44402001D08"));
    assert!(!frames_zero.iter().any(|f| f == "F44402001A06"));

    // Case 5: Invalid subcode to handle_bank_gold is dropped silently
    let mut out_invalid = HandleOutcome::default();
    {
        let mut ctx = OpcodeCtx {
            conn: &mut conn,
            data: &data,
            service: &service,
            out: &mut out_invalid,
            opcode: 0x1D,
            sub: 99,
            payload: &50_000u32.to_le_bytes(),
            decoded: &[],
            env,
        };
        handle_bank_gold(&mut ctx).await;
    }
    assert_eq!(out_invalid.outgoing.len(), 0);
}


