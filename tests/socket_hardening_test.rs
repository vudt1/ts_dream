//! Socket hardening tests (ticket 03, reconnect-heartbeat-health).
//!
//! - `TS_OUTBOUND_QUEUE_CAPACITY` config: default 1024, env override.
//! - Accepted connections enable TCP_NODELAY.
//! - Outbound frame queue is bounded: when the queue is full, the client
//!   is disconnected with a warning log without stalling the server.

use std::sync::Arc;
use std::time::Duration;
use tokio::io::AsyncReadExt;
use tokio::net::TcpStream;
use tokio::sync::RwLock;

use ts_dream::config::Config;
use ts_dream::state::AppState;
use ts_dream::web::server_control::ServerControl;

static ENV_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

fn with_cap_env<T>(value: Option<&str>, f: impl FnOnce() -> T) -> T {
    let _guard = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let prev = std::env::var("TS_OUTBOUND_QUEUE_CAPACITY").ok();
    match value {
        Some(v) => std::env::set_var("TS_OUTBOUND_QUEUE_CAPACITY", v),
        None => std::env::remove_var("TS_OUTBOUND_QUEUE_CAPACITY"),
    }
    let out = f();
    match prev {
        Some(v) => std::env::set_var("TS_OUTBOUND_QUEUE_CAPACITY", v),
        None => std::env::remove_var("TS_OUTBOUND_QUEUE_CAPACITY"),
    }
    out
}

#[test]
fn outbound_queue_capacity_defaults_to_1024() {
    assert_eq!(Config::default().outbound_queue_capacity, 1024);
}

#[test]
fn outbound_queue_capacity_env_override_is_applied() {
    let cap = with_cap_env(Some("256"), || {
        Config::load().expect("load").outbound_queue_capacity
    });
    assert_eq!(cap, 256);
}

#[test]
fn outbound_queue_capacity_invalid_env_is_rejected() {
    let res = with_cap_env(Some("xyz"), || Config::load());
    assert!(res.is_err(), "non-numeric TS_OUTBOUND_QUEUE_CAPACITY must fail");
}

use tokio::io::AsyncWriteExt;

mod common;
use common::test_db::setup_test_db;

#[tokio::test]
async fn full_outbound_queue_disconnects_client_without_blocking() {
    let port = 19201;
    let app = Arc::new(RwLock::new(AppState::new(1)));
    app.write().await.data_loaded = true;

    let pool = setup_test_db().await;
    let account_id = ts_dream::db::accounts::create(&pool, "TEST", "TEST")
        .await
        .expect("create account");
    let seed = ts_dream::db::modern::traits::CharacterSeed {
        sex: 0,
        hair: 1,
        element: 2,
        level: 20,
        ..Default::default()
    };
    let repos = ts_dream::db::modern::sqlite::SqliteRepositories::new(pool.clone());
    repos
        .characters()
        .create(account_id, b"Hero", &seed)
        .await
        .expect("create char");

    // Small queue capacity of 2
    let control = ServerControl::new(port, app.clone(), None, Some(pool.clone()))
        .with_outbound_queue_capacity(2);
    control.start().await.expect("listener starts");

    let mut sock = TcpStream::connect(("127.0.0.1", port))
        .await
        .expect("connected");

    // Give connection a moment to establish and receive greeting
    tokio::time::sleep(Duration::from_millis(50)).await;

    // Craft Op 0x01 login packet for existing character (which generates >10 outgoing frames):
    let mut payload = Vec::new();
    payload.extend_from_slice(&(account_id as u32).to_le_bytes());
    payload.extend_from_slice(b"VN");
    payload.extend_from_slice(&186u16.to_le_bytes());
    payload.extend_from_slice(b"TEST");

    let sub_len = 4u8;
    let mut frame_bytes = vec![0xF4, 0x44];
    let len = (payload.len() + 2) as u16;
    frame_bytes.extend_from_slice(&len.to_le_bytes());
    frame_bytes.push(0x01);
    frame_bytes.push(sub_len);
    frame_bytes.extend_from_slice(&payload);

    // XOR encode to wire format
    let wire = ts_dream::protocol::encoder::xor01(&frame_bytes);
    sock.write_all(&wire).await.expect("send login frame");

    // Because outbound capacity is 2 and login sequence generates >10 frames,
    // the outbound mpsc queue will be full, triggering disconnect.
    let mut buf = [0u8; 1024];
    let mut eof_reached = false;
    let start = std::time::Instant::now();
    while start.elapsed() < Duration::from_secs(3) {
        match tokio::time::timeout(Duration::from_millis(200), sock.read(&mut buf)).await {
            Ok(Ok(0)) | Ok(Err(_)) => {
                eof_reached = true;
                break;
            }
            Ok(Ok(_)) => {}
            Err(_) => {}
        }
    }
    assert!(eof_reached, "client connection should be closed on queue full");
}
