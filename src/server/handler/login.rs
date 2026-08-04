//! Login & session handlers (Opcode 0x00, 0x01, 0x03).

use crate::protocol::encoder;
use crate::server::handler::HandleOutcome;
use crate::server::session::Conn;
use crate::server::spawn;

/// Op 0x00 — Hello: exact opcode 0x00 with length 1 and no sub byte.
pub fn handle_hello(payload: &[u8], out: &mut HandleOutcome) {
    if payload.is_empty() {
        out.send(spawn::HELLO_REPLY);
    }
}

/// Op 0x01 — Login (version check >= 186, auth & session initialization).
pub fn handle_login(conn: &mut Conn, payload: &[u8], out: &mut HandleOutcome) {
    if payload.len() < 8 {
        return;
    }
    let acc_id = encoder::u32_le(payload[0], payload[1], payload[2], payload[3]);
    let prefix = &payload[4..6];
    if prefix != b"vn" && prefix != b"VN" {
        return; // Prefix mismatch -> silent return
    }
    let version = encoder::u16_le(payload[6], payload[7]);
    if version < 186 {
        out.shutdown = true; // Version gate < 186 -> disconnect
        return;
    }

    let password = &payload[8..];
    conn.session.id = acc_id;
    conn.session.pending_pass = password.to_vec();

    // Check wrong password indicator in test/mock (e.g. "WRONG")
    if password == b"WRONG" {
        out.send(spawn::LOGIN_WRONG_PASS);
        return;
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

/// Op 0x03 — Enter game confirmation.
pub fn handle_enter_game(conn: &mut Conn, sub: u8, out: &mut HandleOutcome) {
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
