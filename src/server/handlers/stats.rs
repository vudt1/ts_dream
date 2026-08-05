//! Stat allocation (Opcode 0x08) & hotkey skill bar (Opcode 0x28) handlers.

use crate::battle::engine::{get_hp_max, get_sp_max};
use crate::protocol::encoder;
use crate::server::handler::OpcodeCtx;

/// Build stat update frame `F4440C000801` + type + sign + le32(val) + `00000000`.
pub fn build_stat_update(stat_type: u8, val: i32) -> String {
    let (sign, abs_val) = if val >= 0 {
        ("01", val as u32)
    } else {
        ("02", (-val) as u32)
    };
    let body = format!("{:02X}{}{}{}", stat_type, sign, encoder::le32(abs_val), "00000000");
    crate::protocol::frame("0801", &body)
}

/// Handle Opcode 0x08 — Stat allocation.
pub fn handle_stat_allocation(ctx: &mut OpcodeCtx) {
    let conn = &mut ctx.conn;
    let out = &mut ctx.out;
    let (sub, payload) = (ctx.sub, ctx.payload);
    if sub != 1 || conn.session.battle_id > 0 {
        return;
    }

    let (stat_id, points) = if payload.len() >= 4 && payload[2] != 0 {
        (payload[2], payload[3])
    } else if payload.len() >= 2 {
        (payload[0], payload[1])
    } else {
        return;
    };

    let pts = points as u16;
    if pts == 0 || conn.session.point < pts {
        return;
    }

    match stat_id {
        25 => {
            // Hpmax recompute
            let new_hp = get_hp_max(
                conn.session.reborn as i64,
                conn.session.job as i64,
                conn.session.level as i64,
                (conn.session.hpx + pts) as i64,
            ) as u16
                + conn.session.hpx2 as u16;
            conn.session.hp = new_hp;
            out.send(build_stat_update(0x19, new_hp as i32));
        }
        26 => {
            // Spmax recompute
            let new_sp = get_sp_max(
                conn.session.reborn as i64,
                conn.session.job as i64,
                conn.session.level as i64,
                (conn.session.spx + pts) as i64,
            ) as u16
                + conn.session.spx2 as u16;
            conn.session.sp = new_sp;
            out.send(build_stat_update(0x1A, new_sp as i32));
        }
        27 => {
            // Int
            if conn.session.int1 < 400 {
                conn.session.point -= pts;
                conn.session.int1 += pts;
                out.send(build_stat_update(0x26, conn.session.point as i32));
                out.send(build_stat_update(0x1B, conn.session.int1 as i32));
            }
        }
        28 => {
            // Atk
            if conn.session.atk < 400 {
                conn.session.point -= pts;
                conn.session.atk += pts;
                out.send(build_stat_update(0x26, conn.session.point as i32));
                out.send(build_stat_update(0x1C, conn.session.atk as i32));
            }
        }
        29 => {
            // Def
            if conn.session.def < 400 {
                conn.session.point -= pts;
                conn.session.def += pts;
                out.send(build_stat_update(0x26, conn.session.point as i32));
                out.send(build_stat_update(0x1D, conn.session.def as i32));
            }
        }
        30 => {
            // Agi
            if conn.session.agi < 400 {
                conn.session.point -= pts;
                conn.session.agi += pts;
                out.send(build_stat_update(0x26, conn.session.point as i32));
                out.send(build_stat_update(0x1E, conn.session.agi as i32));
            }
        }
        31 => {
            // Hpx
            if conn.session.hpx < 400 {
                conn.session.point -= pts;
                conn.session.hpx += pts;
                conn.session.recompute_stats();
                out.send(build_stat_update(0x26, conn.session.point as i32));
                out.send(build_stat_update(0x19, conn.session.hp_max as i32));
                out.send(build_stat_update(0x1F, conn.session.hpx as i32));
            }
        }
        32 => {
            // Spx
            if conn.session.spx < 400 {
                conn.session.point -= pts;
                conn.session.spx += pts;
                conn.session.recompute_stats();
                out.send(build_stat_update(0x26, conn.session.point as i32));
                out.send(build_stat_update(0x1A, conn.session.sp_max as i32));
                out.send(build_stat_update(0x20, conn.session.spx as i32));
            }
        }
        _ => {}
    }
}

/// Handle Opcode 0x28 — Hotkey / skill bar.
pub fn handle_hotkey(ctx: &mut OpcodeCtx) {
    let conn = &mut ctx.conn;
    let payload = ctx.payload;
    // payload: data[7..8] LE u16 skill_id, data[9] slot (1..10)
    // payload indices: payload[1..3] as LE u16 skill_id, payload[3] slot
    if payload.len() >= 4 {
        let skill_id = encoder::u16_le(payload[1], payload[2]);
        let slot = payload[3];
        if (1..=10).contains(&slot) {
            conn.session.hotkeys[slot as usize] = skill_id;
        }
    } else if payload.len() >= 3 {
        let skill_id = encoder::u16_le(payload[0], payload[1]);
        let slot = payload[2];
        if (1..=10).contains(&slot) {
            conn.session.hotkeys[slot as usize] = skill_id;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::battle::service::BattleService;
    use crate::data::loader::GameData;
    use crate::server::handler::{test_ctx, HandleOutcome};
    use crate::server::session::Conn;
    use std::sync::Arc;

    #[test]
    fn test_stat_allocation_int() {
        let mut conn = Conn::new();
        conn.session.point = 10;
        conn.session.int1 = 5;

        let data = GameData::default();
        let service = BattleService::new(Arc::new(GameData::default()));
        let mut out = HandleOutcome::default();
        // stat_id = 27 (0x1B), points = 2
        let payload = vec![0, 0, 27, 2];
        let mut ctx = test_ctx(&mut conn, &data, &service, &mut out, 1, &payload);
        handle_stat_allocation(&mut ctx);

        assert_eq!(conn.session.point, 8);
        assert_eq!(conn.session.int1, 7);
        assert_eq!(out.outgoing.len(), 2);
        assert_eq!(out.outgoing[0], build_stat_update(0x26, 8));
        assert_eq!(out.outgoing[1], build_stat_update(0x1B, 7));
    }

    #[test]
    fn test_hotkey_assignment() {
        let mut conn = Conn::new();
        let data = GameData::default();
        let service = BattleService::new(Arc::new(GameData::default()));
        let mut out = HandleOutcome::default();
        // skill_id = 10001 (0x2711), slot = 3
        let payload = vec![0x00, 0x11, 0x27, 3];
        let mut ctx = test_ctx(&mut conn, &data, &service, &mut out, 1, &payload);
        handle_hotkey(&mut ctx);

        assert_eq!(conn.session.hotkeys[3], 10001);
        assert!(out.outgoing.is_empty());
    }
}
