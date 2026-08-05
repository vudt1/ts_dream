//! Battle input handlers (ticket 21): Opcode 0x0B (battle control) and
//! Opcode 0x32 (battle commands).
//!
//! The synchronous handlers validate + gate the request, then hand work to the
//! [`BattleService`] which spawns async per-battle tasks and routes frames.

use crate::battle::runner::BattleCommand;
use crate::battle::service::BattleService;
use crate::data::loader::GameData;
use crate::protocol::encoder;
use crate::server::handler::{HandleOutcome, OpcodeCtx};
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
fn handle_use_item(
    conn: &mut Conn,
    payload: &[u8],
    _data: &GameData,
    service: &BattleService,
) {
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
        if let Some(pet) = session.pets.iter().find(|p| p.stt == session.active_pet_stt) {
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
#[cfg(test)]
mod tests {
    use super::*;
    use crate::battle::service::BattleService;
    use crate::server::handler::test_ctx;
    use crate::server::session::Conn;

    fn service() -> BattleService {
        BattleService::new(std::sync::Arc::new(GameData::default()))
    }

    #[test]
    fn leave_battle_sends_hide_and_clears_battle() {
        let mut conn = Conn::new();
        conn.session.id = 300001;
        conn.session.battle_id = 5;
        let svc = service();
        let data = GameData::default();
        let mut out = HandleOutcome::default();
        let mut ctx = test_ctx(&mut conn, &data, &svc, &mut out, 1, &[3]);
        handle_battle(&mut ctx);
        assert_eq!(conn.session.battle_id, 0);
        assert_eq!(out.outgoing.len(), 1);
        assert!(out.outgoing[0].starts_with("F44408000B00"));
        assert!(out.outgoing[0].ends_with("0000"));
    }

    #[test]
    fn leave_battle_ignored_unless_confirm_3() {
        let mut conn = Conn::new();
        conn.session.battle_id = 5;
        let svc = service();
        let data = GameData::default();
        let mut out = HandleOutcome::default();
        let mut ctx = test_ctx(&mut conn, &data, &svc, &mut out, 1, &[1]);
        handle_battle(&mut ctx);
        assert_eq!(conn.session.battle_id, 5);
        assert!(out.outgoing.is_empty());
    }

    #[test]
    fn attack_npc_blocked_range_does_not_spawn() {
        let mut conn = Conn::new();
        conn.session.id = 300001;
        let svc = service();
        // NPC id 20001 ∈ [20000, 22000) → blocked.
        let npc_id = 20001u32;
        let mut payload = vec![0u8; 6];
        payload[0] = 3;
        payload[1..5].copy_from_slice(&npc_id.to_le_bytes());
        let data = GameData::default();
        let mut out = HandleOutcome::default();
        let mut ctx = test_ctx(&mut conn, &data, &svc, &mut out, 2, &payload);
        handle_battle(&mut ctx);
        assert_eq!(conn.session.battle_id, 0);
        assert!(out.outgoing.is_empty());
    }

    #[test]
    fn pk_challenge_target_pk_zero_replies() {
        let svc = service();
        // Register an online target with pk == 0.
        let target = std::sync::Arc::new(tokio::sync::RwLock::new(Session::new()));
        {
            let mut t = target.try_write().unwrap();
            t.id = 300002;
            t.pk = 0;
        }
        let _rx = svc.register(300002, target);

        let mut conn = Conn::new();
        conn.session.id = 300001;
        conn.session.pk = 1;
        let mut payload = vec![0u8; 5];
        payload[0] = 2;
        payload[1..5].copy_from_slice(&300002u32.to_le_bytes());
        let data = GameData::default();
        let mut out = HandleOutcome::default();
        let mut ctx = test_ctx(&mut conn, &data, &svc, &mut out, 2, &payload);
        handle_battle(&mut ctx);
        assert_eq!(out.outgoing, vec!["F4440300210101".to_string()]);
    }

    #[test]
    fn pk_challenge_self_pk_zero_is_blocked() {
        let svc = service();
        let target = std::sync::Arc::new(tokio::sync::RwLock::new(Session::new()));
        {
            let mut t = target.try_write().unwrap();
            t.id = 300002;
            t.pk = 0;
        }
        let _rx = svc.register(300002, target);

        let mut conn = Conn::new();
        conn.session.id = 300001;
        conn.session.pk = 0; // attacker must have pk == 1
        let mut payload = vec![0u8; 5];
        payload[0] = 2;
        payload[1..5].copy_from_slice(&300002u32.to_le_bytes());
        let data = GameData::default();
        let mut out = HandleOutcome::default();
        let mut ctx = test_ctx(&mut conn, &data, &svc, &mut out, 2, &payload);
        handle_battle(&mut ctx);
        assert!(out.outgoing.is_empty());
    }

    #[test]
    fn skill_command_battle_zero_is_ignored() {
        let mut conn = Conn::new();
        conn.session.battle_id = 0;
        let svc = service();
        let data = GameData::default();
        let mut out = HandleOutcome::default();
        // skill 10000 targeting (0,2)
        let mut payload = vec![0u8; 6];
        payload[0] = 3;
        payload[1] = 2;
        payload[2] = 0;
        payload[3] = 2;
        payload[4..6].copy_from_slice(&10000u16.to_le_bytes());
        let mut ctx = test_ctx(&mut conn, &data, &svc, &mut out, 1, &payload);
        handle_battle_command(&mut ctx);
        assert!(out.outgoing.is_empty());
    }
}
