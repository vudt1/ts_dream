//! Movement & map position handlers (Opcode 0x05, 0x06).

use crate::protocol::encoder;
use crate::server::handler::OpcodeCtx;
use crate::server::spawn;

/// Op 0x05 / 0x06 — Move.
pub fn handle_move(ctx: &mut OpcodeCtx) {
    let conn = &mut ctx.conn;
    let out = &mut ctx.out;
    let (sub, payload) = (ctx.sub, ctx.payload);
    if sub != 1 || payload.len() < 5 {
        return;
    }
    if conn.session.battle_id > 0 {
        return; // In battle → movement is ignored (Ch2 §2.3.5).
    }
    let dir = payload[0];
    let x = encoder::u16_le(payload[1], payload[2]);
    let y = encoder::u16_le(payload[3], payload[4]);
    conn.session.gocnhin = dir;
    conn.session.map_x = x;
    conn.session.map_y = y;
    let id = conn.session.id;
    let id_leader = conn.session.id_leader;
    if id_leader > 0 && id_leader != id {
        return; // Member following a leader; the leader moves everyone.
    }
    out.send(spawn::move_broadcast(id, dir, x, y));
    if id_leader == id {
        for member in conn.session.id_mem {
            if member > 0 {
                out.send(spawn::move_broadcast(member, dir, x, y));
            }
        }
    }
}
