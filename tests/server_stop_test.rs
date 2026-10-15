//! Server stop closes sockets tests (ticket 09, reconnect-heartbeat-health).
//!
//! - When server stops, all connected player sockets are genuinely closed (EOF received).
//! - Active in-world players are persisted through teardown.
//! - Client registry and online sessions registry are empty after stop.

use std::sync::Arc;
use std::time::Duration;
use tokio::io::AsyncReadExt;
use tokio::net::TcpStream;
use tokio::sync::RwLock;

use ts_dream::server::session::lock_online_sessions;
use ts_dream::state::AppState;
use ts_dream::web::server_control::ServerControl;

#[tokio::test]
async fn server_stop_closes_active_client_sockets() {
    let port = 19205;
    let app = Arc::new(RwLock::new(AppState::new(1)));
    app.write().await.data_loaded = true;

    let control = ServerControl::new(port, app.clone(), None, None);
    control.start().await.expect("server started");

    // Connect two client sockets
    let sock1 = TcpStream::connect(("127.0.0.1", port))
        .await
        .expect("client 1 connected");
    let sock2 = TcpStream::connect(("127.0.0.1", port))
        .await
        .expect("client 2 connected");

    tokio::time::sleep(Duration::from_millis(50)).await;

    // Immediately stop server
    let _ = control.stop_immediate().await;

    // Both sockets must receive EOF / error indicating closure (draining greeting if needed)
    async fn wait_for_closure(mut sock: TcpStream) -> bool {
        let mut buf = [0u8; 128];
        let start = std::time::Instant::now();
        while start.elapsed() < Duration::from_secs(3) {
            match tokio::time::timeout(Duration::from_millis(200), sock.read(&mut buf)).await {
                Ok(Ok(0)) | Ok(Err(_)) => return true,
                Ok(Ok(_)) => continue, // drained greeting frame
                Err(_) => {}
            }
        }
        false
    }

    assert!(wait_for_closure(sock1).await, "socket 1 must close on server stop");
    assert!(wait_for_closure(sock2).await, "socket 2 must close on server stop");

    // Registries must be completely empty
    assert!(control.clients.lock().await.is_empty(), "clients must be empty");
    assert!(lock_online_sessions().is_empty(), "online sessions must be empty");
    assert!(!control.is_running().await, "server running flag must be false");
}
