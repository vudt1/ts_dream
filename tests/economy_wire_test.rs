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
    complete_shop_buy, gold_frame, handle_npc_shop, player_shop_catalog_frame, ShopBuyError,
};
use ts_dream::server::handlers::trade_storage::{
    cancel_timed_out_trade, expire_trade_if_timed_out, handle_bank_gold, handle_storage_transfer,
    handle_trade, is_trade_timed_out, TRADE_TIMEOUT_MS,
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
// Test 1: Currency Model & Login Bank Sync (Ticket 02)
// =========================================================================
#[test]
fn test_currency_model_and_login_bank_sync() {
    let mut session = Session::default();
    session.id = 1001;
    session.gold = 500_000;
    session.bank_gold = 1_200_000;
    session.shop_point = 88_888;

    let frames = build_logined_sequence_session(&session);

    // 1. Verify 1A 04 frame contains gold and shop_point in 2nd DWORD
    let expected_gold_frame = format!(
        "F4440A001A04{}{}",
        encoder::le32(500_000),
        encoder::le32(88_888)
    );
    assert!(
        frames.iter().any(|f| f == &expected_gold_frame),
        "spawn frames must contain 1A 04 with gold and shop_point: expected {}",
        expected_gold_frame
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

    let expected_capped_gold = format!(
        "F4440A001A04{}{}",
        encoder::le32(GOLD_CAP),
        encoder::le32(1_500_000_000)
    );
    assert!(
        capped_frames.iter().any(|f| f == &expected_capped_gold),
        "gold must be clamped to GOLD_CAP: expected {}",
        expected_capped_gold
    );

    let expected_capped_bank = format!("F44406001D04{}", encoder::le32(GOLD_CAP));
    assert!(
        capped_frames.iter().any(|f| f == &expected_capped_bank),
        "bank_gold must be clamped to GOLD_CAP: expected {}",
        expected_capped_bank
    );

    // 4. Test gold_frame helper
    let gf = gold_frame(capped_session.gold, capped_session.shop_point);
    assert_eq!(gf, expected_capped_gold);

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

    let expected_sync = format!(
        "F4440A001A04{}{}",
        encoder::le32(970),
        encoder::le32(50)
    );
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

    let expected_sync = format!(
        "F4440A001A04{}{}",
        encoder::le32(150),
        encoder::le32(10)
    );
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
// Test 5: Bank Deposit & Withdrawal Atomic Operations (Ticket 03)
// =========================================================================
#[tokio::test]
async fn test_bank_deposit_withdrawal_atomic() {
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
            sub: 2, // Deposit
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
            sub: 1, // Withdraw
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
        dep_frames.iter().any(|f| f.starts_with("F4440E001E04")),
        "Must send 1E 04 to add item to bank storage"
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
    // Must be 1E 05 01 0F, NOT 1E 05 01 32!
    assert!(
        wdr_frames.iter().any(|f| f == "F44404001E05010F"),
        "1E 05 remove frame must carry exact item count 15 (0x0F), NOT hardcoded 32: found {:?}",
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
        ll_wdr_frames.iter().any(|f| f == "F44404001768012A"),
        "17 68 frame for Luulang must carry exact item count 42 (0x2A), NOT hardcoded 32"
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
            payload: &[0x02], // action == 2 (Cancel)
            decoded: &[],
            env,
        };
        handle_trade(&mut ctx).await;
    }

    let cancel_frames: Vec<String> = out_cancel.outgoing.into_iter().map(|f| f.frame).collect();
    assert!(
        cancel_frames.iter().any(|f| f == "F4440200173B"),
        "Trade cancel must emit 17 3B to close trade form"
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

    // 4. Test Pet Trade Cancellation (action == 2): Sub 12
    let mut out_pet_cancel = HandleOutcome::default();
    {
        let mut ctx = OpcodeCtx {
            conn: &mut conn1,
            data: &data,
            service: &service,
            out: &mut out_pet_cancel,
            opcode: 0x19,
            sub: 12,
            payload: &[0x02], // action == 2 (Cancel)
            decoded: &[],
            env,
        };
        handle_trade(&mut ctx).await;
    }

    let pet_cancel_frames: Vec<String> = out_pet_cancel.outgoing.into_iter().map(|f| f.frame).collect();
    assert!(
        pet_cancel_frames.iter().any(|f| f == "F4440200173B"),
        "Pet trade cancel must emit 17 3B to close trade form"
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
        to_frames.iter().any(|f| f == "F4440200173B"),
        "Timed-out trade must emit 17 3B to close trade form"
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
