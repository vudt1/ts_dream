//! Liveness foundation tests (ticket 01, reconnect-heartbeat-health).
//!
//! - `TS_IDLE_TIMEOUT_SECS` config: default 300s, env override.
//! - `Conn::last_seen` exists and is refreshed by the system-alert (0x00) handler.
//! - Client ACK frame `F444010000` (opcode 0x00) produces no response frame.
//!
//! The env test mutates process-global env, so every env-touching test holds
//! `ENV_LOCK` to stay serial. No DB is touched (`ServerEnv::none()`).

use std::sync::Mutex;
use std::time::{Duration, Instant};

use ts_dream::battle::service::BattleService;
use ts_dream::config::Config;
use ts_dream::data::loader::GameData;
use ts_dream::protocol::encoder;
use ts_dream::server::dispatcher::{dispatch, ServerEnv};
use ts_dream::server::handlers::system::handle_system_alert;
use ts_dream::server::session::Conn;

static ENV_LOCK: Mutex<()> = Mutex::new(());

fn with_idle_env<T>(value: Option<&str>, f: impl FnOnce() -> T) -> T {
    let _guard = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let prev = std::env::var("TS_IDLE_TIMEOUT_SECS").ok();
    match value {
        Some(v) => std::env::set_var("TS_IDLE_TIMEOUT_SECS", v),
        None => std::env::remove_var("TS_IDLE_TIMEOUT_SECS"),
    }
    let out = f();
    match prev {
        Some(v) => std::env::set_var("TS_IDLE_TIMEOUT_SECS", v),
        None => std::env::remove_var("TS_IDLE_TIMEOUT_SECS"),
    }
    out
}

#[test]
fn idle_timeout_defaults_to_300_seconds() {
    assert_eq!(Config::default().idle_timeout_secs, 300);
}

#[test]
fn idle_timeout_defaults_when_env_absent() {
    let secs = with_idle_env(None, || Config::load().expect("load").idle_timeout_secs);
    assert_eq!(secs, 300);
}

#[test]
fn idle_timeout_env_override_is_applied() {
    let secs = with_idle_env(Some("42"), || Config::load().expect("load").idle_timeout_secs);
    assert_eq!(secs, 42);
}

#[test]
fn idle_timeout_invalid_env_is_rejected() {
    let res = with_idle_env(Some("abc"), || Config::load());
    assert!(res.is_err(), "non-numeric TS_IDLE_TIMEOUT_SECS must fail");
}

#[test]
fn conn_last_seen_is_initialised_recently() {
    let before = Instant::now();
    let conn = Conn::new();
    let after = Instant::now();
    assert!(conn.last_seen >= before && conn.last_seen <= after);

    let conn = Conn::with_peer_ip("127.0.0.1");
    assert!(conn.last_seen <= Instant::now());
}

#[test]
fn system_alert_handler_refreshes_last_seen() {
    let data = GameData::default();
    let service = BattleService::default();
    let mut conn = Conn::new();
    let stale = Instant::now() - Duration::from_secs(10_000);
    conn.last_seen = stale;

    let mut out = ts_dream::server::dispatcher::HandleOutcome::default();
    let mut ctx = ts_dream::server::dispatcher::test_ctx(
        &mut conn,
        &data,
        &service,
        &mut out,
        0,
        &[],
    );
    handle_system_alert(&mut ctx);

    assert!(conn.last_seen > stale, "ACK must refresh last_seen");
}

#[tokio::test]
async fn ack_frame_f444010000_produces_no_response() {
    let data = GameData::default();
    let service = BattleService::default();
    let mut conn = Conn::new();
    let frame = encoder::bytes("F444010000").expect("valid hex");

    let out = dispatch(&mut conn, &frame, &data, &service, &ServerEnv::none()).await;

    assert!(out.outgoing.is_empty(), "ACK must not emit frames: {:?}", out.outgoing);
    assert!(out.map_broadcast.is_empty());
    assert!(out.direct_messages.is_empty());
    assert!(!out.shutdown, "ACK must not close the connection");
}

#[tokio::test]
async fn ack_frame_refreshes_last_seen_through_dispatch() {
    let data = GameData::default();
    let service = BattleService::default();
    let mut conn = Conn::new();
    let stale = Instant::now() - Duration::from_secs(10_000);
    conn.last_seen = stale;
    let frame = encoder::bytes("F444010000").expect("valid hex");

    let _ = dispatch(&mut conn, &frame, &data, &service, &ServerEnv::none()).await;

    assert!(conn.last_seen > stale, "dispatched ACK must refresh last_seen");
}
