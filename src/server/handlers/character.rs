//! Character creation & name check handler (Opcode 0x09).

use crate::protocol::encoder;
use crate::server::handler::OpcodeCtx;

/// Op 0x09 — Create character / name check.
pub fn handle_character(ctx: &mut OpcodeCtx) {
    let conn = &mut ctx.conn;
    let out = &mut ctx.out;
    let (sub, payload) = (ctx.sub, ctx.payload);
    match sub {
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
    }
}
