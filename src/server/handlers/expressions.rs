//! Expressions and emotion action handler (Opcode 0x20).

use crate::server::handler::OpcodeCtx;
use crate::server::spawn;

/// Op 0x20 — Expressions.
pub fn handle_expressions(ctx: &mut OpcodeCtx) {
    let conn = &mut ctx.conn;
    let out = &mut ctx.out;
    let (sub, payload) = (ctx.sub, ctx.payload);
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
