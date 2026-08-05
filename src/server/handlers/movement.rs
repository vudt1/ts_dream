//! Movement & map position handlers (Opcode 0x05, 0x06).

use crate::protocol::encoder;
use crate::server::handler::OpcodeCtx;
use crate::server::spawn;

/// Op 0x05 / 0x06 — Move.
pub fn handle_move(ctx: &mut OpcodeCtx) {
    let conn = &mut ctx.conn;
    let out = &mut ctx.out;
    let (sub, payload) = (ctx.sub, ctx.payload);
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
