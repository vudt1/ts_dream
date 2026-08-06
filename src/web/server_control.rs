//! Server control manager (Chapter 7 §7.3 / §7.6).
//!
//! Controls game TCP server lifecycle: start, stop (with 5s 020C countdown),
//! announce broadcast, and active client connection tracking.

use crate::battle::service::BattleService;
use crate::data::loader::GameData;
use crate::protocol::encoder;
use crate::protocol::frame;
use crate::server::handler::{self, ServerEnv};
use crate::server::session::{online_sessions, Conn};
use crate::server::spawn::announce_frame;
use crate::state::AppState;
use axum::http::StatusCode;
use sqlx::MySqlPool;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::sync::{broadcast, mpsc, Mutex, RwLock};
use tokio::time::{sleep, Duration};

/// Handles sending packets to a connected client session.
pub type ClientSender = mpsc::UnboundedSender<String>;

#[derive(Clone)]
pub struct ServerControl {
    pub game_port: u16,
    pub app: Arc<RwLock<AppState>>,
    pub data: Option<Arc<GameData>>,
    pub pool: Option<MySqlPool>,
    pub clients: Arc<Mutex<HashMap<u32, ClientSender>>>,
    shutdown_tx: Arc<Mutex<Option<broadcast::Sender<()>>>>,
}

impl ServerControl {
    pub fn new(
        game_port: u16,
        app: Arc<RwLock<AppState>>,
        data: Option<Arc<GameData>>,
        pool: Option<MySqlPool>,
    ) -> Self {
        Self {
            game_port,
            app,
            data,
            pool,
            clients: Arc::new(Mutex::new(HashMap::new())),
            shutdown_tx: Arc::new(Mutex::new(None)),
        }
    }

    pub async fn is_running(&self) -> bool {
        self.app.read().await.running
    }

    /// Register a connected client sender.
    pub async fn register_client(&self, player_id: u32, sender: ClientSender) {
        self.clients.lock().await.insert(player_id, sender);
    }

    /// Unregister a client sender.
    pub async fn unregister_client(&self, player_id: u32) {
        self.clients.lock().await.remove(&player_id);
    }

    /// Start the game server listener on `game_port`.
    ///
    /// Gated on `AppState.data_loaded`: clients must never be accepted with
    /// empty/partial static data, so `start` refuses (and pushes a log) until
    /// the data load has fully finished.
    pub async fn start(&self) -> Result<bool, String> {
        let mut app = self.app.write().await;
        if app.running {
            return Ok(true);
        }
        if !app.data_loaded {
            let err_msg =
                "Game server cannot start: static data not loaded (DataLoaded=false)".to_string();
            app.push_log("error", err_msg.clone());
            return Err(err_msg);
        }

        let (shutdown_sender, _) = broadcast::channel::<()>(1);
        *self.shutdown_tx.lock().await = Some(shutdown_sender.clone());

        let addr = std::net::SocketAddr::from(([0, 0, 0, 0], self.game_port));
        let listener = match tokio::net::TcpListener::bind(addr).await {
            Ok(l) => l,
            Err(e) => {
                let err_msg = format!("Failed to bind port {}: {e}", self.game_port);
                app.push_log("error", err_msg.clone());
                return Err(err_msg);
            }
        };

        app.running = true;
        app.push_log(
            "system",
            format!(
                "Game server started listening on 0.0.0.0:{}",
                self.game_port
            ),
        );
        drop(app);

        let app_clone = self.app.clone();
        let data_clone = self.data.clone();
        let pool_clone = self.pool.clone();
        let control_clone = self.clone();
        let mut shutdown_rx = shutdown_sender.subscribe();

        tokio::spawn(async move {
            loop {
                tokio::select! {
                    accept_res = listener.accept() => {
                        match accept_res {
                            Ok((stream, peer)) => {
                                let app = app_clone.clone();
                                let data = data_clone.clone();
                                let pool = pool_clone.clone();
                                let control = control_clone.clone();
                                tokio::spawn(async move {
                                    handle_client_connection(stream, peer, app, data, pool, control).await;
                                });
                            }
                            Err(e) => {
                                tracing::warn!("accept error: {e}");
                            }
                        }
                    }
                    _ = shutdown_rx.recv() => {
                        tracing::info!("Game server TCP listener shut down.");
                        break;
                    }
                }
            }
        });

        Ok(true)
    }

    /// Stop the game server with 5s 020C countdown, kicking all clients & closing listener.
    /// Returns 409 Conflict if server is not running.
    pub async fn stop(&self) -> Result<bool, (StatusCode, String)> {
        let is_running = self.app.read().await.running;
        if !is_running {
            return Err((StatusCode::CONFLICT, "server not running".to_string()));
        }

        // 5-second countdown broadcasting 020C
        for count in (1..=5).rev() {
            let msg = format!("Server will be closed in {} second(s)", count);
            self.broadcast_packet(&announce_frame(&msg)).await;
            self.app.write().await.push_log("system", msg);
            sleep(Duration::from_secs(1)).await;
        }

        // Send shutdown signal to listener
        if let Some(tx) = self.shutdown_tx.lock().await.take() {
            let _ = tx.send(());
        }

        // Disconnect all clients
        let mut clients = self.clients.lock().await;
        clients.clear();
        drop(clients);

        let mut app = self.app.write().await;
        app.running = false;
        app.online.clear();
        app.push_log("system", "Game server stopped".to_string());

        Ok(false)
    }

    /// Broadcast announcement opcode 0x02 sub 0x0C to all connected clients.
    /// Returns 409 Conflict if server is not running.
    pub async fn announce(&self, text: &str) -> Result<(), (StatusCode, String)> {
        let is_running = self.app.read().await.running;
        if !is_running {
            return Err((StatusCode::CONFLICT, "server not running".to_string()));
        }

        let frame = announce_frame(text);
        self.broadcast_packet(&frame).await;
        self.app
            .write()
            .await
            .push_log("system", format!("Announcement sent: {}", text));

        Ok(())
    }

    /// Helper to send packet frame to all connected clients.
    pub async fn broadcast_packet(&self, hex_frame: &str) {
        let clients = self.clients.lock().await;
        for (player_id, tx) in clients.iter() {
            if let Err(_) = tx.send(hex_frame.to_string()) {
                tracing::debug!("Failed to send broadcast to player {player_id}");
            }
        }
    }

    /// Register a freshly-authenticated connection under `player_id`, atomically
    /// with the double-login check. Returns `false` (and keeps the map untouched)
    /// when the id is already online — the login must then be shut down.
    pub async fn login_register(&self, player_id: u32, sender: &ClientSender) -> bool {
        let mut clients = self.clients.lock().await;
        if clients.contains_key(&player_id) {
            return false;
        }
        clients.insert(player_id, sender.clone());
        true
    }

    /// Send `hex_frame` to every registered client except `from_id`
    /// (map-broadcast fan-out for move/expressions).
    pub async fn broadcast_except(&self, from_id: u32, hex_frame: &str) {
        let clients = self.clients.lock().await;
        for (player_id, tx) in clients.iter() {
            if *player_id != from_id {
                if let Err(_) = tx.send(hex_frame.to_string()) {
                    tracing::debug!("Failed to send broadcast to player {player_id}");
                }
            }
        }
    }

    /// Send `hex_frame` to one registered client. No-op when the player is
    /// offline (whisper/party/gold-item routing; C# `Server.SendToClient`).
    pub async fn send_to(&self, player_id: u32, hex_frame: &str) {
        let clients = self.clients.lock().await;
        if let Some(tx) = clients.get(&player_id) {
            if tx.send(hex_frame.to_string()).is_err() {
                tracing::debug!("Failed to send to player {player_id}");
            }
        }
    }

    /// Disconnect teardown for a logged-in session (Ch2 §2.1): broadcast the
    /// leave-battle / offline hide frame to peers, then drop the client
    /// registration and the online-session snapshot.
    pub async fn disconnect_player(&self, player_id: u32) {
        let hide = crate::server::spawn::session_offline_frame(player_id);
        self.broadcast_except(player_id, &hide).await;
        self.unregister_client(player_id).await;
        online_sessions().lock().unwrap().remove(&player_id);
    }
}

/// True when a decoded frame must be fanned out to other players on the same
/// map: move (op 0x06 sub 0x01) and expressions/actions (op 0x20).
fn is_map_broadcast(decoded: &[u8]) -> bool {
    match decoded.get(4).copied().unwrap_or(0) {
        0x06 => decoded.get(5).copied().unwrap_or(0) == 1,
        0x20 => true,
        _ => false,
    }
}

/// Connection handler for game TCP streams: frames inbound, dispatches opcodes,
/// writes outgoing frames, registers/logins the client and fans out
/// map-broadcast frames to peers.
async fn handle_client_connection(
    stream: TcpStream,
    peer: std::net::SocketAddr,
    app: Arc<RwLock<AppState>>,
    data: Option<Arc<GameData>>,
    pool: Option<MySqlPool>,
    control: ServerControl,
) {
    let peer_ip = peer.to_string();
    app.write()
        .await
        .push_log("system", format!("Client connected from {peer_ip}"));

    let data = data.unwrap_or_else(|| Arc::new(GameData::default()));
    let service = BattleService::new(Arc::clone(&data));

    let (mut read_half, mut write_half) = stream.into_split();
    let (tx, mut rx) = mpsc::unbounded_channel::<String>();

    let mut conn = Conn::new();
    let mut buf = vec![0u8; 8192];
    let mut logined_id = 0u32;

    // One teardown path for every exit (peer close, read error, handler
    // shutdown, write error): set `close` and break, then run the cleanup once.
    let mut close = false;
    while !close {
        tokio::select! {
            read_res = read_half.read(&mut buf) => {
                match read_res {
                    Ok(0) => close = true, // Peer closed (0-byte receive → shutdown)
                    Ok(n) => {
                        for frame_hex in conn.decoder.feed(&buf[..n]) {
                            let Some(decoded) = encoder::bytes(&frame_hex) else {
                                continue;
                            };
                            if !frame::check_magic(&decoded) {
                                tracing::warn!("dropping frame without F4 44 magic from {peer_ip}");
                                continue;
                            }
                            let env = ServerEnv {
                                pool: pool.as_ref(),
                                hub: Some(&control),
                                sender: Some(&tx),
                            };
                            // Pull the authoritative snapshot (a buyer may have
                            // mutated us through the player shop registry).
                            if logined_id > 0 {
                                if let Some(snapshot) =
                                    online_sessions().lock().unwrap().get(&logined_id).cloned()
                                {
                                    conn.session = snapshot;
                                }
                            }
                            let out = handler::dispatch(&mut conn, &decoded, &data, &service, &env).await;
                            let id = conn.session.id;
                            if logined_id > 0 {
                                online_sessions().lock().unwrap().insert(logined_id, conn.session.clone());
                            }
                            for f in &out.outgoing {
                                let _ = tx.send(f.clone());
                            }
                            if is_map_broadcast(&decoded) {
                                for f in &out.outgoing {
                                    control.broadcast_except(id, f).await;
                                }
                            }
                            if conn.session.logined && logined_id == 0 && id > 0 {
                                logined_id = id;
                            }
                            if out.shutdown {
                                app.write().await.push_log(
                                    "system",
                                    format!("Shutting down connection from {peer_ip}"),
                                );
                                close = true;
                                break;
                            }
                        }
                    }
                    Err(_) => close = true,
                }
            }
            Some(frame_hex) = rx.recv() => {
                if let Ok(wire) = frame::encode_to_wire(&frame_hex) {
                    if write_half.write_all(&wire).await.is_err() {
                        close = true;
                    }
                }
            }
        }
    }

    if logined_id > 0 {
        // Ch2 §2.1: a logged-in disconnect broadcasts the leave-battle +
        // offline hide frame to the map, then drops registration.
        control.disconnect_player(logined_id).await;
    }
    app.write()
        .await
        .push_log("system", format!("Client disconnected from {peer_ip}"));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::AppState;

    fn control() -> ServerControl {
        let app = Arc::new(RwLock::new(AppState::new(100)));
        ServerControl::new(6414, app, None, None)
    }

    #[tokio::test]
    async fn start_refuses_when_data_not_loaded() {
        let c = control(); // AppState::new() leaves data_loaded = false
        let r = c.start().await;
        assert!(
            r.is_err(),
            "must refuse to start when static data not loaded"
        );
        assert!(
            !c.app.read().await.running,
            "must not flip running on a refused start"
        );
    }

    #[tokio::test]
    async fn start_accepts_when_data_loaded() {
        let app = Arc::new(RwLock::new(AppState::new(100)));
        app.write().await.data_loaded = true;
        // Port 0 binds an ephemeral port, avoiding collisions with other tests.
        let c = ServerControl::new(0, app.clone(), None, None);

        let r = c.start().await;
        assert!(r.unwrap_or(false), "must start once static data is loaded");
        assert!(
            c.app.read().await.running,
            "running flag flips once started"
        );

        // Stop the accept loop cleanly (no 5s countdown in tests).
        if let Some(tx) = c.shutdown_tx.lock().await.take() {
            let _ = tx.send(());
        }
        c.app.write().await.running = false;
    }

    #[tokio::test]
    async fn login_register_is_atomic_double_login_guard() {
        let c = control();
        let (tx1, _rx1) = mpsc::unbounded_channel::<String>();
        let (tx2, _rx2) = mpsc::unbounded_channel::<String>();

        assert!(
            c.login_register(300001, &tx1).await,
            "first login registers"
        );
        assert!(
            !c.login_register(300001, &tx2).await,
            "second concurrent login is rejected (double-login guard)"
        );
        // The failed registration must not clobber the original sender.
        assert_eq!(c.clients.lock().await.len(), 1);
    }

    #[tokio::test]
    async fn broadcast_except_skips_origin() {
        let c = control();
        let (tx1, mut rx1) = mpsc::unbounded_channel::<String>();
        let (tx2, mut rx2) = mpsc::unbounded_channel::<String>();
        c.login_register(300001, &tx1).await;
        c.login_register(300002, &tx2).await;

        c.broadcast_except(300001, "F4440B000601FFFFFFFF026400C800")
            .await;

        assert_eq!(
            rx1.try_recv(),
            Err(mpsc::error::TryRecvError::Empty),
            "origin excluded"
        );
        assert_eq!(
            rx2.try_recv().unwrap(),
            "F4440B000601FFFFFFFF026400C800",
            "peer receives the broadcast"
        );
    }

    #[tokio::test]
    async fn disconnect_player_broadcasts_offline_and_unregisters() {
        let c = control();
        let (tx1, mut rx1) = mpsc::unbounded_channel::<String>();
        let (tx2, mut rx2) = mpsc::unbounded_channel::<String>();
        c.login_register(300001, &tx1).await;
        c.login_register(300002, &tx2).await;
        online_sessions()
            .lock()
            .unwrap()
            .insert(300001, Default::default());

        c.disconnect_player(300001).await;

        // Peers receive the leave/offline hide frame (Ch2 §2.1).
        assert_eq!(rx2.try_recv().unwrap(), "F44408000B00E19304000000");
        assert_eq!(
            rx1.try_recv(),
            Err(mpsc::error::TryRecvError::Empty),
            "origin gets nothing"
        );
        // Registration and online snapshot are dropped.
        assert_eq!(c.clients.lock().await.len(), 1);
        assert!(!online_sessions().lock().unwrap().contains_key(&300001));
    }
}
