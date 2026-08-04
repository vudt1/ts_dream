//! Server control manager (Chapter 7 §7.3 / §7.6).
//!
//! Controls game TCP server lifecycle: start, stop (with 5s 020C countdown),
//! announce broadcast, and active client connection tracking.

use crate::data::loader::GameData;
use crate::server::spawn::announce_frame;
use crate::state::AppState;
use axum::http::StatusCode;
use sqlx::MySqlPool;
use std::collections::HashMap;
use std::sync::Arc;
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
    pub async fn start(&self) -> Result<bool, String> {
        let mut app = self.app.write().await;
        if app.running {
            return Ok(true);
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
            format!("Game server started listening on 0.0.0.0:{}", self.game_port),
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
}

/// Connection handler for game TCP streams.
async fn handle_client_connection(
    stream: tokio::net::TcpStream,
    peer: std::net::SocketAddr,
    app: Arc<RwLock<AppState>>,
    _data: Option<Arc<GameData>>,
    _pool: Option<MySqlPool>,
    _control: ServerControl,
) {
    let peer_ip = peer.to_string();
    app.write().await.push_log(
        "system",
        format!("Client connected from {peer_ip}"),
    );
    let _ = stream;
}
