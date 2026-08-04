//! Shared server state (Chapter 7 §7.2 / Chapter 1 §1.4).
//!
//! The `AppState` is shared between the game server and the web dashboard via
//! `Arc<RwLock<…>>` plus a broadcast channel for logs.

use tokio::sync::broadcast;

/// One online player entry shown on the dashboard.
#[derive(Debug, Clone, serde::Serialize)]
pub struct OnlineEntry {
    pub id: u32,
    pub name: String,
    pub ip: String,
}

/// A ring-buffer log line + SSE event payload.
#[derive(Debug, Clone, serde::Serialize)]
pub struct LogEvent {
    pub level: String,
    pub ts: u64,
    pub msg: String,
}

#[derive(Debug)]
pub struct AppState {
    pub online: Vec<OnlineEntry>,
    pub running: bool,
    pub perexp: u32,
    pub log_buffer: std::collections::VecDeque<LogEvent>,
    pub broadcast: broadcast::Sender<LogEvent>,
    /// DataLoaded flag for the accept gate.
    pub data_loaded: bool,
}

impl AppState {
    pub fn new(perexp_default: u32) -> Self {
        let (tx, _rx) = broadcast::channel(256);
        Self {
            online: Vec::new(),
            running: false,
            perexp: perexp_default,
            log_buffer: std::collections::VecDeque::new(),
            broadcast: tx,
            data_loaded: false,
        }
    }

    pub fn push_log(&mut self, level: &str, msg: String) {
        let ts = chrono::Utc::now().timestamp() as u64;
        let event = LogEvent {
            level: level.to_string(),
            ts,
            msg,
        };
        if self.log_buffer.len() >= 500 {
            self.log_buffer.pop_front();
        }
        self.log_buffer.push_back(event.clone());
        let _ = self.broadcast.send(event);
    }
}