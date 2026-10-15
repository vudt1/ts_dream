//! Idle-timeout kick tests (ticket 02, reconnect-heartbeat-health).
//!
//! Drive the real game listener over TCP with a shortened idle timeout:
//! - a silent socket receives a system-alert frame (opcode 0x00, sub 13 =
//!   ConnectionLost) before the server closes it (EOF);
//! - the alert bytes arrive intact (not lost to an early close);
//! - a player that was never logged in leaves no registration behind.
//!
//! No env is read: the timeout is injected via `ServerControl::with_idle_timeout`.
//! Tests use a dedicated port per test to avoid cross-test interference.

use std::sync::Arc;
use std::time::{Duration, Instant};

use tokio::io::AsyncReadExt;
use tokio::net::TcpStream;
use tokio::sync::RwLock;

use ts_dream::protocol::encoder;
use ts_dream::state::AppState;
use ts_dream::web::server_control::ServerControl;

const IDLE: Duration = Duration::from_millis(400);

/// Start a listener on `port` with a short idle timeout. Returns the control
/// handle so tests can inspect registrations.
async fn start_server(port: u16) -> (ServerControl, Arc<RwLock<AppState>>) {
    let app = Arc::new(RwLock::new(AppState::new(1)));
    app.write().await.data_loaded = true;
    let control = ServerControl::new(port, app.clone(), None, None)
        .with_idle_timeout(IDLE);
    control.start().await.expect("listener starts");
    (control, app)
}

async fn connect(port: u16) -> TcpStream {
    for _ in 0..50 {
        if let Ok(s) = TcpStream::connect(("127.0.0.1", port)).await {
            return s;
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    panic!("could not connect to 127.0.0.1:{port}");
}

/// Read everything until EOF (or the deadline). Returns the raw wire bytes.
async fn read_until_eof(stream: &mut TcpStream, deadline: Duration) -> (Vec<u8>, bool) {
    let mut all = Vec::new();
    let mut buf = [0u8; 1024];
    let start = Instant::now();
    loop {
        let remaining = deadline.saturating_sub(start.elapsed());
        if remaining.is_zero() {
            return (all, false);
        }
        match tokio::time::timeout(remaining, stream.read(&mut buf)).await {
            Ok(Ok(0)) => return (all, true),
            Ok(Ok(n)) => all.extend_from_slice(&buf[..n]),
            Ok(Err(_)) => return (all, true), // reset counts as closed
            Err(_) => return (all, false),
        }
    }
}

/// Decode the XOR-wire bytes into hex frames using the production decoder.
fn decode_frames(wire: &[u8]) -> Vec<String> {
    let mut dec = ts_dream::protocol::frame::Decoder::new();
    dec.feed(wire)
}

#[tokio::test]
async fn silent_socket_receives_connection_lost_alert_then_closes() {
    let port = 19101;
    let (_control, _app) = start_server(port).await;
    let mut sock = connect(port).await;

    // Greeting (login scene) arrives first; the rest is what matters.
    let (wire, closed) = read_until_eof(&mut sock, Duration::from_secs(5)).await;
    assert!(closed, "idle socket must be closed by the server");

    let frames = decode_frames(&wire);
    let alert = "F4440200000D"; // opcode 0x00, sub 13 (0x0D) = ConnectionLost
    assert!(
        frames.iter().any(|f| f.eq_ignore_ascii_case(alert)),
        "expected ConnectionLost alert {alert} among frames {frames:?}"
    );
    // The alert must be the last frame: nothing is written after the kick.
    assert_eq!(
        frames.last().map(|f| f.to_uppercase()).as_deref(),
        Some(alert),
        "alert must be the final frame before close"
    );
}

#[tokio::test]
async fn kick_frame_is_on_the_wire_before_eof() {
    // Guards against closing the socket before the queued alert is written:
    // the alert bytes must be present in the stream that ends in EOF.
    let port = 19102;
    let (_control, _app) = start_server(port).await;
    let mut sock = connect(port).await;

    let (wire, closed) = read_until_eof(&mut sock, Duration::from_secs(5)).await;
    assert!(closed);
    let expected = encoder::xor01(&encoder::bytes("F4440200000D").unwrap());
    assert!(
        wire.windows(expected.len()).any(|w| w == expected.as_slice()),
        "XOR-encoded alert bytes must appear in the byte stream before EOF"
    );
}

#[tokio::test]
async fn active_socket_is_not_kicked_while_sending_bytes() {
    // Any inbound byte refreshes the idle deadline. Send a 0x00 ACK every
    // 150 ms (< IDLE 400 ms) for longer than IDLE; the socket must stay open.
    let port = 19103;
    let (_control, _app) = start_server(port).await;
    let mut sock = connect(port).await;
    let ack = encoder::xor01(&encoder::bytes("F444010000").unwrap());

    use tokio::io::AsyncWriteExt;
    let start = Instant::now();
    while start.elapsed() < Duration::from_millis(1200) {
        sock.write_all(&ack).await.expect("write ACK");
        tokio::time::sleep(Duration::from_millis(150)).await;
    }

    // Still open: a read must time out, not hit EOF.
    let (_, closed) = read_until_eof(&mut sock, Duration::from_millis(100)).await;
    assert!(!closed, "keep-alive ACKs must prevent the idle kick");
}

#[tokio::test]
async fn unauthenticated_kick_leaves_no_registration() {
    let port = 19104;
    let (control, _app) = start_server(port).await;
    let mut sock = connect(port).await;
    let _ = read_until_eof(&mut sock, Duration::from_secs(5)).await;

    assert!(
        control.clients.lock().await.is_empty(),
        "no client may remain registered after an idle kick"
    );
}
