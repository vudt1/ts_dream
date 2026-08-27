//! Party handlers (Opcode 0x0D, Chapter 2 §2.3).
//!
//! Ticket 20 G4 scope: the quan-su (quartermaster) designation frames only —
//! sub 5 sets the leader's `_My_IdQS` when the payload names one of the four
//! party members, sub 6 clears it. The full party system (invite/leave/etc.)
//! is out of scope; `Session.id_mem` is the source of the eligible members.

use crate::protocol::encoder;
use crate::server::dispatcher::OpcodeCtx;

/// Dispatch Opcode 0x0D — Party ops. Only the quan-su sub-ops are ported.
pub fn handle_party(ctx: &mut OpcodeCtx) {
    let conn = &mut ctx.conn;
    let out = &mut ctx.out;
    let (sub, payload) = (ctx.sub, ctx.payload);
    match sub {
        // Sub 5 — designate a member as quan-su (leader only).
        5 => {
            if payload.len() < 4 {
                return;
            }
            if conn.session.id != conn.session.id_leader {
                return; // only the party leader designates.
            }
            let member = encoder::u32_le_slice(&payload[0..4]);
            let eligible = conn.session.id_mem.iter().any(|m| *m == member && *m > 0);
            if !eligible {
                return;
            }
            conn.session.id_qs = member;
            let id4 = encoder::le32(member);
            // To the leader (self).
            out.send(format!("F44406000D08{id4}"));
            out.send(format!("F44406000D07{id4}"));
            out.send(format!("F44406000D0B{id4}"));
            // Map-wide fan-out.
            out.broadcast(conn.session.id, format!("F44406000D07{id4}"));
            out.broadcast(conn.session.id, format!("F44406000D0B{id4}"));
        }
        // Sub 6 — clear the quan-su designation.
        6 => {
            let qs = conn.session.id_qs;
            if qs == 0 {
                return;
            }
            let id4 = encoder::le32(qs);
            out.send(format!("F44406000D08{id4}"));
            out.send(format!("F44406000D0C{id4}"));
            out.broadcast(conn.session.id, format!("F44406000D08{id4}"));
            out.broadcast(conn.session.id, format!("F44406000D0C{id4}"));
            conn.session.id_qs = 0;
        }
        _ => {}
    }
}
