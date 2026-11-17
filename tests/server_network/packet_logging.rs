//! Packet logging & Live SSE Terminal tests.
//!
//! Verifies:
//! 1. `packet_logging` config default is `false`.
//! 2. `TS_PACKET_LOGGING` environment variable overrides (`true`/`1`/`yes`/`on` vs `false`/`0`/`no`/`off`).
//! 3. Invalid `TS_PACKET_LOGGING` value is cleanly rejected.
//! 4. When `packet_logging` is disabled (`false`), neither C2S nor S2C packets are emitted to the SSE broadcast channel or buffer.
//! 5. When `packet_logging` is enabled (`true`), C2S and S2C packets are emitted to the SSE broadcast channel and buffer with proper level tags ("c2s", "s2c") and formatting `[{peer_ip}] {frame_hex}`.
//! 6. `drain_outbound` logs queued S2C frames during disconnect, and idle timeout logs with level `"warning"` (CSS parity).

use std::sync::Arc;
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::sync::RwLock;

use ts_dream::config::Config;
use ts_dream::protocol::SystemAlertReason;
use ts_dream::state::AppState;
use ts_dream::web::server_control::ServerControl;

fn with_packet_logging_env<T>(value: Option<&str>, f: impl FnOnce() -> T) -> T {
    let _guard = crate::common::env_lock();
    let prev = std::env::var("TS_PACKET_LOGGING").ok();
    match value {
        Some(v) => std::env::set_var("TS_PACKET_LOGGING", v),
        None => std::env::remove_var("TS_PACKET_LOGGING"),
    }
    let out = f();
    match prev {
        Some(v) => std::env::set_var("TS_PACKET_LOGGING", v),
        None => std::env::remove_var("TS_PACKET_LOGGING"),
    }
    out
}

#[test]
fn packet_logging_defaults_to_false() {
    assert!(!Config::default().packet_logging);
}

#[test]
fn packet_logging_env_override_truthy() {
    for truthy in &["1", "true", "yes", "on", "TRUE", "On"] {
        let enabled = with_packet_logging_env(Some(truthy), || {
            Config::load().expect("load").packet_logging
        });
        assert!(enabled, "expected true for {truthy}");
    }
}

#[test]
fn packet_logging_env_override_falsy() {
    for falsy in &["0", "false", "no", "off", "FALSE", "Off"] {
        let enabled = with_packet_logging_env(Some(falsy), || {
            Config::load().expect("load").packet_logging
        });
        assert!(!enabled, "expected false for {falsy}");
    }
}

#[test]
fn packet_logging_invalid_env_is_rejected() {
    let res = with_packet_logging_env(Some("not_a_bool"), || Config::load());
    assert!(res.is_err(), "non-boolean TS_PACKET_LOGGING must fail");
}

async fn start_test_server(port: u16, packet_logging: bool, idle: Option<Duration>) -> (ServerControl, Arc<RwLock<AppState>>) {
    let app = Arc::new(RwLock::new(AppState::new(1)));
    app.write().await.data_loaded = true;

    let mut control = ServerControl::new(port, app.clone(), None, None)
        .with_packet_logging(packet_logging);

    if let Some(d) = idle {
        control = control.with_idle_timeout(d);
    }

    control.start().await.expect("listener starts");
    (control, app)
}

async fn connect_to_server(port: u16) -> TcpStream {
    for _ in 0..50 {
        if let Ok(s) = TcpStream::connect(("127.0.0.1", port)).await {
            return s;
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    panic!("could not connect to 127.0.0.1:{port}");
}

/// Craft a login frame with version 100 (< MIN_VERSION 186) XOR-encoded for wire transmission.
/// This triggers incoming C2S processing and immediate outgoing S2C VersionMismatch alert.
fn craft_version_mismatch_wire() -> (Vec<u8>, String) {
    let mut payload = Vec::new();
    payload.extend_from_slice(&70001u32.to_le_bytes());
    payload.extend_from_slice(b"VN");
    payload.extend_from_slice(&100u16.to_le_bytes()); // version 100 < 186
    payload.extend_from_slice(b"PASS");

    let sub_len = 4u8;
    let mut frame_bytes = vec![0xF4, 0x44];
    let len = (payload.len() + 2) as u16;
    frame_bytes.extend_from_slice(&len.to_le_bytes());
    frame_bytes.push(0x01);
    frame_bytes.push(sub_len);
    frame_bytes.extend_from_slice(&payload);

    let hex = frame_bytes.iter().map(|b| format!("{b:02X}")).collect::<String>();
    let wire = frame_bytes.iter().map(|b| b ^ 0xAD).collect::<Vec<u8>>();
    (wire, hex)
}

#[tokio::test]
async fn live_packet_logging_disabled_emits_no_packet_logs() {
    let port = 19401;
    let (_control, app) = start_test_server(port, false, None).await;
    let mut rx = app.read().await.broadcast.subscribe();

    let mut sock = connect_to_server(port).await;
    let (wire, _hex) = craft_version_mismatch_wire();
    sock.write_all(&wire).await.expect("write wire");

    // Read the response from the server until close
    let mut resp = Vec::new();
    let _ = sock.read_to_end(&mut resp).await;

    // Check broadcast channel for any packet events
    tokio::time::sleep(Duration::from_millis(50)).await;
    while let Ok(evt) = rx.try_recv() {
        assert_ne!(evt.level, "c2s", "should not emit c2s when packet_logging=false");
        assert_ne!(evt.level, "s2c", "should not emit s2c when packet_logging=false");
    }

    // Check buffered logs
    let app_lock = app.read().await;
    for evt in &app_lock.log_buffer {
        assert_ne!(evt.level, "c2s", "log_buffer should not contain c2s");
        assert_ne!(evt.level, "s2c", "log_buffer should not contain s2c");
    }
}

#[tokio::test]
async fn live_packet_logging_enabled_emits_c2s_and_s2c() {
    let port = 19402;
    let (_control, app) = start_test_server(port, true, None).await;
    let mut rx = app.read().await.broadcast.subscribe();

    let mut sock = connect_to_server(port).await;
    let (wire, expected_c2s_hex) = craft_version_mismatch_wire();
    sock.write_all(&wire).await.expect("write wire");

    // Read server response (VersionMismatch alert)
    let mut resp = Vec::new();
    let _ = sock.read_to_end(&mut resp).await;

    tokio::time::sleep(Duration::from_millis(50)).await;

    let mut found_c2s = false;
    let mut found_s2c_greeting = false;
    let mut found_s2c_alert = false;
    let greeting_hex = ts_dream::server::spawn::LOGIN_SCENE_GREETING;
    let expected_s2c_hex = SystemAlertReason::VersionMismatch.to_hex();

    while let Ok(evt) = rx.try_recv() {
        if evt.level == "c2s" {
            assert!(evt.msg.contains("127.0.0.1"), "c2s msg should contain peer ip: {}", evt.msg);
            if evt.msg.contains(&expected_c2s_hex) {
                found_c2s = true;
            }
        } else if evt.level == "s2c" {
            assert!(evt.msg.contains("127.0.0.1"), "s2c msg should contain peer ip: {}", evt.msg);
            if evt.msg.contains(greeting_hex) {
                found_s2c_greeting = true;
            }
            if evt.msg.contains(&expected_s2c_hex) {
                found_s2c_alert = true;
            }
        }
    }

    assert!(found_c2s, "must have received c2s log event via SSE broadcast");
    assert!(found_s2c_greeting, "must have received s2c greeting log event via SSE broadcast");
    assert!(found_s2c_alert, "must have received s2c alert log event via SSE broadcast");

    // Also verify buffered in app.log_buffer
    let app_lock = app.read().await;
    let buf_c2s = app_lock.log_buffer.iter().any(|e| e.level == "c2s" && e.msg.contains(&expected_c2s_hex));
    let buf_s2c_greeting = app_lock.log_buffer.iter().any(|e| e.level == "s2c" && e.msg.contains(greeting_hex));
    let buf_s2c_alert = app_lock.log_buffer.iter().any(|e| e.level == "s2c" && e.msg.contains(&expected_s2c_hex));
    assert!(buf_c2s, "log_buffer must contain c2s event");
    assert!(buf_s2c_greeting, "log_buffer must contain s2c greeting event");
    assert!(buf_s2c_alert, "log_buffer must contain s2c alert event");
}

#[tokio::test]
async fn drain_outbound_logs_s2c_and_warning_on_idle_timeout() {
    let port = 19403;
    let short_idle = Duration::from_millis(200);
    let (_control, app) = start_test_server(port, true, Some(short_idle)).await;
    let mut rx = app.read().await.broadcast.subscribe();

    let mut sock = connect_to_server(port).await;

    // Sit idle and wait for server to close connection due to idle timeout
    let mut resp = Vec::new();
    let _ = sock.read_to_end(&mut resp).await;

    tokio::time::sleep(Duration::from_millis(50)).await;

    let mut found_s2c_alert = false;
    let mut found_warning = false;
    let expected_s2c_hex = SystemAlertReason::ConnectionLost.to_hex();

    while let Ok(evt) = rx.try_recv() {
        if evt.level == "s2c" && evt.msg.contains(&expected_s2c_hex) {
            found_s2c_alert = true;
        }
        if evt.level == "warning" && evt.msg.contains("Idle timeout") {
            found_warning = true;
        }
        // Ensure "warn" (bug) is not emitted
        assert_ne!(evt.level, "warn", "must use 'warning' level instead of 'warn' for CSS parity");
    }

    assert!(found_s2c_alert, "drain_outbound must log s2c ConnectionLost alert");
    assert!(found_warning, "idle timeout must log with level 'warning'");
}

#[tokio::test]
async fn sse_log_event_json_structure_matches_dashboard_contract() {
    let mut app = AppState::new(1);
    app.push_log("c2s", "[127.0.0.1:1234] F444010000".to_string());

    let event = app.log_buffer.back().expect("log event").clone();
    let json_str = serde_json::to_string(&event).expect("serialize log event");
    let json_val: serde_json::Value = serde_json::from_str(&json_str).expect("parse json");

    assert_eq!(json_val["level"], "c2s");
    assert!(json_val["ts"].is_u64());
    assert_eq!(json_val["msg"], "[127.0.0.1:1234] F444010000");
}

#[tokio::test]
async fn broadcast_channel_capacity_is_1024() {
    let mut app = AppState::new(1);
    let mut rx = app.broadcast.subscribe();

    for i in 0..1024 {
        app.push_log("packet", format!("packet {i}"));
    }

    // All 1024 messages must be readable without lagging error
    for i in 0..1024 {
        let msg = rx.try_recv().expect("recv within capacity");
        assert_eq!(msg.msg, format!("packet {i}"));
    }
}

