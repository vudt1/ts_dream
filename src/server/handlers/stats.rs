//! Stat allocation (Opcode 0x08) & hotkey skill bar (Opcode 0x28) handlers.

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
    // `data[6..]`, so they sit at `payload[2]`/`payload[3..5]`.
    if payload.len() < 5 {
        return;
    }
    let stat_id = payload[2];
    let target_val = encoder::u16_le(payload[3], payload[4]);

    if conn.session.point == 0 {
        return;
    }

    let player_id = conn.session.id;
    match stat_id {
        27 => {
            // Int
            if target_val <= conn.session.int1 + 1 && conn.session.int1 < 400 {
                conn.session.point -= 1;
                conn.session.int1 += 1;
                persist::update_player(pool, player_id, "Point", i64::from(conn.session.point))
                    .await;
                persist::update_player(pool, player_id, "Int", i64::from(conn.session.int1)).await;
                out.send(build_stat_update(0x26, conn.session.point as i32));
                out.send(build_stat_update(0x1B, conn.session.int1 as i32));
            }
        }
        28 => {
            // Atk
            if target_val <= conn.session.atk + 1 && conn.session.atk < 400 {
                conn.session.point -= 1;
                conn.session.atk += 1;
                persist::update_player(pool, player_id, "Point", i64::from(conn.session.point))
                    .await;
                persist::update_player(pool, player_id, "Atk", i64::from(conn.session.atk)).await;
                out.send(build_stat_update(0x26, conn.session.point as i32));
                out.send(build_stat_update(0x1C, conn.session.atk as i32));
            }
        }
        29 => {
            // Def
            if target_val <= conn.session.def + 1 && conn.session.def < 400 {
                conn.session.point -= 1;
                conn.session.def += 1;
                persist::update_player(pool, player_id, "Point", i64::from(conn.session.point))
                    .await;
                persist::update_player(pool, player_id, "Def", i64::from(conn.session.def)).await;
                out.send(build_stat_update(0x26, conn.session.point as i32));
                out.send(build_stat_update(0x1D, conn.session.def as i32));
            }
        }
        30 => {
            // Agi
            if target_val <= conn.session.agi + 1 && conn.session.agi < 400 {
                conn.session.point -= 1;
                conn.session.agi += 1;
                persist::update_player(pool, player_id, "Point", i64::from(conn.session.point))
                    .await;
                persist::update_player(pool, player_id, "Agi", i64::from(conn.session.agi)).await;
                out.send(build_stat_update(0x26, conn.session.point as i32));
                out.send(build_stat_update(0x1E, conn.session.agi as i32));
            }
        }
        31 => {
            // Hpx
            if target_val <= conn.session.hpx + 1 && conn.session.hpx < 400 {
                conn.session.point -= 1;
                conn.session.hpx += 1;
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
        32 => {
            // Spx
            if target_val <= conn.session.spx + 1 && conn.session.spx < 400 {
                conn.session.point -= 1;
                conn.session.spx += 1;
                conn.session.recompute_stats();
                persist::update_player(pool, player_id, "Point", i64::from(conn.session.point))
                    .await;
                persist::update_player(pool, player_id, "Spx", i64::from(conn.session.spx)).await;
                persist::update_player(pool, player_id, "SpMax", i64::from(conn.session.sp_max))
                    .await;
                out.send(build_stat_update(0x26, conn.session.point as i32));
                out.send(build_stat_update(0x20, conn.session.spx as i32));
            }
        }
        _ => {}
    }
}

/// Handle Opcode 0x28 — Hotkey / skill bar (Bear `HotkeyHandler` parity).
///
/// Wire (payload = `data[2..]`): `payload[0]` = kind (`0` = clear slot,
/// `2` = assign; other kinds ignored), `payload[1..3]` = LE u16 skill id,
/// `payload[3]` = slot 1-based (1..10). Sub must be 1. aLogin never sends
/// 0x28 (SendCommand case 0x28 is empty — opcode_28.md §6), so C→S is
/// Bear-dialect only. Slot 0 is invalid (Bear would index -1 and crash),
/// so it is rejected instead of touching `hotkeys[0]`.
/// No response frame — this only writes the DB row.
pub async fn handle_hotkey(ctx: &mut OpcodeCtx<'_>) {
    let conn = &mut ctx.conn;
    let pool = ctx.env.pool;
    let (sub, payload) = (ctx.sub, ctx.payload);
    if sub != 1 || payload.len() < 4 {
        return;
    }
    let kind = payload[0];
    let skill_id = encoder::u16_le(payload[1], payload[2]);
    let slot = payload[3];
    // Bear hotkey[slot-1] with a 10-entry array; Rust keeps hotkeys[1..=10]
    // (index 0 spare, used by the dump/load loops), so the valid range maps
    // 1:1 onto the Rust array indices.
    if !(1..=10).contains(&slot) {
        return;
    }
    match kind {
        0 => conn.session.hotkeys[slot as usize] = 0,
        2 => conn.session.hotkeys[slot as usize] = skill_id,
        _ => return,
    }
    persist::update_skillsave(pool, conn.session.id, slot, conn.session.hotkeys[slot as usize])
        .await;
}
