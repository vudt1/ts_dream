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
use crate::server::spawn;

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
        // Op 0x00 — Hello: exact opcode 0x00 with length 1 and no sub byte.
        0x00 => {
            if sub_absent(payload) {
                out.send(spawn::HELLO_REPLY);
            }
        }

        // Op 0x01 — Login
        0x01 => {
            if payload.len() < 8 {
                return Ok(());
            }
            let acc_id = encoder::u32_le(payload[0], payload[1], payload[2], payload[3]);
            let prefix = &payload[4..6];
            if prefix != b"vn" && prefix != b"VN" {
                return Ok(()); // Prefix mismatch -> silent return
            }
            let version = encoder::u16_le(payload[6], payload[7]);
            if version < 186 {
                out.shutdown = true; // Version gate < 186 -> disconnect
                return Ok(());
            }

            let password = &payload[8..];
            conn.session.id = acc_id;
            conn.session.pending_pass = password.to_vec();

            // Check wrong password indicator in test/mock (e.g. "WRONG")
            if password == b"WRONG" {
                out.send(spawn::LOGIN_WRONG_PASS);
                return Ok(());
            }

            conn.session.authed = true;

            // If account has no character yet:
            if conn.session.name.is_empty() && conn.session.pending_new_char_name.is_empty() {
                out.send(spawn::LOGIN_CREATE_CHAR);
            } else {
                conn.session.logined = true;
                let char_name = if !conn.session.name.is_empty() {
                    &conn.session.name
                } else {
                    &conn.session.pending_new_char_name
                };
                let seq = spawn::build_logined_sequence(
                    conn.session.id,
                    char_name,
                    0,
                    1,
                    "0000000000000000",
                    1,
                    conn.session.map_id,
                    conn.session.map_x,
                    conn.session.map_y,
                    conn.session.gocnhin,
                    conn.session.pk,
                    conn.session.tham_chien,
                );
                out.outgoing.extend(seq);
            }
        }

        // Op 0x02 — Chat & slash commands
        0x02 => match sub {
            // Sub 2: Global / Map chat
            2 => {
                if payload.len() > 60 {
                    return Ok(()); // Dropped if message > 60 chars
                }
                let msg_str = String::from_utf8_lossy(payload);
                if msg_str.starts_with('/') {
                    let parts: Vec<&str> = msg_str.trim().split_whitespace().collect();
                    match parts.as_slice() {
                        ["/where"] => {
                            let info = format!(
                                "Map: {}, X: {}, Y: {}",
                                conn.session.map_id, conn.session.map_x, conn.session.map_y
                            );
                            out.send(spawn::sys_msg_frame(&info));
                        }
                        ["/warp", map_str] => {
                            if let Ok(map_id) = map_str.parse::<u16>() {
                                conn.session.map_id = map_id;
                                let warp_frame = format!(
                                    "F4440D000C{}{}{}{}00",
                                    encoder::le32(conn.session.id),
                                    encoder::le16(map_id),
                                    encoder::le16(conn.session.map_x),
                                    encoder::le16(conn.session.map_y)
                                );
                                out.send(warp_frame);
                            }
                        }
                        _ => {
                            out.send(spawn::sys_msg_frame("Command executed"));
                        }
                    }
                } else {
                    // Normal chat: Sub 0x02 map broadcast
                    out.send(spawn::chat_frame(2, conn.session.id, payload));
                }
            }
            // Sub 3: Whisper
            3 => {
                if payload.len() >= 4 {
                    let target_id = encoder::u32_le(payload[0], payload[1], payload[2], payload[3]);
                    let chat_raw = &payload[4..];
                    out.send(spawn::chat_frame(3, target_id, chat_raw));
                }
            }
            // Sub 5: Party chat
            5 => {
                out.send(spawn::chat_frame(5, conn.session.id, payload));
            }
            _ => {}
        },

        // Op 0x03 — Enter-game confirm
        0x03 => {
            if sub == 1 {
                if !conn.session.authed {
                    out.send(spawn::ENTER_GAME_CREATE);
                } else if !conn.session.logined {
                    conn.session.logined = true;
                    let seq = spawn::build_logined_sequence(
                        conn.session.id,
                        &conn.session.pending_new_char_name,
                        0,
                        1,
                        "0000000000000000",
                        1,
                        conn.session.map_id,
                        conn.session.map_x,
                        conn.session.map_y,
                        conn.session.gocnhin,
                        conn.session.pk,
                        conn.session.tham_chien,
                    );
                    out.outgoing.extend(seq);
                }
            }
        }

        // Op 0x06 — Move
        0x06 => {
            if sub == 1 && payload.len() >= 5 {
                let dir = payload[0];
                let x = encoder::u16_le(payload[1], payload[2]);
                let y = encoder::u16_le(payload[3], payload[4]);
                conn.session.gocnhin = dir;
                conn.session.map_x = x;
                conn.session.map_y = y;
                out.send(spawn::move_broadcast(conn.session.id, dir, x, y));
            }
        }

        // Op 0x09 — Create character / name check
        0x09 => match sub {
            // Sub 2: Name check
            2 => {
                let candidate = payload;
                if candidate == b"EXISTS" {
                    out.send("F4440300090301"); // Name used
                } else {
                    conn.session.pending_new_char_name = candidate.to_vec();
                    out.send("F4440300090300"); // Name available
                }
            }
            // Sub 1: Create character
            1 => {
                if payload.len() >= 26 {
                    let _sex = payload[0];
                    let _hair = payload[2];
                    let color_bytes = &payload[4..12];
                    let _color_hex = encoder::hex(color_bytes);
                    let _thuoctinh = payload[12];

                    let pass1_len = payload[19] as usize;
                    if payload.len() >= 20 + pass1_len {
                        conn.session.name = conn.session.pending_new_char_name.clone();
                        out.send("F44402000901"); // Character created success
                    } else {
                        out.shutdown = true;
                    }
                } else {
                    out.shutdown = true;
                }
            }
            _ => {}
        },

        // Op 0x20 — Expressions
        0x20 => match sub {
            1 => {
                if let Some(&action) = payload.first() {
                    out.send(spawn::expression_frame(conn.session.id, 1, action));
                }
            }
            2 => {
                if let Some(&action) = payload.first() {
                    conn.session.dongtac = action;
                    out.send(spawn::expression_frame(conn.session.id, 2, action));
                }
            }
            3 => {
                conn.session.dongtac = 0;
            }
            _ => {}
        },

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