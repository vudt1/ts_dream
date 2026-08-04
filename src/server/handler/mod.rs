//! Modular Opcode Dispatcher (Chapter 2 §2.2).
//!
//! `UpdateMainGrid_Recv` switches on byte [4] and delegates business logic to
//! specialized handlers in submodules:
//! - `login.rs`: Opcode 0x00, 0x01 (version check >= 186), 0x03
//! - `chat.rs`: Opcode 0x02 (chat channels, whisper, party, slash commands)
//! - `movement.rs`: Opcode 0x05, 0x06 (movement & map position)
//! - `character.rs`: Opcode 0x09 (character creation & name check)
//! - `expressions.rs`: Opcode 0x20 (actions & expressions)

pub mod character;
pub mod chat;
pub mod expressions;
pub mod login;
pub mod movement;

use crate::data::loader::GameData;
use crate::error::Result;
use crate::protocol::encoder;
use crate::server::session::Conn;

/// Result of handling one decoded frame.
#[derive(Debug, Default, Clone)]
pub struct HandleOutcome {
    pub outgoing: Vec<String>,
    pub shutdown: bool,
}

impl HandleOutcome {
    pub fn send(&mut self, frame: impl Into<String>) {
        self.outgoing.push(frame.into());
    }
}

/// Dispatch one full decoded frame (its bytes). `conn` carries session state,
/// `data` gives read tables. Handlers run inside a silent catch.
pub fn dispatch(conn: &mut Conn, decoded: &[u8], data: &GameData) -> HandleOutcome {
    let mut out = HandleOutcome::default();
    let opcode = decoded.get(4).copied().unwrap_or(0);
    let sub = decoded.get(5).copied().unwrap_or(0);
    let payload = decoded.get(6..).unwrap_or(&[]);
    // C# swallows handler exceptions: never propagate.
    let _ = handle(conn, opcode, sub, payload, data, &mut out);
    out
}

fn handle(
    conn: &mut Conn,
    opcode: u8,
    sub: u8,
    payload: &[u8],
    data: &GameData,
    out: &mut HandleOutcome,
) -> Result<()> {
    match opcode {
        // Op 0x00, 0x01, 0x03 — Hello, Login, Enter game confirm
        0x00 => login::handle_hello(payload, out),
        0x01 => login::handle_login(conn, payload, out),
        0x03 => login::handle_enter_game(conn, sub, out),

        // Op 0x02 — Chat & slash commands
        0x02 => chat::handle_chat(conn, sub, payload, out),

        // Op 0x05, 0x06 — Move
        0x05 | 0x06 => movement::handle_move(conn, sub, payload, out),

        // Op 0x09 — Character creation & name check
        0x09 => character::handle_character(conn, sub, payload, out),

        // Op 0x20 — Expressions
        0x20 => expressions::handle_expressions(conn, sub, payload, out),

        _ => {
            // Not yet ported / unknown: silently ignored.
            let _ = (sub, payload, data);
        }
    }
    Ok(())
}

/// Convert a raw decoded byte frame into the hex string (for callers that
/// already have the bytes rather than the wire hex).
pub fn hex_of(decoded: &[u8]) -> String {
    encoder::hex(decoded)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dummy_data() -> GameData {
        GameData::default()
    }

    #[test]
    fn hello_replies() {
        let mut conn = Conn::new();
        // frame: F4 44 01 00 00 (opcode 0x00, length 1, no sub byte).
        let decoded = encoder::bytes("F444010000").unwrap();
        let out = dispatch(&mut conn, &decoded, &dummy_data());
        assert_eq!(out.outgoing, vec!["F4440300010901"]);
    }

    #[test]
    fn login_version_too_low_causes_shutdown() {
        let mut conn = Conn::new();
        // Login payload with version 100 (< 186): opcode 0x01 sub 0x01 id=1 prefix="vn" ver=100 pass="123"
        let decoded = encoder::bytes("F4440B00010101000000766E6400313233").unwrap();
        let out = dispatch(&mut conn, &decoded, &dummy_data());
        assert!(out.shutdown);
    }

    #[test]
    fn login_wrong_password() {
        let mut conn = Conn::new();
        // ver=186 (0xBA), pass="WRONG"
        let decoded = encoder::bytes("F4440D00010101000000766EBA0057524F4E47").unwrap();
        let out = dispatch(&mut conn, &decoded, &dummy_data());
        assert_eq!(out.outgoing, vec!["F44402000106"]);
        assert!(!out.shutdown);
    }

    #[test]
    fn create_character_name_check_and_creation() {
        let mut conn = Conn::new();

        // 1. Name check free: opcode 0x09 sub 2 name "TESTNAME"
        let name_check_decoded = encoder::bytes("F4440A000902544553544E414D45").unwrap();
        let out1 = dispatch(&mut conn, &name_check_decoded, &dummy_data());
        assert_eq!(out1.outgoing, vec!["F4440300090300"]);
        assert_eq!(conn.session.pending_new_char_name, b"TESTNAME");

        // 2. Create character: opcode 0x09 sub 1 with valid payload
        let mut payload = vec![0u8; 26];
        payload[0] = 1; // sex
        payload[2] = 2; // hair
        payload[12] = 3; // element
        payload[19] = 4; // pass1 len
        let mut frame_bytes = vec![0xF4, 0x44, 28, 0x00, 0x09, 0x01];
        frame_bytes.extend(payload);

        let out2 = dispatch(&mut conn, &frame_bytes, &dummy_data());
        assert_eq!(out2.outgoing, vec!["F44402000901"]);
    }

    #[test]
    fn move_broadcasts_to_map() {
        let mut conn = Conn::new();
        conn.session.id = 300001;
        // Move opcode 0x06 sub 1: dir=2, x=100 (0x0064), y=200 (0x00C8)
        let move_decoded = encoder::bytes("F44407000601026400C800").unwrap();
        let out = dispatch(&mut conn, &move_decoded, &dummy_data());
        assert_eq!(conn.session.map_x, 100);
        assert_eq!(conn.session.map_y, 200);
        assert_eq!(conn.session.gocnhin, 2);
        assert_eq!(out.outgoing.len(), 1);
        assert!(out.outgoing[0].starts_with("F4440B000601"));
    }

    #[test]
    fn expression_handling() {
        let mut conn = Conn::new();
        conn.session.id = 300001;
        // Expression sub 2 action=5
        let expr_decoded = encoder::bytes("F4440300200205").unwrap();
        let out = dispatch(&mut conn, &expr_decoded, &dummy_data());
        assert_eq!(conn.session.dongtac, 5);
        assert_eq!(out.outgoing, vec!["F44407002002E193040005"]);
    }

    #[test]
    fn chat_slash_command_where() {
        let mut conn = Conn::new();
        conn.session.id = 300001;
        conn.session.map_id = 12001;
        conn.session.map_x = 400;
        conn.session.map_y = 500;
        // Chat "/where": op 0x02 sub 2 msg="/where"
        let chat_decoded = encoder::bytes("F4440C0002022F7768657265").unwrap();
        let out = dispatch(&mut conn, &chat_decoded, &dummy_data());
        assert_eq!(out.outgoing.len(), 1);
        assert!(out.outgoing[0].contains("020B")); // sys msg frame
    }
}
