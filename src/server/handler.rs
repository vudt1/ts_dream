//! Opcode dispatch (Chapter 2 §2.2).
//!
//! `UpdateMainGrid_Recv` switches on byte [4]. All handlers run inside an
//! empty `try`/`catch` in the C# — a thrown exception is swallowed and the
//! socket stays open. Unknown opcodes are silently ignored (no reply, no
//! close). Handlers emit an ordered list of hex reply frames.

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
        // Hello — exact opcode 0x00 with length 1 and no sub byte.
        0x00 => {
            if sub_absent(payload) {
                out.send("F4440300010901");
            }
        }
        // Enter-game confirm.
        0x03 => {
            if !conn.session.authed {
                out.send("F4440300010300");
            }
        }
        _ => {
            // Not yet ported / unknown: silently ignored.
            let _ = (sub, payload, data);
        }
    }
    Ok(())
}

/// True when there is no sub-opcode byte (op 0x00 Hello: whole frame must be
/// `F444010000` — a length-1 frame whose only payload byte is the opcode).
fn sub_absent(payload: &[u8]) -> bool {
    payload.is_empty()
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
    fn enter_game_before_auth_creates_char() {
        let mut conn = Conn::new();
        let decoded = encoder::bytes("F44402000301").unwrap();
        let out = dispatch(&mut conn, &decoded, &dummy_data());
        assert_eq!(out.outgoing, vec!["F4440300010300"]);
    }

    #[test]
    fn unknown_opcode_silent() {
        let mut conn = Conn::new();
        let decoded = encoder::bytes("F4440200C700").unwrap();
        let out = dispatch(&mut conn, &decoded, &dummy_data());
        assert!(out.outgoing.is_empty());
    }
}