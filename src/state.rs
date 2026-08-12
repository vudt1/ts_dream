//! Shared server state (Chapter 7 §7.2 / Chapter 1 §1.4).
//!
//! The `AppState` is shared between the game server and the web dashboard via
//! `Arc<RwLock<…>>` plus a broadcast channel for logs.

use tokio::sync::broadcast;

/// Live MySQL connectivity state surfaced on the dashboard (Ch7 ticket #22).
///
/// `Connecting` is the transient bootstrap state shown until the first
/// liveness probe resolves; thereafter the state is `Connected` (green) or
/// `Disconnected` (dark). The game server boot stays fail-fast (spec §1.1),
/// so `Disconnected` is only reachable if the DB drops *after* a successful
/// boot — which the background probe (db/pool.rs) detects at runtime.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DbStatus {
    Connecting,
    Connected,
    Disconnected,
}

impl DbStatus {
    /// Frontend color token: `green` / `light` / `dark`.
    pub fn as_str(&self) -> &'static str {
        match self {
            DbStatus::Connected => "green",
            DbStatus::Connecting => "light",
            DbStatus::Disconnected => "dark",
        }
    }
}

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
    /// Live MySQL connectivity (Ch7 ticket #22).
    pub db_status: DbStatus,
    /// Fan-out for DB status changes (separate from the log channel so the
    /// SSE `dbstatus` stream stays decoupled from packet/event logging).
    pub db_status_tx: broadcast::Sender<DbStatus>,
}

impl AppState {
    pub fn new(perexp_default: u32) -> Self {
        let (tx, _rx) = broadcast::channel(256);
        let (dtx, _drx) = broadcast::channel(16);
        Self {
            online: Vec::new(),
            running: false,
            perexp: perexp_default,
            log_buffer: std::collections::VecDeque::new(),
            broadcast: tx,
            data_loaded: false,
            db_status: DbStatus::Connecting,
            db_status_tx: dtx,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn db_status_as_str_mapping() {
        assert_eq!(DbStatus::Connecting.as_str(), "light");
        assert_eq!(DbStatus::Connected.as_str(), "green");
        assert_eq!(DbStatus::Disconnected.as_str(), "dark");
    }

    #[test]
    fn app_state_new_starts_connecting() {
        assert_eq!(AppState::new(0).db_status, DbStatus::Connecting);
    }

    #[tokio::test]
    async fn db_status_broadcast_delivers_to_subscriber() {
        let state = AppState::new(0);
        let mut rx = state.db_status_tx.subscribe();
        let tx = state.db_status_tx.clone();
        tx.send(DbStatus::Connected).unwrap();
        let got = rx.recv().await.expect("subscriber should receive status");
        assert_eq!(got, DbStatus::Connected);
    }
}
