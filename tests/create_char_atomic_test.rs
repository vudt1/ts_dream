//! Atomic character-creation tests (E3/E6/E7).
//!
//! - One-transaction create: characters row + money + password overwrite +
//!   starter inventories commit together (no half-created rows).
//! - Bear `initChar` parity: non-empty pass1/pass2 from the packet overwrite
//!   the dashboard passwords; empty passwords keep them.
//! - Homdo is NOT seeded; Trangbi slot 2 = 19737 is.
//! - Name race: second account reusing a taken name gets `09 03 01` and no row.
//! - `DOUBLE_LOGIN` == Bear `[00][19]` == SystemAlert 19.
//! - Enter-game short-circuit for charless authed sessions (no extra login).

use ts_dream::db::modern::sqlite::SqliteRepositories;
use ts_dream::db::pool::{bootstrap, DbPool};
use ts_dream::protocol::SystemAlertReason;
use ts_dream::server::dispatcher::{OpcodeCtx, ServerEnv};
use ts_dream::server::handlers::character::handle_character;
use ts_dream::server::handlers::login::handle_enter_game;
use ts_dream::server::session::{Conn, Session};
use ts_dream::server::spawn;

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
    let mut probe = Session::new();
    assert!(repos.sessions().load(acc, &mut probe).await.unwrap());
    assert_eq!(probe.name, b"AtomicHero");
    assert_eq!(probe.map_id, 10817);
    assert_eq!((probe.map_x, probe.map_y), (442, 758));
    assert_eq!(probe.hair, 3);
    assert!(
        probe.homdo.iter().any(|i| i.slot == 1 && i.id == 32012 && i.count == 4),
        "homdo (inventories type 1) must hold starter 32012x4, got {:?}",
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
