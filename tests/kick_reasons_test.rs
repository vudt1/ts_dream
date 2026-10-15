//! Kick system alert reasons tests (ticket 08, reconnect-heartbeat-health).
//!
//! - All server-initiated disconnect paths emit an alert reason frame before closing:
//!   - VersionMismatch (sub 10 = 0x0A)
//!   - AccountLocked (sub 30 = 0x1E)
//!   - DuplicateLoginOtherLocation (sub 19 = 0x13)
//!   - ProtectionDisconnect / GM kick (sub 28 = 0x1C)
//!   - ConnectionLost / idle timeout (sub 13 = 0x0D)
//! - Outbound alert frames reach the wire before TCP EOF.

use ts_dream::protocol::SystemAlertReason;
use ts_dream::server::dispatcher::{dispatch, ServerEnv};
use ts_dream::server::session::Conn;

#[test]
fn alert_reason_hex_values_match_spec() {
    assert_eq!(SystemAlertReason::VersionMismatch.to_hex(), "F4440200000A");
    assert_eq!(SystemAlertReason::AccountLocked.to_hex(), "F4440200001E");
    assert_eq!(SystemAlertReason::DuplicateLoginOtherLocation.to_hex(), "F44402000013");
    assert_eq!(SystemAlertReason::ProtectionDisconnect.to_hex(), "F4440200001C");
    assert_eq!(SystemAlertReason::ConnectionLost.to_hex(), "F4440200000D");
}

#[tokio::test]
async fn version_mismatch_sends_alert_and_shutdown() {
    let mut conn = Conn::new();
    let data = ts_dream::data::loader::GameData::default();
    let service = ts_dream::battle::service::BattleService::default();

    // Craft login with version 100 (< MIN_VERSION 186)
    let mut payload = Vec::new();
    payload.extend_from_slice(&70001u32.to_le_bytes());
    payload.extend_from_slice(b"VN");
    payload.extend_from_slice(&100u16.to_le_bytes()); // invalid version
    payload.extend_from_slice(b"PASS");

    let sub_len = 4u8;
    let mut frame_bytes = vec![0xF4, 0x44];
    let len = (payload.len() + 2) as u16;
    frame_bytes.extend_from_slice(&len.to_le_bytes());
    frame_bytes.push(0x01);
    frame_bytes.push(sub_len);
    frame_bytes.extend_from_slice(&payload);

    let out = dispatch(&mut conn, &frame_bytes, &data, &service, &ServerEnv::none()).await;

    assert!(out.shutdown, "version mismatch must request shutdown");
    let has_alert = out
        .outgoing
        .iter()
        .any(|f| f.frame == SystemAlertReason::VersionMismatch.to_hex());
    assert!(has_alert, "version mismatch must emit VersionMismatch alert");
}

use std::sync::Arc;
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::sync::RwLock;
use ts_dream::db::modern::sqlite::SqliteRepositories;
use ts_dream::db::modern::traits::CharacterSeed;
use ts_dream::db::pool::{bootstrap, DbPool};
use ts_dream::server::session::lock_online_sessions;
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

async fn read_all_frames_until_eof(sock: &mut TcpStream, deadline: Duration) -> (Vec<String>, bool) {
    let mut raw = Vec::new();
    let mut buf = [0u8; 1024];
    let start = std::time::Instant::now();
    let mut eof = false;
    while start.elapsed() < deadline {
        match tokio::time::timeout(Duration::from_millis(150), sock.read(&mut buf)).await {
            Ok(Ok(0)) | Ok(Err(_)) => {
                eof = true;
                break;
            }
            Ok(Ok(n)) => raw.extend_from_slice(&buf[..n]),
            Err(_) => {}
        }
    }
    let mut dec = ts_dream::protocol::frame::Decoder::new();
    let frames = dec.feed(&raw);
    (frames, eof)
}

#[tokio::test]
async fn gm_kick_sends_protection_disconnect_alert_and_closes_socket() {
    let port = 19301;
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
        .create(acc_id, b"KickMe", &seed)
        .await
        .expect("create char");

    let app = Arc::new(RwLock::new(AppState::new(1)));
    app.write().await.data_loaded = true;

    let control = ServerControl::new(port, app.clone(), None, Some(pool));
    control.start().await.expect("start server");

    let mut sock = TcpStream::connect(("127.0.0.1", port))
        .await
        .expect("connected");

    tokio::time::sleep(Duration::from_millis(50)).await;

    let login_wire = build_login_wire(acc_id as u32, b"PASS1");
    sock.write_all(&login_wire).await.expect("send login");

    // Wait until registered
    let mut registered = false;
    for _ in 0..30 {
        if control.clients.lock().await.contains_key(&(acc_id as u32)) {
            registered = true;
            break;
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    assert!(registered, "client must be registered online");

    // GM kicks player: sends ProtectionDisconnect alert frame and calls disconnect_player
    control
        .send_to(
            acc_id as u32,
            &SystemAlertReason::ProtectionDisconnect.to_hex(),
        )
        .await;
    control.disconnect_player(acc_id as u32).await;

    // Client must receive ProtectionDisconnect (F4440200001C) and then TCP EOF
    let (frames, eof) = read_all_frames_until_eof(&mut sock, Duration::from_secs(3)).await;
    assert!(eof, "socket must be closed (EOF) after GM kick");
    assert!(
        frames.iter().any(|f| f.eq_ignore_ascii_case("F4440200001C")),
        "frames must contain ProtectionDisconnect alert (0x00 sub 28): got {:?}",
        frames
    );

    // Registries must no longer contain this player
    assert!(!control.clients.lock().await.contains_key(&(acc_id as u32)));
    assert!(!lock_online_sessions().contains_key(&(acc_id as u32)));
}

#[tokio::test]
async fn suspended_account_sends_account_locked_alert_and_closes_socket() {
    let port = 19302;
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
        .create(acc_id, b"LockedMe", &seed)
        .await
        .expect("create char");

    // Suspend the account
    sqlx::query("UPDATE accounts SET issuspended = 1 WHERE playerid = ?")
        .bind(acc_id)
        .execute(&pool.write)
        .await
        .expect("suspend account");

    let app = Arc::new(RwLock::new(AppState::new(1)));
    app.write().await.data_loaded = true;

    let control = ServerControl::new(port, app.clone(), None, Some(pool));
    control.start().await.expect("start server");

    let mut sock = TcpStream::connect(("127.0.0.1", port))
        .await
        .expect("connected");

    tokio::time::sleep(Duration::from_millis(50)).await;

    let login_wire = build_login_wire(acc_id as u32, b"PASS1");
    sock.write_all(&login_wire).await.expect("send login");

    let (frames, eof) = read_all_frames_until_eof(&mut sock, Duration::from_secs(3)).await;
    assert!(eof, "socket must be closed (EOF) for suspended account");
    assert!(
        frames.iter().any(|f| f.eq_ignore_ascii_case("F4440200001E")),
        "frames must contain AccountLocked alert (0x00 sub 30): got {:?}",
        frames
    );
}

#[tokio::test]
async fn double_login_live_incumbent_sends_alert_and_closes_new_socket() {
    let port = 19303;
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
        .create(acc_id, b"DualMe", &seed)
        .await
        .expect("create char");

    let app = Arc::new(RwLock::new(AppState::new(1)));
    app.write().await.data_loaded = true;

    let control = ServerControl::new(port, app.clone(), None, Some(pool));
    control.start().await.expect("start server");

    // Client 1 connects and logs in
    let mut sock1 = TcpStream::connect(("127.0.0.1", port))
        .await
        .expect("connected 1");
    tokio::time::sleep(Duration::from_millis(50)).await;
    let login_wire = build_login_wire(acc_id as u32, b"PASS1");
    sock1.write_all(&login_wire).await.expect("send login 1");

    // Wait until registered
    let mut registered = false;
    for _ in 0..30 {
        if control.clients.lock().await.contains_key(&(acc_id as u32)) {
            registered = true;
            break;
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    assert!(registered, "client 1 must be registered online");

    // Client 2 connects and tries to login with the same account
    let mut sock2 = TcpStream::connect(("127.0.0.1", port))
        .await
        .expect("connected 2");
    tokio::time::sleep(Duration::from_millis(50)).await;
    sock2.write_all(&login_wire).await.expect("send login 2");

    let (frames2, eof2) = read_all_frames_until_eof(&mut sock2, Duration::from_secs(3)).await;
    assert!(eof2, "client 2 socket must be closed (EOF) on duplicate login");
    assert!(
        frames2.iter().any(|f| f.eq_ignore_ascii_case("F44402000013")),
        "client 2 must receive DuplicateLoginOtherLocation (0x00 sub 19): got {:?}",
        frames2
    );

    // Client 1 remains connected
    assert!(control.clients.lock().await.contains_key(&(acc_id as u32)));
}
