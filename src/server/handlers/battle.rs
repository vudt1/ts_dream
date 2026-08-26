//! Battle input handlers (ticket 21): Opcode 0x0B (battle control) and
//! Opcode 0x32 (battle commands).
//!
//! The synchronous handlers validate + gate the request, then hand work to the
//! [`BattleService`] which spawns async per-battle tasks and routes frames.

use crate::battle::runner::BattleCommand;
use crate::battle::service::BattleService;
use crate::data::loader::GameData;
use crate::protocol::encoder;
use crate::server::dispatcher::{HandleOutcome, OpcodeCtx};
use crate::server::session::{Conn, Session};

/// Dispatch Opcode 0x0B — Battle control (Ch2 §2.3.8).
pub fn handle_battle(ctx: &mut OpcodeCtx) {
    let conn = &mut ctx.conn;
    let out = &mut ctx.out;
    let service = ctx.service;
    let (sub, payload) = (ctx.sub, ctx.payload);
    match sub {
        1 => handle_leave_battle(conn, payload, service, out),
        2 => handle_pk_or_attack(conn, payload, service, out),
        4 => {
            if payload.len() >= 4 {
                let leader = encoder::u32_le_slice(&payload[0..4]) as i32;
                service.join_battle(&mut conn.session, leader);
            }
        }
        5 => {
            // JamPlayerToBattle — no-op stub.
        }
        6 => {
            // Broadcast `F44406000B06` + id4 to the map.
            service.send_map(
                i64::from(conn.session.id),
                format!("F44406000B06{}", encoder::le32(conn.session.id)),
            );
        }
        _ => {
            let _ = (ctx.data, out);
        }
    }
}

/// Sub 1 — leave battle (`data[6] == 3`).
fn handle_leave_battle(
    conn: &mut Conn,
    payload: &[u8],
    service: &BattleService,
    out: &mut HandleOutcome,
) {
    let confirm = payload.first().copied().unwrap_or(0);
    if confirm != 3 || conn.session.battle_id == 0 {
        return;
    }
    service.leave_battle(&mut conn.session);
    out.send(format!(
        "F44408000B00{}0000",
        encoder::le32(conn.session.id)
    ));
}

/// Sub 2 — inner sub 2 (PK challenge) / inner sub 3 (attack NPC).
fn handle_pk_or_attack(
    conn: &mut Conn,
    payload: &[u8],
    service: &BattleService,
    out: &mut HandleOutcome,
) {
    let inner = payload.first().copied().unwrap_or(0);
    match inner {
        2 => handle_pk_challenge(conn, payload, service, out),
        3 => handle_attack_npc(conn, payload, service),
        _ => {}
    }
}

/// Sub 2 sub 2 — PK challenge. Gates: not in battle, `_My_Pk == 1`, target
/// online + not in battle. Target `Pk == 0` → `F4440300210101`; `Pk == 1` →
/// start a PK battle (DiaHinh 112).
fn handle_pk_challenge(
    conn: &mut Conn,
    payload: &[u8],
    service: &BattleService,
    out: &mut HandleOutcome,
) {
    if conn.session.pk != 1 || payload.len() < 5 {
        return;
    }
    let target_id = encoder::u32_le_slice(&payload[1..5]);
    let Some(target_pk) = service.target_pk(i64::from(target_id)) else {
        return; // target offline / unknown
    };
    let target_in_battle = service.target_battle(i64::from(target_id)).unwrap_or(0) != 0;
    if conn.session.battle_id != 0 || conn.session.pk != 1 || target_in_battle {
        return;
    }
    if !target_pk {
        out.send("F4440300210101");
    } else {
        service.start_pk_battle(&mut conn.session, i64::from(target_id));
    }
}

/// Sub 2 sub 3 — attack NPC. Gates: not in battle; blocked for quest-flag/doll
/// NPC ranges; else start an NPC battle (DiaHinh 112, idNpcOnMap bytes 5-6).
fn handle_attack_npc(conn: &mut Conn, payload: &[u8], service: &BattleService) {
    if conn.session.battle_id != 0 || payload.len() < 5 {
        return;
    }
    let npc_id = encoder::u32_le_slice(&payload[1..5]);
    if (20000..22000).contains(&npc_id)
        || (23000..25000).contains(&npc_id)
        || (26000..27000).contains(&npc_id)
    {
        return;
    }
    let npc_on_map = if payload.len() >= 7 {
        encoder::u16_le(payload[5], payload[6]) as i64
    } else {
        0
    };
    service.start_npc_battle(&mut conn.session, i64::from(npc_id), npc_on_map);
}

/// Dispatch Opcode 0x32 — Battle commands (Ch2 §2.3.27).
pub fn handle_battle_command(ctx: &mut OpcodeCtx) {
    let conn = &mut ctx.conn;
    let out = &mut ctx.out;
    let data = ctx.data;
    let service = ctx.service;
    let (sub, payload) = (ctx.sub, ctx.payload);
    match sub {
        1 => handle_skill_command(conn, payload, data, service, out),
        2 => handle_use_item(conn, payload, data, service),
        _ => {}
    }
}

/// Sub 1 — skill command. Row/col/rowAttack/colAttack + skill id (LE16).
/// Range-checks; cell must exist with `_Id > 0`; level via player skill or pet
/// skill match; broadcast `F44404003505`+row+col.
fn handle_skill_command(
    conn: &mut Conn,
    payload: &[u8],
    _data: &GameData,
    service: &BattleService,
    out: &mut HandleOutcome,
) {
    if payload.len() < 6 || conn.session.battle_id == 0 {
        return;
    }
    let row = payload[0];
    let col = payload[1];
    let row_attack = payload[2];
    let col_attack = payload[3];
    let skill_id = encoder::u16_le(payload[4], payload[5]) as i64;
    if row > 3 || col > 4 || row_attack > 3 || col_attack > 4 || skill_id == 0 {
        return;
    }

    let skill_lv = skill_level_for(&conn.session, skill_id, row);
    let cmd = BattleCommand {
        row,
        col,
        skill_id,
        skill_lv,
        row_attack,
        col_attack,
        use_item: 0,
    };
    if service.submit_command(&conn.session, cmd) {
        // Broadcast `F44404003505`+row+col (SendSKillingToParty).
        service.broadcast(format!("F44404003505{:02X}{:02X}", row, col));
        let _ = out;
    }
}

/// Sub 2 — use item (`26001..=27165`). Heals the target cell + owner's pet
/// (in the battle task), removes 1 from inventory, sets `_Attacked = 1`.
fn handle_use_item(conn: &mut Conn, payload: &[u8], _data: &GameData, service: &BattleService) {
    if payload.len() < 6 || conn.session.battle_id == 0 {
        return;
    }
    let row = payload[0];
    let col = payload[1];
    let row_attack = payload[2];
    let col_attack = payload[3];
    let item_id = encoder::u16_le(payload[4], payload[5]) as i64;
    if !(26001..=27165).contains(&item_id) {
        return;
    }
    // Consume one from inventory (synchronous).
    conn.session.remove_homdo_item(item_id as u16, 1);
    let cmd = BattleCommand {
        row,
        col,
        skill_id: 0,
        skill_lv: 0,
        row_attack,
        col_attack,
        use_item: item_id,
    };
    let _ = service.submit_command(&conn.session, cmd);
}

/// Resolve the skill level for a submitted command: player skill (`SkillGet`)
/// for player cells, or the active pet's matching skill for pet cells.
fn skill_level_for(session: &Session, skill_id: i64, row: u8) -> i64 {
    if row == 2 {
        // Pet cells resolve against the session's active pet's skill list.
        if let Some(pet) = session
            .pets
            .iter()
            .find(|p| p.stt == session.active_pet_stt)
        {
            for (sid, lv) in &pet.skills {
                if i64::from(*sid) == skill_id {
                    return i64::from(*lv);
                }
            }
        }
        return 1;
    }
    session
        .skills
        .iter()
        .find(|(sid, _)| i64::from(*sid) == skill_id)
        .map(|(_, lv)| i64::from(*lv))
        .unwrap_or(1)
}
