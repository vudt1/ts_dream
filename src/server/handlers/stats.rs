//! Stat allocation (Opcode 0x08) & hotkey skill bar (Opcode 0x28) handlers.

use crate::battle::engine::{get_hp_max, get_sp_max};
use crate::db::persist;
use crate::protocol::encoder;
use crate::server::dispatcher::OpcodeCtx;

/// Build stat update frame `F4440C000801` + type + sign + le32(val) + `00000000`.
pub fn build_stat_update(stat_type: u8, val: i32) -> String {
    let (sign, abs_val) = if val >= 0 {
        ("01", val as u32)
    } else {
        ("02", (-val) as u32)
    };
    let body = format!(
        "{:02X}{}{}{}",
        stat_type,
        sign,
        encoder::le32(abs_val),
        "00000000"
    );
    crate::protocol::frame("0801", &body)
}

/// Handle Opcode 0x08 — Stat allocation.
///
/// Every allocation mutates the in-memory session and, on the live server,
/// writes the same column through to `players` (`PlayerUpdateDataId`).
pub async fn handle_stat_allocation(ctx: &mut OpcodeCtx<'_>) {
    let conn = &mut ctx.conn;
    let out = &mut ctx.out;
    let pool = ctx.env.pool;
    let (sub, payload) = (ctx.sub, ctx.payload);
    if sub != 1 || conn.session.battle_id > 0 {
        return;
    }

    // Raw packet bytes [8]/[9] carry the stat id and points; payload is
    // `data[6..]`, so they sit at `payload[2]`/`payload[3]`.
    if payload.len() < 4 {
        return;
    }
    let stat_id = payload[2];
    let points = payload[3];

    let pts = points as u16;
    if pts == 0 || conn.session.point < pts {
        return;
    }

    let player_id = conn.session.id;
    match stat_id {
        25 => {
            // Stat 25: hp-max recompute; sets current HP to the new max.
            let new_hp = get_hp_max(
                conn.session.reborn as i64,
                conn.session.job as i64,
                conn.session.level as i64,
                (conn.session.hpx + pts) as i64,
            ) as u16
                + conn.session.hpx2 as u16;
            conn.session.hp = new_hp;
            persist::update_player(pool, player_id, "Hp", i64::from(new_hp)).await;
            out.send(build_stat_update(0x19, new_hp as i32));
        }
        26 => {
            // Stat 26: sp-max recompute.
            let new_sp = get_sp_max(
                conn.session.reborn as i64,
                conn.session.job as i64,
                conn.session.level as i64,
                (conn.session.spx + pts) as i64,
            ) as u16
                + conn.session.spx2 as u16;
            conn.session.sp = new_sp;
            persist::update_player(pool, player_id, "Sp", i64::from(new_sp)).await;
            out.send(build_stat_update(0x1A, new_sp as i32));
        }
        27 => {
            // Int
            if conn.session.int1 < 400 {
                conn.session.point -= pts;
                conn.session.int1 += pts;
                persist::update_player(pool, player_id, "Point", i64::from(conn.session.point))
                    .await;
                persist::update_player(pool, player_id, "Int", i64::from(conn.session.int1)).await;
                out.send(build_stat_update(0x26, conn.session.point as i32));
                out.send(build_stat_update(0x1B, conn.session.int1 as i32));
            }
        }
        28 => {
            // Atk
            if conn.session.atk < 400 {
                conn.session.point -= pts;
                conn.session.atk += pts;
                persist::update_player(pool, player_id, "Point", i64::from(conn.session.point))
                    .await;
                persist::update_player(pool, player_id, "Atk", i64::from(conn.session.atk)).await;
                out.send(build_stat_update(0x26, conn.session.point as i32));
                out.send(build_stat_update(0x1C, conn.session.atk as i32));
            }
        }
        29 => {
            // Def
            if conn.session.def < 400 {
                conn.session.point -= pts;
                conn.session.def += pts;
                persist::update_player(pool, player_id, "Point", i64::from(conn.session.point))
                    .await;
                persist::update_player(pool, player_id, "Def", i64::from(conn.session.def)).await;
                out.send(build_stat_update(0x26, conn.session.point as i32));
                out.send(build_stat_update(0x1D, conn.session.def as i32));
            }
        }
        30 => {
            // Agi
            if conn.session.agi < 400 {
                conn.session.point -= pts;
                conn.session.agi += pts;
                persist::update_player(pool, player_id, "Point", i64::from(conn.session.point))
                    .await;
                persist::update_player(pool, player_id, "Agi", i64::from(conn.session.agi)).await;
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
                persist::update_player(pool, player_id, "Point", i64::from(conn.session.point))
                    .await;
                persist::update_player(pool, player_id, "Hpx", i64::from(conn.session.hpx)).await;
                persist::update_player(pool, player_id, "HpMax", i64::from(conn.session.hp_max))
                    .await;
                out.send(build_stat_update(0x26, conn.session.point as i32));
                out.send(build_stat_update(0x1F, conn.session.hpx as i32));
            }
        }
        32 if conn.session.spx < 400 => {
            // Spx
            conn.session.point -= pts;
            conn.session.spx += pts;
            conn.session.recompute_stats();
            persist::update_player(pool, player_id, "Point", i64::from(conn.session.point)).await;
            persist::update_player(pool, player_id, "Spx", i64::from(conn.session.spx)).await;
            persist::update_player(pool, player_id, "SpMax", i64::from(conn.session.sp_max)).await;
            out.send(build_stat_update(0x26, conn.session.point as i32));
            out.send(build_stat_update(0x20, conn.session.spx as i32));
        }
        _ => {}
    }
}

/// Handle Opcode 0x28 — Hotkey / skill bar.
///
/// Protocol layout (payload = `data[6..]`): `data[7..8]` = LE u16 skill id,
/// `data[9]` = slot (1..10; 0 clears — a display-only no-op). So skill id is
/// at `payload[1..3]` and slot at `payload[3]` (the leading `data[6]` byte is
/// ignored). No response frame — this only writes the DB row.
pub async fn handle_hotkey(ctx: &mut OpcodeCtx<'_>) {
    let conn = &mut ctx.conn;
    let pool = ctx.env.pool;
    let payload = ctx.payload;
    if payload.len() < 4 {
        return;
    }
    let skill_id = encoder::u16_le(payload[1], payload[2]);
    let slot = payload[3];
    // Slot 0 clears without writing a hotbar row. Display slot 0 is not part
    // of the 1..10 hotbar dump, so this is a clear no-op.
    if !(1..=10).contains(&slot) {
        if slot == 0 {
            conn.session.hotkeys[0] = 0;
        }
        return;
    }
    conn.session.hotkeys[slot as usize] = skill_id;
    persist::update_skillsave(pool, conn.session.id, slot, skill_id).await;
}
