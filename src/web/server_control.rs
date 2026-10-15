//! Server control manager (Chapter 7 §7.3 / §7.6).
//!
//! Controls game TCP server lifecycle: start, stop (with 5s 020C countdown),
//! announce broadcast, and active client connection tracking.

use crate::battle::service::BattleService;
use crate::data::loader::GameData;
use crate::db::modern::sqlite::SqliteRepositories;
use crate::db::pool::DbPool;
use crate::protocol::encoder;
use crate::protocol::frame;
use crate::protocol::SystemAlertReason;
use crate::server::dispatcher::{self, ServerEnv};
use crate::server::session::{lock_online_sessions, Conn};
use crate::server::spawn::announce_frame;
use crate::state::AppState;
use axum::http::StatusCode;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::sync::{broadcast, mpsc, Mutex, RwLock};
use tokio::time::{sleep, Duration};

/// Handles sending packets to a connected client session.
pub type ClientSender = mpsc::Sender<String>;

#[derive(Clone)]
pub struct ServerControl {
    pub game_port: u16,
    pub app: Arc<RwLock<AppState>>,
    pub data: Option<Arc<GameData>>,
    pub pool: Option<DbPool>,
    pub clients: Arc<Mutex<HashMap<u32, ClientSender>>>,
    pub client_activity: Arc<Mutex<HashMap<u32, std::time::Instant>>>,
    pub battle_service: Arc<BattleService>,
    /// Shutdown signal for the accept loop (`Some` while listening). Exposed
    /// so lifecycle owners (and tests) can halt the listener without the 5s
    /// countdown performed by [`ServerControl::stop`].
    pub shutdown_tx: Arc<Mutex<Option<broadcast::Sender<()>>>>,
    /// Broadcast channel signalling all active client connections to close immediately.
    pub conn_shutdown_tx: broadcast::Sender<()>,
    /// Broadcast channel signalling a specific player connection to close (GM kick / dead eviction).
    pub player_disconnect_tx: broadcast::Sender<u32>,
    /// Silent-connection idle timeout (ticket 02). Defaults to
    /// `Config::default().idle_timeout_secs`; `main` overrides it from config.
    pub idle_timeout: Duration,
    /// Bounded queue capacity for per-client outbound frames (ticket 03).
    pub outbound_queue_capacity: usize,
}

impl ServerControl {
    pub fn new(
        game_port: u16,
        app: Arc<RwLock<AppState>>,
        data: Option<Arc<GameData>>,
        pool: Option<DbPool>,
    ) -> Self {
        let gd = data.clone().unwrap_or_else(|| Arc::new(GameData::default()));
        let mut svc = BattleService::new(gd);
        if let Some(p) = pool.as_ref() {
            svc = svc.with_pool(p.clone());
        }
        let battle_service = Arc::new(svc);
        // CP6 #4: let `battle_ended` start a chained Eve battle through the
        // weak back-reference (the sink has no spawn machinery of its own).
        BattleService::install_eve_backref(&battle_service);
        let (conn_shutdown_tx, _) = broadcast::channel(16);
        let (player_disconnect_tx, _) = broadcast::channel(64);
        Self {
            game_port,
            app,
            data,
            pool,
            clients: Arc::new(Mutex::new(HashMap::new())),
            client_activity: Arc::new(Mutex::new(HashMap::new())),
            battle_service,
            shutdown_tx: Arc::new(Mutex::new(None)),
            conn_shutdown_tx,
            player_disconnect_tx,
            idle_timeout: Duration::from_secs(crate::config::Config::default().idle_timeout_secs),
            outbound_queue_capacity: crate::config::Config::default().outbound_queue_capacity,
        }
    }

    /// Override the idle timeout (used by config wiring and tests).
    pub fn with_idle_timeout(mut self, idle: Duration) -> Self {
        self.idle_timeout = idle;
        self
    }

    /// Override the outbound frame queue capacity (used by config wiring and tests).
    pub fn with_outbound_queue_capacity(mut self, cap: usize) -> Self {
        self.outbound_queue_capacity = cap;
        self
    }

    pub async fn is_running(&self) -> bool {
        self.app.read().await.running
    }

    /// Register a connected client sender.
    pub async fn register_client(&self, player_id: u32, sender: ClientSender) {
        self.clients.lock().await.insert(player_id, sender);
        self.client_activity
            .lock()
            .await
            .insert(player_id, std::time::Instant::now());
    }

    /// Unregister a client sender.
    pub async fn unregister_client(&self, player_id: u32) {
        self.clients.lock().await.remove(&player_id);
        self.client_activity.lock().await.remove(&player_id);
        self.battle_service.unregister(i64::from(player_id));
    }

    /// Touch a client's last active time.
    pub async fn touch_client(&self, player_id: u32) {
        self.client_activity
            .lock()
            .await
            .insert(player_id, std::time::Instant::now());
    }

    /// Check if a registered client is still live (not exceeded idle timeout).
    pub async fn is_client_live(&self, player_id: u32) -> bool {
        let clients = self.clients.lock().await;
        if !clients.contains_key(&player_id) {
            return false;
        }
        let activity = self.client_activity.lock().await;
        match activity.get(&player_id) {
            Some(last_seen) => last_seen.elapsed() < self.idle_timeout,
            None => false,
        }
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
        let idle = self.idle_timeout;
        let cap = self.outbound_queue_capacity;
        let mut shutdown_rx = shutdown_sender.subscribe();

        tokio::spawn(async move {
            loop {
                tokio::select! {
                    accept_res = listener.accept() => {
                        match accept_res {
                            Ok((stream, peer)) => {
                                if let Err(e) = stream.set_nodelay(true) {
                                    tracing::debug!(peer_ip = %peer, "failed to set TCP_NODELAY: {e}");
                                }
                                let app = app_clone.clone();
                                let data = data_clone.clone();
                                let pool = pool_clone.clone();
                                let control = control_clone.clone();
                                tokio::spawn(async move {
                                    handle_client_connection(stream, peer, app, data, pool, control, idle, cap).await;
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

    /// Immediately stop server: shuts listener, sends close to connection handlers,
    /// disconnects & persists all sessions, checkpoints WAL, and clears online state.
    pub async fn stop_immediate(&self) -> Result<bool, (StatusCode, String)> {
        let is_running = self.app.read().await.running;
        if !is_running {
            return Err((StatusCode::CONFLICT, "server not running".to_string()));
        }

        // Send shutdown signal to listener
        if let Some(tx) = self.shutdown_tx.lock().await.take() {
            let _ = tx.send(());
        }

        // Broadcast shutdown to all active connection handlers
        let _ = self.conn_shutdown_tx.send(());

        // Disconnect and persist all registered players
        let player_ids: Vec<u32> = {
            let clients = self.clients.lock().await;
            clients.keys().copied().collect()
        };
        for pid in player_ids {
            self.disconnect_player(pid).await;
        }

        self.clients.lock().await.clear();
        self.client_activity.lock().await.clear();
        crate::server::session::lock_online_sessions().clear();

        if let Some(pool) = self.pool.as_ref() {
            let saved = crate::server::auto_save::save_all_dirty(pool).await;
            if saved > 0 {
                tracing::info!("Server stop: auto-saved {saved} dirty session(s)");
            }
            if let Err(e) = pool.checkpoint().await {
                tracing::warn!("Server stop: WAL checkpoint failed: {e}");
            } else {
                tracing::info!("Server stop: WAL checkpoint (TRUNCATE) completed");
            }
        }

        let mut app = self.app.write().await;
        app.running = false;
        app.online.clear();
        app.push_log("system", "Game server stopped".to_string());

        Ok(false)
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

        self.stop_immediate().await
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
            if tx.try_send(hex_frame.to_string()).is_err() {
                tracing::debug!("Failed to send broadcast to player {player_id}");
            }
        }
    }

    /// Register a freshly-authenticated connection under `player_id`, atomically
    /// with the double-login check. Returns `false` (and keeps the map untouched)
    /// when the id is already online — the login must then be shut down.
    /// If the incumbent connection is dead (idle timeout elapsed), it is evicted.
    pub async fn login_register(&self, player_id: u32, sender: &ClientSender) -> bool {
        let mut clients = self.clients.lock().await;
        if clients.contains_key(&player_id) {
            let is_dead = {
                let activity = self.client_activity.lock().await;
                match activity.get(&player_id) {
                    Some(last_seen) => last_seen.elapsed() >= self.idle_timeout,
                    None => true,
                }
            };
            if !is_dead {
                return false;
            }
            tracing::info!(
                player_id,
                "evicting dead incumbent session exceeding idle timeout"
            );
            drop(clients);
            self.disconnect_player(player_id).await;
            clients = self.clients.lock().await;
            if clients.contains_key(&player_id) {
                return false;
            }
        }
        clients.insert(player_id, sender.clone());
        self.client_activity
            .lock()
            .await
            .insert(player_id, std::time::Instant::now());
        if let Some(sess) = lock_online_sessions().get(&player_id).cloned() {
            self.battle_service.register_sender(
                i64::from(player_id),
                Arc::new(tokio::sync::RwLock::new(sess)),
                sender.clone(),
            );
        }
        true
    }

    /// Send `hex_frame` to every registered client except `from_id` (used by
    /// chat/inventory/shop handlers that broadcast directly).
    pub async fn broadcast_except(&self, from_id: u32, hex_frame: &str) {
        let clients = self.clients.lock().await;
        for (player_id, tx) in clients.iter() {
            if *player_id != from_id && tx.try_send(hex_frame.to_string()).is_err() {
                tracing::debug!("Failed to send broadcast to player {player_id}");
            }
        }
    }

    /// Map-scoped fan-out for move/expression frames.
    pub async fn broadcast_map(&self, from_id: u32, frames: &[dispatcher::MapBroadcast]) {
        let targets: Vec<(u32, String)> = {
            let sessions = lock_online_sessions();
            let Some(from) = sessions.get(&from_id) else {
                return; // no map scope → nothing to fan out
            };
            let mut result = Vec::new();
            for b in frames {
                let target_map = b.map_id.unwrap_or(from.map_id);
                for (pid, s) in sessions.iter() {
                    if *pid != from_id && *pid != b.subject && s.map_id == target_map {
                        result.push((*pid, b.frame.clone()));
                    }
                }
            }
            result
        };
        if targets.is_empty() {
            return;
        }
        let clients = self.clients.lock().await;
        for (pid, frame) in targets {
            if let Some(tx) = clients.get(&pid) {
                if tx.try_send(frame).is_err() {
                    tracing::debug!("Failed to send map broadcast to player {pid}");
                }
            }
        }
    }

    /// Send `hex_frame` to one registered client. No-op when the player is
    /// offline (whisper/party/gold-item routing).
    pub async fn send_to(&self, player_id: u32, hex_frame: &str) {
        let clients = self.clients.lock().await;
        if let Some(tx) = clients.get(&player_id) {
            if tx.try_send(hex_frame.to_string()).is_err() {
                tracing::debug!("Failed to send to player {player_id}");
            }
        }
    }

    /// Disconnect teardown for a logged-in session (Ch2 §2.1): broadcast the
    /// leave-battle / offline hide frame to peers, persist state to database,
    /// then drop the client registration and the online-session snapshot.
    pub async fn disconnect_player(&self, player_id: u32) {
        let _ = self.player_disconnect_tx.send(player_id);
        let hide = crate::server::spawn::session_offline_frame(player_id);
        self.broadcast_except(player_id, &hide).await;
        self.unregister_client(player_id).await;
        crate::server::auto_save::forget_player(player_id);
        let session_opt = lock_online_sessions().remove(&player_id);
        if let Some(session) = session_opt {
            if session.authed && session.id > 0 {
                let _ = crate::db::persist::persist_sessions_transaction(
                    self.pool.as_ref(),
                    &[&session],
                    &["stats", "homdo", "trangbi", "quest", "pet"],
                )
                .await;
            }
        }
    }
}

/// Write every frame already queued on `rx` to `write_half`, in order. Used
/// before a deliberate close so queued frames (e.g. the kick alert) reach the
/// socket instead of being dropped with the channel.
async fn drain_outbound(
    rx: &mut mpsc::Receiver<String>,
    write_half: &mut tokio::net::tcp::OwnedWriteHalf,
) {
    while let Ok(frame_hex) = rx.try_recv() {
        if let Ok(wire) = frame::encode_to_wire(&frame_hex) {
            if write_half.write_all(&wire).await.is_err() {
                break;
            }
        }
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
    pool: Option<DbPool>,
    control: ServerControl,
    idle: Duration,
    outbound_capacity: usize,
) {
    let peer_ip = peer.to_string();
    app.write()
        .await
        .push_log("system", format!("Client connected from {peer_ip}"));

    let data = data.unwrap_or_else(|| Arc::new(GameData::default()));
    let repos = pool
        .as_ref()
        .map(|pool| SqliteRepositories::new(pool.clone()));
    let service = Arc::clone(&control.battle_service);

    let (mut read_half, mut write_half) = stream.into_split();
    let (tx, mut rx) = mpsc::channel::<String>(outbound_capacity);
    let mut conn_shutdown_rx = control.conn_shutdown_tx.subscribe();
    let mut player_disc_rx = control.player_disconnect_tx.subscribe();

    let mut conn = Conn::with_peer_ip(peer.ip().to_string());
    let mut buf = vec![0u8; 8192];
    let mut logined_id = 0u32;

    // TS Online Client (aLogin.exe) hoàn toàn thụ động chờ Server gửi gói tin đầu tiên.
    // Gửi ngay gói tin chào mừng chuyển sang Login Scene (Op 0x01, Sub 0x09, Scene 90 / 0x5A).
    let _ = tx.try_send(crate::server::spawn::LOGIN_SCENE_GREETING.to_string());

    // One teardown path for every exit (peer close, read error, handler
    // shutdown, write error): set `close` and break, then run the cleanup once.
    // Idle deadline (ticket 02): pinned outside the loop so the timer survives
    // across `select!` iterations; reset on every inbound byte.
    let idle_timer = sleep(idle);
    tokio::pin!(idle_timer);

    let mut close = false;
    while !close {
        tokio::select! {
            _ = conn_shutdown_rx.recv() => {
                tracing::debug!(peer_ip = %peer_ip, "server stopping: closing connection");
                close = true;
            }
            disc_res = player_disc_rx.recv() => {
                match disc_res {
                    Ok(disc_id) => {
                        if logined_id > 0 && disc_id == logined_id {
                            tracing::debug!(peer_ip = %peer_ip, player_id = disc_id, "player kicked/evicted: closing connection");
                            close = true;
                        }
                    }
                    Err(broadcast::error::RecvError::Lagged(_)) => {
                        if logined_id > 0 && !control.clients.lock().await.contains_key(&logined_id) {
                            tracing::debug!(peer_ip = %peer_ip, player_id = logined_id, "player missing from clients after lag: closing connection");
                            close = true;
                        }
                    }
                    Err(broadcast::error::RecvError::Closed) => {
                        close = true;
                    }
                }
            }
            read_res = read_half.read(&mut buf) => {
                match read_res {
                    Ok(0) => close = true, // Peer closed (0-byte receive → shutdown)
                    Ok(n) => {
                        // Liveness: any byte read from the socket counts as activity.
                        conn.last_seen = std::time::Instant::now();
                        idle_timer.as_mut().reset(tokio::time::Instant::now() + idle);
                        if logined_id > 0 {
                            control.touch_client(logined_id).await;
                        }
                        for frame_hex in conn.decoder.feed(&buf[..n]) {
                            let Some(decoded) = encoder::bytes(&frame_hex) else {
                                continue;
                            };
                            if !frame::check_magic(&decoded) {
                                tracing::warn!("dropping frame without F4 44 magic from {peer_ip}");
                                continue;
                            }
                            let mut operation_ids = vec![logined_id];
                            if decoded.get(4) == Some(&0x17) && decoded.get(5) == Some(&33) {
                                operation_ids.push(conn.session.open_shop_id);
                            }
                            if decoded.get(4) == Some(&0x19) {
                                let sub = decoded.get(5).copied().unwrap_or(0);
                                let partner = if matches!(sub, 1 | 10) {
                                    decoded.get(6..10).and_then(|p| p.try_into().ok()).map(u32::from_le_bytes).unwrap_or(0)
                                } else if sub == 20 {
                                    decoded.get(10..14).and_then(|p| p.try_into().ok()).map(u32::from_le_bytes).unwrap_or(0)
                                } else {
                                    conn.session.trade.partner_id
                                };
                                operation_ids.push(partner);
                            }
                            let _operation_guards =
                                crate::server::session::lock_player_operations(operation_ids).await;
                            let env = ServerEnv {
                                pool: pool.as_ref(),
                                repos: repos.as_ref(),
                                hub: Some(&control),
                                sender: Some(&tx),
                            };
                            // Pull the authoritative snapshot (a buyer may have
                            // mutated us through the player shop registry).
                            if logined_id > 0 {
                                if let Some(snapshot) =
                                    lock_online_sessions().get(&logined_id).cloned()
                                {
                                    conn.session = snapshot;
                                }
                            }
                            let out = dispatcher::dispatch(&mut conn, &decoded, &data, &service, &env).await;
                            let id = conn.session.id;
                            if logined_id > 0 {
                                lock_online_sessions()
                                    .insert(logined_id, conn.session.clone());
                                service.register_sender(
                                    i64::from(logined_id),
                                    Arc::new(tokio::sync::RwLock::new(conn.session.clone())),
                                    tx.clone(),
                                );
                            }
                            for frame in &out.outgoing {
                                // Dialog fragments are paced (500 ms between
                                // dialog splits); honor it on the
                                // live connection, never blocking the runtime.
                                if frame.delay_ms > 0 {
                                    tokio::time::sleep(Duration::from_millis(frame.delay_ms))
                                        .await;
                                }
                                match tx.try_send(frame.frame.clone()) {
                                    Ok(()) => {}
                                    Err(mpsc::error::TrySendError::Full(_)) => {
                                        tracing::warn!(
                                            peer_ip = %peer_ip,
                                            "client outbound queue full, disconnecting client"
                                        );
                                        close = true;
                                        break;
                                    }
                                    Err(mpsc::error::TrySendError::Closed(_)) => {
                                        close = true;
                                        break;
                                    }
                                }
                            }
                            if close {
                                break;
                            }
                            // CP6 #3: a scripted Eve battle starts only after
                            // this outcome's own frames are queued — the
                            // battle's start frames then land behind them in
                            // the same client channel, so talk frames are
                            // never reordered behind battle frames.
                            if let Some((fight_id, diahinh)) = out.eve_battle {
                                service.start_eve_encounter(&mut conn.session, fight_id, diahinh);
                                if logined_id > 0 {
                                    lock_online_sessions()
                                        .insert(logined_id, conn.session.clone());
                                    service.register_sender(
                                        i64::from(logined_id),
                                        Arc::new(tokio::sync::RwLock::new(conn.session.clone())),
                                        tx.clone(),
                                    );
                                }
                            }
                            if !out.map_broadcast.is_empty() {
                                control.broadcast_map(id, &out.map_broadcast).await;
                            }
                            if !out.direct_messages.is_empty() {
                                for (target_id, frame) in &out.direct_messages {
                                    control.send_to(*target_id, frame).await;
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
                                drain_outbound(&mut rx, &mut write_half).await;
                                let _ = write_half.shutdown().await;
                                close = true;
                                break;
                            }
                        }
                    }
                    Err(e) => {
                        tracing::debug!(peer_ip = %peer_ip, "socket read error, closing: {e}");
                        close = true;
                    }
                }
            }
            Some(frame_hex) = rx.recv() => {
                if let Ok(wire) = frame::encode_to_wire(&frame_hex) {
                    if write_half.write_all(&wire).await.is_err() {
                        close = true;
                    }
                }
            }
            () = &mut idle_timer => {
                // Idle timeout: tell the client why, flush the alert onto the
                // socket, then close so the normal teardown below runs.
                let _ = tx.try_send(SystemAlertReason::ConnectionLost.to_hex());
                drain_outbound(&mut rx, &mut write_half).await;
                let _ = write_half.shutdown().await;
                tracing::warn!(
                    peer_ip = %peer_ip,
                    idle_timeout_secs = idle.as_secs(),
                    "idle timeout: closing silent connection"
                );
                app.write().await.push_log(
                    "warn",
                    format!("Idle timeout ({}s) from {peer_ip}; connection closed", idle.as_secs()),
                );
                close = true;
            }
        }
    }

    drain_outbound(&mut rx, &mut write_half).await;
    let _ = write_half.shutdown().await;

    if logined_id > 0 {
        let _operation_guards = crate::server::session::lock_player_operations([logined_id]).await;
        // Ch2 §2.1: a logged-in disconnect broadcasts the leave-battle +
        // offline hide frame to the map, then drops registration.
        control.disconnect_player(logined_id).await;
    }
    app.write()
        .await
        .push_log("system", format!("Client disconnected from {peer_ip}"));
}
