//! Game server session: owns one socket, frames traffic, dispatches opcodes,
//! and holds per-connection player state.

use crate::protocol::frame::Decoder;

/// Per-connection game session state (mirrors the subset of the C# `Client`
/// the handlers touch: `_My_Id`, `_My_Logined`, login phase, talk state).
#[derive(Debug, Default, Clone)]
pub struct Session {
    /// `_My_Id`.
    pub id: u32,
    /// `_My_Logined`.
    pub logined: bool,
    /// Whether auth/hello succeeded.
    pub authed: bool,
    /// idtalking (talk/NPC target) set on start-talk.
    pub idtalking: i32,
    /// SelectMenu for talk menus.
    pub select_menu: i32,
    /// Active battle id (`_My_IdBattle`).
    pub battle_id: i32,
    /// Pending password from login (compared to acc).
    pub pending_pass: Vec<u8>,
    /// Pending character name from name-check.
    pub pending_new_char_name: Vec<u8>,
}

impl Session {
    pub fn new() -> Self {
        Self::default()
    }
}

/// Owns the incoming decode buffer for a connection.
pub struct Conn {
    pub decoder: Decoder,
    pub session: Session,
}

impl Conn {
    pub fn new() -> Self {
        Self {
            decoder: Decoder::new(),
            session: Session::new(),
        }
    }
}