//! Party handlers (Opcode 0x0D, Chapter 2 §2.3).
//!
//! Ticket 20 G4 scope: the quan-su (quartermaster) designation frames only —
//! sub 5 sets the leader's `_My_IdQS` when the payload names one of the four
//! party members, sub 6 clears it. The full party system (invite/leave/etc.)
//! is out of scope; `Session.id_mem` is the source of the eligible members.

use crate::protocol::encoder;
use crate::server::handler::OpcodeCtx;

/// Dispatch Opcode 0x0D — Party ops. Only the quan-su sub-ops are ported.
pub fn handle_party(ctx: &mut OpcodeCtx) {
    let conn = &mut ctx.conn;
    let out = &mut ctx.out;
    let (sub, payload) = (ctx.sub, ctx.payload);
    match sub {
        // Sub 5 — designate a member as quan-su (leader only, C# `Client.cs:1590-1638`).
        5 => {
            if payload.len() < 4 {
                return;
            }
            if conn.session.id != conn.session.id_leader {
                return; // only the party leader designates.
            }
            let member = encoder::u32_le_slice(&payload[0..4]);
            let eligible = conn
                .session
                .id_mem
                .iter()
                .any(|m| *m == member && *m > 0);
            if !eligible {
                return;
            }
            conn.session.id_qs = member;
            let id4 = encoder::le32(member);
            // To the leader (self).
            out.send(format!("F44406000D08{id4}"));
            out.send(format!("F44406000D07{id4}"));
            out.send(format!("F44406000D0B{id4}"));
            // Map-wide fan-out (C# `SendToAllClientMapid`).
            out.broadcast(conn.session.id, format!("F44406000D07{id4}"));
            out.broadcast(conn.session.id, format!("F44406000D0B{id4}"));
        }
        // Sub 6 — clear the quan-su designation (C# `Client.cs:1640-1648`).
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::battle::service::BattleService;
    use crate::data::loader::GameData;
    use crate::server::handler::{test_ctx, HandleOutcome};
    use crate::server::session::Conn;

    #[test]
    fn leader_designates_quan_su() {
        let svc = BattleService::new(std::sync::Arc::new(GameData::default()));
        let data = GameData::default();
        let mut conn = Conn::new();
        conn.session.id = 300001;
        conn.session.id_leader = 300001;
        conn.session.id_mem = [300002, 300003, 0, 0];
        let mut out = HandleOutcome::default();
        let payload = 300002u32.to_le_bytes().to_vec();
        let mut c = test_ctx(&mut conn, &data, &svc, &mut out, 5, &payload);
        handle_party(&mut c);
        assert_eq!(conn.session.id_qs, 300002);
        // Self frames + map fan-out = 3 + 2.
        assert_eq!(out.outgoing.len(), 3);
        assert!(out.outgoing.iter().all(|f| f.contains("0D0")));
        assert_eq!(out.map_broadcast.len(), 2);
    }

    #[test]
    fn non_member_or_non_leader_ignored() {
        let svc = BattleService::new(std::sync::Arc::new(GameData::default()));
        let data = GameData::default();
        let mut conn = Conn::new();
        conn.session.id = 300001;
        conn.session.id_leader = 300001;
        conn.session.id_mem = [300002, 300003, 0, 0];
        let mut out = HandleOutcome::default();
        // Not a member.
        let payload = 300009u32.to_le_bytes().to_vec();
        let mut c = test_ctx(&mut conn, &data, &svc, &mut out, 5, &payload);
        handle_party(&mut c);
        assert_eq!(conn.session.id_qs, 0);
        assert!(out.outgoing.is_empty());
        // Member but not leader.
        let svc2 = BattleService::new(std::sync::Arc::new(GameData::default()));
        let data2 = GameData::default();
        let mut conn2 = Conn::new();
        conn2.session.id = 300002;
        conn2.session.id_leader = 300001;
        conn2.session.id_mem = [300002, 300003, 0, 0];
        let mut out2 = HandleOutcome::default();
        let payload = 300003u32.to_le_bytes().to_vec();
        let mut c2 = test_ctx(&mut conn2, &data2, &svc2, &mut out2, 5, &payload);
        handle_party(&mut c2);
        assert_eq!(conn2.session.id_qs, 0);
        assert!(out2.outgoing.is_empty());
    }

    #[test]
    fn clear_quan_su() {
        let svc = BattleService::new(std::sync::Arc::new(GameData::default()));
        let data = GameData::default();
        let mut conn = Conn::new();
        conn.session.id = 300001;
        conn.session.id_qs = 300002;
        let mut out = HandleOutcome::default();
        let mut c = test_ctx(&mut conn, &data, &svc, &mut out, 6, &[]);
        handle_party(&mut c);
        assert_eq!(conn.session.id_qs, 0);
        assert_eq!(out.outgoing.len(), 2);
        assert_eq!(out.map_broadcast.len(), 2);
    }
}