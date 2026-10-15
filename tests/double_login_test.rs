//! Double login tests (ticket 04, reconnect-heartbeat-health).
//!
//! - Incumbent session still live -> second login rejected (DOUBLE_LOGIN frame).
//! - Incumbent session dead (idle timeout elapsed) -> evicted cleanly (persisted + unmounted)
//!   and second login succeeds.
//! - Concurrent logins for same player id are atomic and race-free.

use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{mpsc, RwLock};

use ts_dream::db::modern::sqlite::SqliteRepositories;
use ts_dream::db::pool::{bootstrap, DbPool};
use ts_dream::server::session::{lock_online_sessions, Session};
use ts_dream::state::AppState;
use ts_dream::web::server_control::ServerControl;

async fn setup_test_db() -> DbPool {
    let pool = bootstrap("sqlite::memory:?cache=shared", None)
        .await
        .expect("bootstrap SQLite");
    let schema = include_str!("../migrations/0001_init.sql");
    sqlx::raw_sql(schema)
        .execute(&pool.write)
        .await
        .expect("init schema");
    pool
}

#[tokio::test]
async fn live_incumbent_rejects_second_login() {
    let app = Arc::new(RwLock::new(AppState::new(1)));
    let control = ServerControl::new(19202, app, None, None)
        .with_idle_timeout(Duration::from_secs(300));

    let player_id = 50001;
    let (tx1, _rx1) = mpsc::channel(16);
    let (tx2, _rx2) = mpsc::channel(16);

    // First login succeeds
    let ok1 = control.login_register(player_id, &tx1).await;
    assert!(ok1, "first login must succeed");

    // Second login while first is fresh must be rejected
    let ok2 = control.login_register(player_id, &tx2).await;
    assert!(!ok2, "second login while live must be rejected");
}

#[tokio::test]
async fn dead_incumbent_is_evicted_and_new_login_succeeds() {
    let pool = setup_test_db().await;
    let _repos = SqliteRepositories::new(pool.clone());
    let app = Arc::new(RwLock::new(AppState::new(1)));
    let control = ServerControl::new(19203, app, None, Some(pool.clone()))
        .with_idle_timeout(Duration::from_millis(100));

    let player_id = 50002;
    let (tx1, _rx1) = mpsc::channel(16);
    let (tx2, _rx2) = mpsc::channel(16);

    // Initial session in online map
    let mut sess = Session {
        id: player_id,
        authed: true,
        name: b"OldChar".to_vec(),
        gold: 100,
        ..Default::default()
    };
    sess.db_character_id = player_id as i64;
    lock_online_sessions().insert(player_id, sess);

    // First login
    assert!(control.login_register(player_id, &tx1).await);

    // Wait for idle timeout to elapse on incumbent
    tokio::time::sleep(Duration::from_millis(150)).await;
    assert!(!control.is_client_live(player_id).await, "incumbent must be dead after timeout");

    // Second login should evict dead incumbent and succeed
    let ok2 = control.login_register(player_id, &tx2).await;
    assert!(ok2, "second login after incumbent timeout must succeed");
}

#[tokio::test]
async fn concurrent_logins_never_race() {
    let app = Arc::new(RwLock::new(AppState::new(1)));
    let control = Arc::new(
        ServerControl::new(19204, app, None, None)
            .with_idle_timeout(Duration::from_secs(300)),
    );

    let player_id = 50003;
    let (tx1, _rx1) = mpsc::channel(16);
    let (tx2, _rx2) = mpsc::channel(16);

    let c1 = control.clone();
    let c2 = control.clone();

    let (res1, res2) = tokio::join!(
        c1.login_register(player_id, &tx1),
        c2.login_register(player_id, &tx2),
    );

    // Exactly one must succeed, one must fail
    assert_ne!(res1, res2, "exactly one concurrent login must succeed");
}

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use ts_dream::db::modern::traits::CharacterSeed;

fn build_login_wire(player_id: u32, pass: &[u8]) -> Vec<u8> {
    let mut payload = Vec::new();
    payload.extend_from_slice(&player_id.to_le_bytes());
    payload.extend_from_slice(b"VN");
    payload.extend_from_slice(&186u16.to_le_bytes());
    payload.extend_from_slice(pass);

    let sub_len = pass.len() as u8;
    let mut frame_bytes = vec![0xF4, 0x44];
    let len = (payload.len() + 2) as u16;
    frame_bytes.extend_from_slice(&len.to_le_bytes());
    frame_bytes.push(0x01);
    frame_bytes.push(sub_len);
    frame_bytes.extend_from_slice(&payload);
    ts_dream::protocol::encoder::xor01(&frame_bytes)
}

#[tokio::test]
async fn dead_incumbent_socket_is_closed_over_tcp_when_evicted() {
    let port = 19305;
    let pool = setup_test_db().await;
    let acc_id = ts_dream::db::accounts::create(&pool, "PASS1", "PASS2")
        .await
        .expect("create account");
    let seed = CharacterSeed {
        sex: 0,
        hair: 1,
        element: 1,
        level: 25,
        ..Default::default()
    };
    let repos = SqliteRepositories::new(pool.clone());
    repos
        .characters()
        .create(acc_id, b"EvictMe", &seed)
        .await
        .expect("create char");

    let app = Arc::new(RwLock::new(AppState::new(1)));
    app.write().await.data_loaded = true;

    // Short idle timeout: 200 ms
    let control = ServerControl::new(port, app.clone(), None, Some(pool))
        .with_idle_timeout(Duration::from_millis(200));
    control.start().await.expect("start server");

    // Client 1 connects and logs in
    let mut sock1 = TcpStream::connect(("127.0.0.1", port))
        .await
        .expect("connected 1");
    tokio::time::sleep(Duration::from_millis(50)).await;
    let login_wire = build_login_wire(acc_id as u32, b"PASS1");
    sock1.write_all(&login_wire).await.expect("send login 1");

    // Wait until client 1 is registered
    let mut registered = false;
    for _ in 0..30 {
        if control.clients.lock().await.contains_key(&(acc_id as u32)) {
            registered = true;
            break;
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    assert!(registered, "client 1 must be registered online");

    // Wait 250 ms so client 1 exceeds idle timeout (becomes dead incumbent)
    tokio::time::sleep(Duration::from_millis(250)).await;
    assert!(
        !control.is_client_live(acc_id as u32).await,
        "client 1 must be dead after idle timeout"
    );

    // Client 2 connects and logs in with the same account -> evicts dead incumbent
    let mut sock2 = TcpStream::connect(("127.0.0.1", port))
        .await
        .expect("connected 2");
    tokio::time::sleep(Duration::from_millis(50)).await;
    sock2.write_all(&login_wire).await.expect("send login 2");

    // Client 1's socket must be closed (EOF) by the eviction teardown
    let mut buf = [0u8; 1024];
    let start = std::time::Instant::now();
    let mut sock1_eof = false;
    while start.elapsed() < Duration::from_secs(3) {
        match tokio::time::timeout(Duration::from_millis(150), sock1.read(&mut buf)).await {
            Ok(Ok(0)) | Ok(Err(_)) => {
                sock1_eof = true;
                break;
            }
            Ok(Ok(_)) => {}
            Err(_) => {}
        }
    }
    assert!(sock1_eof, "client 1 socket must be closed (EOF) upon eviction");

    // Client 2 should now be registered online
    let mut c2_registered = false;
    for _ in 0..30 {
        if control.clients.lock().await.contains_key(&(acc_id as u32)) {
            c2_registered = true;
            break;
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    assert!(c2_registered, "client 2 must succeed and be registered online");
}
