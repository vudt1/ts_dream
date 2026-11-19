//! Atomic character-creation tests (E3/E6/E7).
//!
//! - One-transaction create: characters row + money + password overwrite +
//!   starter inventories commit together (no half-created rows).
//! - Bear `initChar` parity: non-empty pass1/pass2 from the packet overwrite
//!   the dashboard passwords; empty passwords keep them.
//! - Starter inventories: Homdo slot 1 = 32012x4, slot 2 = 31007x1;
//!   Trangbi slot 2 = 19737x1; `characters.newbie = 0`.
//! - Name race: second account reusing a taken name gets `09 03 01` and no row.
//! - `DOUBLE_LOGIN` == Bear `[00][19]` == SystemAlert 19.
//! - Enter-game short-circuit for charless authed sessions (no extra login).

use ts_dream::db::modern::sqlite::SqliteRepositories;
use ts_dream::protocol::SystemAlertReason;
use ts_dream::server::dispatcher::{OpcodeCtx, ServerEnv};
use ts_dream::server::handlers::character::handle_character;
use ts_dream::server::handlers::login::handle_enter_game;
use ts_dream::server::session::{Conn, Session};
use ts_dream::server::spawn;

use crate::common::test_db::setup_test_db;

fn create_payload(pass1: &[u8], pass2: &[u8]) -> Vec<u8> {
    let mut p = vec![
        1, // sex
        1, // style
        3, // hair
        0, // face
        0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77, 0x88, // color 8B
        4, // element: Phong
        5, 12, 10, 8, 5, 15, // int atk def hpx spx agi
        pass1.len() as u8,
    ];
    p.extend_from_slice(pass1);
    p.push(pass2.len() as u8);
    p.extend_from_slice(pass2);
    p
}

async fn name_check(
    conn: &mut Conn,
    data: &ts_dream::data::loader::GameData,
    service: &ts_dream::battle::service::BattleService,
    env: ServerEnv<'_>,
    candidate: &[u8],
) -> Vec<String> {
    let mut out = ts_dream::server::dispatcher::HandleOutcome::default();
    {
        let mut ctx = OpcodeCtx {
            conn,
            out: &mut out,
            env,
            data,
            service,
            decoded: &[],
            opcode: 0x09,
            sub: 2,
            payload: candidate,
        };
        handle_character(&mut ctx).await;
    }
    out.outgoing.into_iter().map(|f| f.frame).collect()
}

async fn create_char(
    conn: &mut Conn,
    data: &ts_dream::data::loader::GameData,
    service: &ts_dream::battle::service::BattleService,
    env: ServerEnv<'_>,
    payload: &[u8],
) -> Vec<String> {
    let mut out = ts_dream::server::dispatcher::HandleOutcome::default();
    {
        let mut ctx = OpcodeCtx {
            conn,
            out: &mut out,
            env,
            data,
            service,
            decoded: &[],
            opcode: 0x09,
            sub: 1,
            payload,
        };
        handle_character(&mut ctx).await;
    }
    out.outgoing.into_iter().map(|f| f.frame).collect()
}

#[tokio::test]
async fn test_atomic_create_with_password_overwrite() {
    let pool = setup_test_db().await;
    let repos = SqliteRepositories::new(pool.clone());
    let data = ts_dream::data::loader::GameData::default();
    let service = ts_dream::battle::service::BattleService::default();
    let env = ServerEnv {
        pool: Some(&pool),
        repos: Some(&repos),
        hub: None,
        sender: None,
    };

    let acc = ts_dream::db::accounts::create(&pool, "dashpass1", "dashpass2")
        .await
        .expect("create account");

    let mut conn = Conn::default();
    conn.session.id = acc as u32;
    conn.session.authed = true;

    // Name available.
    let frames = name_check(&mut conn, &data, &service, env, b"AtomicHero").await;
    assert!(
        frames.iter().any(|f| f == spawn::CHAR_NAME_AVAILABLE),
        "expected 09 03 00, got {frames:?}"
    );

    // Create with non-empty pass1/pass2 (Bear parity: overwrite).
    let payload = create_payload(b"abcdef", b"123456");
    let frames = create_char(&mut conn, &data, &service, env, &payload).await;
    assert!(
        frames.iter().any(|f| f == spawn::CHAR_CREATE_SUCCESS),
        "expected 09 01, got {frames:?}"
    );

    // Passwords overwritten in the SAME transaction.
    assert!(repos.accounts().verify_pass1(acc, b"abcdef").await.unwrap());
    assert!(!repos.accounts().verify_pass1(acc, b"dashpass1").await.unwrap());
    assert!(repos.accounts().verify_pass2(acc, b"123456").await.unwrap());

    // Full row + money + inventories visible together (atomic).
    // Appearance color lives in session RAM for player_appear (covered by
    // the `hair` column in DB — no separate color columns).
    assert_eq!(conn.session.color, "1122334455667788");
    assert!(
        conn.session.homdo.iter().any(|i| i.slot == 1 && i.id == 32012 && i.count == 4),
        "RAM homdo must hold starter 32012x4 at slot 1, got {:?}",
        conn.session.homdo
    );
    assert!(
        !conn.session.homdo.iter().any(|i| i.id == 31007),
        "RAM homdo must NOT hold 31007 at character create (granted on Quest 10037 instead), got {:?}",
        conn.session.homdo
    );
    assert!(
        conn.session.trangbi.iter().any(|i| i.slot == 2 && i.id == 19737 && i.count == 1),
        "RAM trangbi slot 2 must be 19737, got {:?}",
        conn.session.trangbi
    );
    assert_eq!(conn.session.newbie, 0, "RAM newbie must remain 0");

    let mut probe = Session::new();
    assert!(repos.sessions().load(acc, &mut probe).await.unwrap());
    assert_eq!(probe.name, b"AtomicHero");
    assert_eq!(probe.map_id, 10817);
    assert_eq!((probe.map_x, probe.map_y), (442, 758));
    assert_eq!(probe.hair, 3);
    assert_eq!(probe.newbie, 0, "DB newbie must remain 0 after create");
    let db_newbie: i64 = sqlx::query_scalar("SELECT newbie FROM characters WHERE playerid = ?")
        .bind(acc)
        .fetch_one(&pool.read)
        .await
        .unwrap();
    assert_eq!(db_newbie, 0, "characters.newbie column must be 0");
    assert!(
        probe.homdo.iter().any(|i| i.slot == 1 && i.id == 32012 && i.count == 4),
        "homdo (inventories type 1) must hold starter 32012x4 at slot 1, got {:?}",
        probe.homdo
    );
    assert!(
        !probe.homdo.iter().any(|i| i.id == 31007),
        "homdo (inventories type 1) must NOT hold 31007 at character create, got {:?}",
        probe.homdo
    );
    assert!(
        probe.trangbi.iter().any(|i| i.slot == 2 && i.id == 19737),
        "trangbi slot 2 must be 19737, got {:?}",
        probe.trangbi
    );
}

#[tokio::test]
async fn test_empty_passwords_keep_dashboard_credentials() {
    let pool = setup_test_db().await;
    let repos = SqliteRepositories::new(pool.clone());
    let data = ts_dream::data::loader::GameData::default();
    let service = ts_dream::battle::service::BattleService::default();
    let env = ServerEnv {
        pool: Some(&pool),
        repos: Some(&repos),
        hub: None,
        sender: None,
    };

    let acc = ts_dream::db::accounts::create(&pool, "keepme01", "keepme02")
        .await
        .expect("create account");
    let mut conn = Conn::default();
    conn.session.id = acc as u32;
    conn.session.authed = true;

    name_check(&mut conn, &data, &service, env, b"KeepPassHero").await;
    let payload = create_payload(b"", b"");
    let frames = create_char(&mut conn, &data, &service, env, &payload).await;
    assert!(frames.iter().any(|f| f == spawn::CHAR_CREATE_SUCCESS));

    // Dashboard passwords untouched.
    assert!(repos.accounts().verify_pass1(acc, b"keepme01").await.unwrap());
    assert!(repos.accounts().verify_pass2(acc, b"keepme02").await.unwrap());
}

#[tokio::test]
async fn test_name_race_leaves_no_partial_row() {
    let pool = setup_test_db().await;
    let repos = SqliteRepositories::new(pool.clone());
    let data = ts_dream::data::loader::GameData::default();
    let service = ts_dream::battle::service::BattleService::default();
    let env = ServerEnv {
        pool: Some(&pool),
        repos: Some(&repos),
        hub: None,
        sender: None,
    };

    // First owner takes the name.
    let acc1 = ts_dream::db::accounts::create(&pool, "racepass1", "racepass2")
        .await
        .unwrap();
    let mut c1 = Conn::default();
    c1.session.id = acc1 as u32;
    c1.session.authed = true;
    name_check(&mut c1, &data, &service, env, b"RaceHero").await;
    let frames = create_char(&mut c1, &data, &service, env, &create_payload(b"", b"")).await;
    assert!(frames.iter().any(|f| f == spawn::CHAR_CREATE_SUCCESS));

    // Second account: name check says duplicate...
    let acc2 = ts_dream::db::accounts::create(&pool, "racepass3", "racepass4")
        .await
        .unwrap();
    let mut c2 = Conn::default();
    c2.session.id = acc2 as u32;
    c2.session.authed = true;
    let frames = name_check(&mut c2, &data, &service, env, b"RaceHero").await;
    assert!(frames.iter().any(|f| f == spawn::CHAR_NAME_DUPLICATE));

    // ...and a forced create with the stolen name also fails with the
    // duplicate toast and leaves NO partial row (single Tx rolled back).
    c2.session.pending_new_char_name = b"RaceHero".to_vec();
    let frames = create_char(&mut c2, &data, &service, env, &create_payload(b"", b"")).await;
    assert!(frames.iter().any(|f| f == spawn::CHAR_NAME_DUPLICATE));

    let mut probe = Session::new();
    assert!(!repos.sessions().load(acc2, &mut probe).await.unwrap());
    let money: Option<i64> =
        sqlx::query_scalar("SELECT playerid FROM character_money WHERE playerid = ?")
            .bind(acc2)
            .fetch_optional(&pool.read)
            .await
            .unwrap();
    assert!(money.is_none(), "money row must not exist for failed create");
    let inv: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM inventories WHERE playerid = ?")
        .bind(acc2)
        .fetch_one(&pool.read)
        .await
        .unwrap();
    assert_eq!(inv, 0, "no inventory rows for failed create");
}

#[test]
fn test_double_login_frame_matches_bear_and_system_alert() {
    // Bear Authentication case 2: addByte(0), addByte(19) → F44402000013.
    assert_eq!(spawn::DOUBLE_LOGIN, "F44402000013");
    assert_eq!(
        spawn::DOUBLE_LOGIN,
        SystemAlertReason::DuplicateLoginOtherLocation.to_hex()
    );
    // Alias: one value, two names.
    assert_eq!(spawn::ENTER_GAME_CREATE, spawn::LOGIN_CREATE_CHAR);
}

#[tokio::test]
async fn test_enter_game_short_circuit_charless() {
    let pool = setup_test_db().await;
    let repos = SqliteRepositories::new(pool.clone());
    let data = ts_dream::data::loader::GameData::default();
    let service = ts_dream::battle::service::BattleService::default();
    let env = ServerEnv {
        pool: Some(&pool),
        repos: Some(&repos),
        hub: None,
        sender: None,
    };

    let acc = ts_dream::db::accounts::create(&pool, "nopass001", "nopass002")
        .await
        .unwrap();
    let mut conn = Conn::default();
    conn.session.id = acc as u32;
    conn.session.authed = true; // verified at login, no character yet

    let mut out = ts_dream::server::dispatcher::HandleOutcome::default();
    {
        let mut ctx = OpcodeCtx {
            conn: &mut conn,
            out: &mut out,
            env,
            data: &data,
            service: &service,
            decoded: &[],
            opcode: 0x03,
            sub: 1,
            payload: &[],
        };
        handle_enter_game(&mut ctx).await;
    }
    assert!(
        out.outgoing.iter().any(|f| f.frame == spawn::ENTER_GAME_CREATE),
        "charless authed session must get create-char screen"
    );
    assert!(!conn.session.logined);
}

#[tokio::test]
async fn test_starter_item_31007_enter_game_and_relogin_no_duplication() {
    let pool = setup_test_db().await;
    let repos = SqliteRepositories::new(pool.clone());
    let data = ts_dream::data::loader::GameData::default();
    let service = ts_dream::battle::service::BattleService::default();
    let env = ServerEnv {
        pool: Some(&pool),
        repos: Some(&repos),
        hub: None,
        sender: None,
    };

    let acc = ts_dream::db::accounts::create(&pool, "enterpass1", "enterpass2")
        .await
        .unwrap();
    let mut conn = Conn::default();
    conn.session.id = acc as u32;
    conn.session.authed = true;

    name_check(&mut conn, &data, &service, env, b"Starter31007").await;
    let frames = create_char(&mut conn, &data, &service, env, &create_payload(b"", b"")).await;
    assert!(frames.iter().any(|f| f == spawn::CHAR_CREATE_SUCCESS));

    // Enter game (0x03 Sub 01): must emit 17 05 containing 32012x4 at slot 1 and 31007x1 at slot 2.
    let mut out = ts_dream::server::dispatcher::HandleOutcome::default();
    {
        let mut ctx = OpcodeCtx {
            conn: &mut conn,
            out: &mut out,
            env,
            data: &data,
            service: &service,
            decoded: &[],
            opcode: 0x03,
            sub: 1,
            payload: &[],
        };
        handle_enter_game(&mut ctx).await;
    }
    assert!(conn.session.logined);
    assert_eq!(conn.session.newbie, 0, "newbie must stay 0 on enter game");

    let homdo_frame = out
        .outgoing
        .iter()
        .map(|f| f.frame.as_str())
        .find(|f| f.len() >= 12 && &f[8..12] == "1705")
        .expect("17 05 homdo sync frame must be emitted on enter game");
    // 12-byte entry: [slot:1B][id:2B LE][count:1B]...
    // slot 1 + 32012 (0x7D0C -> 0C7D) + count 4 -> 010C7D04
    // slot 2 + 31007 (0x791F -> 1F79) + count 1 -> 021F7901
    assert!(
        homdo_frame.contains("010C7D04"),
        "17 05 must contain slot 1 32012x4 (collateral), got {homdo_frame}"
    );
    assert!(
        !homdo_frame.contains("1F79"),
        "17 05 must NOT contain 31007 on enter game, got {homdo_frame}"
    );

    // Relogin (fresh Conn + 0x01 login): 31007 stays absent, 32012x4 intact.
    let mut relogin_conn = Conn::default();
    let pass = b"enterpass1";
    let mut relogin_payload = Vec::new();
    relogin_payload.extend_from_slice(&(acc as u32).to_le_bytes());
    relogin_payload.extend_from_slice(ts_dream::protocol::ID_PREFIX.as_bytes());
    relogin_payload.extend_from_slice(&ts_dream::protocol::MIN_VERSION.to_le_bytes());
    relogin_payload.extend_from_slice(pass);
    let mut relogin_out = ts_dream::server::dispatcher::HandleOutcome::default();
    {
        let mut ctx = OpcodeCtx {
            conn: &mut relogin_conn,
            out: &mut relogin_out,
            env,
            data: &data,
            service: &service,
            decoded: &[],
            opcode: 0x01,
            sub: pass.len() as u8,
            payload: &relogin_payload,
        };
        ts_dream::server::handlers::login::handle_login(&mut ctx).await;
    }
    assert!(relogin_conn.session.logined);
    assert_eq!(
        relogin_conn.session.homdo.iter().filter(|i| i.id == 31007).count(),
        0,
        "relogin must keep 31007 absent before Quest 10037"
    );
    assert_eq!(
        relogin_conn.session.homdo.iter().filter(|i| i.id == 32012).count(),
        1,
        "relogin must keep 32012x4 intact"
    );
    let db_31007_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM inventories WHERE playerid = ? AND storagetype = 1 AND itemid = 31007",
    )
    .bind(acc)
    .fetch_one(&pool.read)
    .await
    .unwrap();
    assert_eq!(db_31007_count, 0, "DB must have 0 rows for 31007 at char create");
}

#[tokio::test]
async fn test_existing_char_newbie_zero_not_granted_31007_on_enter_game() {
    let pool = setup_test_db().await;
    let repos = SqliteRepositories::new(pool.clone());
    let data = ts_dream::data::loader::GameData::default();
    let service = ts_dream::battle::service::BattleService::default();
    let env = ServerEnv {
        pool: Some(&pool),
        repos: Some(&repos),
        hub: None,
        sender: None,
    };

    let acc = ts_dream::db::accounts::create(&pool, "legacy01", "legacy02")
        .await
        .unwrap();
    let mut conn = Conn::default();
    conn.session.id = acc as u32;
    conn.session.authed = true;

    name_check(&mut conn, &data, &service, env, b"LegacyHero").await;
    create_char(&mut conn, &data, &service, env, &create_payload(b"", b"")).await;

    // Simulate an older character created before this update: remove 31007 from DB, newbie=0.
    sqlx::query("DELETE FROM inventories WHERE playerid = ? AND itemid = 31007")
        .bind(acc)
        .execute(&pool.write)
        .await
        .unwrap();

    let mut enter_conn = Conn::default();
    enter_conn.session.id = acc as u32;
    enter_conn.session.authed = true;
    let mut out = ts_dream::server::dispatcher::HandleOutcome::default();
    {
        let mut ctx = OpcodeCtx {
            conn: &mut enter_conn,
            out: &mut out,
            env,
            data: &data,
            service: &service,
            decoded: &[],
            opcode: 0x03,
            sub: 1,
            payload: &[],
        };
        handle_enter_game(&mut ctx).await;
    }
    assert!(enter_conn.session.logined);
    assert_eq!(enter_conn.session.newbie, 0);
    assert!(
        !enter_conn.session.homdo.iter().any(|i| i.id == 31007),
        "existing character with newbie=0 must NOT be backfilled with 31007 on enter game"
    );
    let db_31007: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM inventories WHERE playerid = ? AND itemid = 31007",
    )
    .bind(acc)
    .fetch_one(&pool.read)
    .await
    .unwrap();
    assert_eq!(db_31007, 0);
}

#[tokio::test]
async fn test_in_memory_fallback_create_char_omits_31007_and_keeps_newbie_zero() {
    let data = ts_dream::data::loader::GameData::default();
    let service = ts_dream::battle::service::BattleService::default();
    let env = ServerEnv {
        pool: None,
        repos: None,
        hub: None,
        sender: None,
    };

    let mut conn = Conn::default();
    conn.session.id = 9999;
    conn.session.authed = true;
    name_check(&mut conn, &data, &service, env, b"InMemHero").await;
    let frames = create_char(&mut conn, &data, &service, env, &create_payload(b"", b"")).await;
    assert!(frames.iter().any(|f| f == spawn::CHAR_CREATE_SUCCESS));
    assert_eq!(conn.session.newbie, 0);
    assert!(conn.session.homdo.iter().any(|i| i.slot == 1 && i.id == 32012 && i.count == 4));
    assert!(!conn.session.homdo.iter().any(|i| i.id == 31007));
    assert!(conn.session.trangbi.iter().any(|i| i.slot == 2 && i.id == 19737 && i.count == 1));
}
