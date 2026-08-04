//! Expressions and emotion action handler (Opcode 0x20).

use crate::server::handler::HandleOutcome;
use crate::server::session::Conn;
use crate::server::spawn;

/// Op 0x20 — Expressions.
pub fn handle_expressions(conn: &mut Conn, sub: u8, payload: &[u8], out: &mut HandleOutcome) {
    match sub {
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
    }
}
