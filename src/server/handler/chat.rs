//! Chat & slash commands handler (Opcode 0x02).

use crate::protocol::encoder;
use crate::server::handler::HandleOutcome;
use crate::server::session::Conn;
use crate::server::spawn;

/// Op 0x02 — Chat & slash commands.
pub fn handle_chat(conn: &mut Conn, sub: u8, payload: &[u8], out: &mut HandleOutcome) {
    match sub {
        // Sub 2: Global / Map chat
        2 => {
            if payload.len() > 60 {
                return; // Dropped if message > 60 chars
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
    }
}
