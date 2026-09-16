//! ResponseSender: typed outgoing-packet abstraction.
//!
//! Handlers describe *what* to send (`send_bag_items`, `send_dialog_talk`,
//! `end_talk`, …); the sender owns the frame construction and the routing into
//! the [`HandleOutcome`] channels (direct reply, delayed dialog fragments, or
//! map broadcast). This keeps byte-formatting out of handler business logic.

use crate::server::dispatcher::{HandleOutcome, OutFrame};
use crate::server::handlers::stats::build_stat_update;
use crate::server::session::{Conn, Session};

/// One player's response pipeline: session in, frames out.
pub struct ResponseSender<'a> {
    conn: &'a mut Conn,
    out: &'a mut HandleOutcome,
}

impl<'a> ResponseSender<'a> {
    pub fn new(conn: &'a mut Conn, out: &'a mut HandleOutcome) -> Self {
        Self { conn, out }
    }

    /// Read-only view of the session (for building conditional responses).
    pub fn session(&self) -> &Session {
        &self.conn.session
    }

    /// Mutable view of the session (for state mutations between sends).
    pub fn session_mut(&mut self) -> &mut Session {
        &mut self.conn.session
    }

    /// Queue a raw pre-built frame as a direct reply.
    pub fn raw(&mut self, frame: impl Into<String>) {
        self.out.send(frame);
    }

    /// Queue a raw frame with a pacing delay (dialog fragments).
    pub fn raw_delayed(&mut self, frame: impl Into<String>, delay_ms: u64) {
        self.out.send_delayed(frame, delay_ms);
    }

    /// Queue a map-scoped broadcast owned by `subject`.
    pub fn broadcast(&mut self, subject: u32, frame: impl Into<String>) {
        self.out.broadcast(subject, frame);
    }

    /// Send an OP_SYSTEM_ALERT (Opcode 0x00) packet to client.
    pub fn send_system_alert(&mut self, reason: crate::protocol::SystemAlertReason) {
        self.out.send_system_alert(reason);
    }

    /// Full bag dump (`1705`).
    pub fn send_bag_items(&mut self) {
        let frame = self.conn.session.dump_homdo();
        self.out.send(frame);
    }

    /// Full equipment dump (`170B`).
    pub fn send_equipment_items(&mut self) {
        let frame = self.conn.session.dump_trangbi();
        self.out.send(frame);
    }

    /// TienTrang storage dump (`1E01`).
    pub fn send_storage_items(&mut self) {
        let frame = self.conn.session.dump_tientrang();
        self.out.send(frame);
    }

    /// Split a multi-frame dialog string on `F444` and emit each fragment
    /// 500 ms apart.
    pub fn send_dialog_talk(&mut self, talk_string: &str) {
        crate::server::handlers::talk::talk_messages(self.conn, talk_string, self.out);
    }

    /// EndTalk packet + full talk-context reset.
    pub fn end_talk(&mut self) {
        crate::server::handlers::talk::end_talk(self.conn, self.out);
    }

    /// Single-stat update (`0x19`/`0x1A` HP/SP family, `code` selects the stat).
    pub fn send_stat_update(&mut self, code: u8, value: i32) {
        let frame = build_stat_update(code, value);
        self.out.send(frame);
    }

    /// Current-HP then current-SP update pair (post-heal / post-battle sync).
    pub fn send_hp_sp_updates(&mut self) {
        let (hp, sp) = (self.conn.session.hp, self.conn.session.sp);
        self.send_stat_update(0x19, i32::from(hp));
        self.send_stat_update(0x1A, i32::from(sp));
    }

    /// Consume the sender, returning the underlying outcome for further
    /// dispatcher post-processing (battle triggers etc.).
    pub fn into_outcome(self) -> &'a mut HandleOutcome {
        self.out
    }
}

/// Re-export so callers can pattern-match `OutFrame`s returned elsewhere.
pub type SentFrame = OutFrame;
