//! Battle input handler:
//! - Opcode 0x32 is the C→S battle-command channel (Ticket 19 / ADR 0006).
//! - Opcode 0x0B handles battle initialization, leaving, and map clicks.
//!
//! Client C→S sub map for `0x0B` (ASM `0077f414`, `HANDOFF-client-reverse.md §1.2`):
//!
//! | sub  | payload                            | meaning                          |
//! |-----:|------------------------------------|----------------------------------|
//! | `0`  | `[b]`                              | client byte (no-op)              |
//! | `1`  | `[confirm]`                        | leave battle (`confirm == 3`)    |
//! | `2`  | `[kind][u32 target][u16 index]`    | battle command (kinds 2/3/4/5)   |
//! | `3`  | `[kind][u32 target][u16 index]`    | map action                       |
//! | `4`  | `[kind][u32 target][u16 index]`    | map action                       |
//! | `5`  | `[b1][b2]`                         | raw pair — **no-op**, NOT flee   |
//! | `6`  | (none)                             | body-less map ping               |
//! | `7`  | `[mode][counter]`                  | auto-battle toggle + counter     |
//! | `8`  | `[kind][u32 target][index]`        | map/PK click (no auto-open)      |
//! | `9`  | (none)                             | banner                           |
//! | `0A` | `[kind][u32 target][index]`        | map/PK click (no auto-open)      |
//! | `FA` | (none)                             | scene loaded ack                 |
//!
//! The synchronous handlers validate + gate the request, then hand work to the
//! [`BattleService`] which spawns async per-battle tasks and routes frames.

use crate::battle::runner::{BattleAction, BattleCommand};
use crate::battle::service::BattleService;
use crate::protocol::encoder;
use crate::server::dispatcher::{HandleOutcome, OpcodeCtx};
use crate::server::session::{Conn, Session};

/// Dispatch Opcode 0x0B — the battle-command channel (Ch2 §2.3.8).
pub fn handle_battle(ctx: &mut OpcodeCtx) {
    let conn = &mut ctx.conn;
    let out = &mut ctx.out;
    let service = ctx.service;
    let data = ctx.data;
    let (sub, payload) = (ctx.sub, ctx.payload);
    match sub {
        // Sub 0 — a bare client byte; nothing to do server-side.
        0 => {
            tracing::debug!(
                sub = 0,
                byte = payload.first().copied().unwrap_or(0),
                "0x0B sub 0 (client byte; no-op)"
            );
        }
        // Sub 1 — leave battle (`0B 01 03`).
        1 => handle_leave_battle(conn, payload, service, out),
        // Sub 2 — battle command channel: `[kind][u32 target][u16 index]`.
        2 => {
            let body = parse_action_body(payload);
            tracing::debug!(
                sub = 2,
                kind = body.kind,
                target = body.target,
                index = body.index,
                "0x0B sub 2 battle command"
            );
            handle_pk_or_attack(conn, payload, service, out);
        }
        // Sub 3 — map action (8 bytes: [kind:1B][target:4B LE][param:2B LE], Ticket 19).
        3 => log_map_action(3, payload),
        // Sub 4 — map action (8 bytes: [kind:1B][target:4B LE][param:2B LE], Ticket 19).
        4 => log_map_action(4, payload),
        // Sub 5 — raw 2-byte body. The client builder for sub 5 is a no-op
        // (`JA 0x78a4f2`), so this is NOT a flee branch.
        5 => log_raw_pair(5, payload),
        // Sub 6 — 2-byte, body-less ping; keep the existing map broadcast.
        6 => {
            tracing::debug!(
                sub = 6,
                "0x0B sub 6 (2-byte, no body) map ping"
            );
            service.send_map(
                i64::from(conn.session.id),
                format!("F44406000B06{}", encoder::le32(conn.session.id)),
            );
        }
        // Sub 7 — auto-battle toggle + turn counter: `[mode][counter]`.
        7 => {
            tracing::debug!(
                sub = 7,
                mode = payload.first().copied().unwrap_or(0),
                counter = payload.get(1).copied().unwrap_or(0),
                "0x0B sub 7 auto-battle (mode/counter)"
            );
        }
        // Sub 8 — map/PK click (`[kind][u32 target][index]`). Never spawns a
        // battle directly (HANDOFF §5: these are out-of-battle map ops).
        8 => log_map_action(8, payload),
        // Sub 9 — empty banner sub.
        9 => tracing::debug!(sub = 9, "0x0B sub 9 (empty, banner)"),
        // Sub 0A — map/PK click, same family as sub 8. Never auto-opens.
        0x0A => log_map_action(0x0A, payload),
        // Sub FA — open-battle marker (the heavy S→C frame; C→S is bare ack - Ticket 20).
        0xFA => {
            tracing::debug!(sub = 0xFA, "0x0B sub 0xFA (client scene ack)");
            service.ack_scene(conn.session.battle_id);
        }
        _ => {
            tracing::debug!(sub, "0x0B unknown sub (ignored)");
            let _ = (data, out);
        }
    }
}

/// Parsed `[kind][u32 target][index]` body shared by subs 2, 8 and 0A.
///
/// The client builder emits `kind:1 + target:u32 LE + index` where `index` is a
/// `u16 LE` for sub 2 (`00780634: MOV DX, word ptr [EDX+0x22]`) but only a
/// single low byte for sub 8 (`@LStrFromChar`, `battle/02_battle_positions.md
/// §0.1.5`); sub 0A carries no index. Parsing is length-tolerant so every
/// variant logs without panicking on a short frame.
struct ActionBody {
    kind: u8,
    target: u32,
    index: u16,
}

fn parse_action_body(payload: &[u8]) -> ActionBody {
    let kind = payload.first().copied().unwrap_or(0);
    let target = if payload.len() >= 5 {
        encoder::u32_le_slice(&payload[1..5])
    } else {
        0
    };
    let index = if payload.len() >= 7 {
        encoder::u16_le(payload[5], payload[6])
    } else if payload.len() == 6 {
        u16::from(payload[5])
    } else {
        0
    };
    ActionBody {
        kind,
        target,
        index,
    }
}

/// Structured log for a body-less/raw 2-byte `0x0B` sub (e.g. sub 5).
fn log_raw_pair(sub: u8, payload: &[u8]) {
    tracing::debug!(
        sub,
        b1 = payload.first().copied().unwrap_or(0),
        b2 = payload.get(1).copied().unwrap_or(0),
        "0x0B raw 2-byte body (no-op)"
    );
}

/// Structured log for the out-of-battle map/PK subs 8 / 0A. No battle is opened.
fn log_map_action(sub: u8, payload: &[u8]) {
    let body = parse_action_body(payload);
    tracing::debug!(
        sub,
        kind = body.kind,
        target = body.target,
        index = body.index,
        "0x0B map/PK click (no battle auto-open)"
    );
}

/// Sub 1 — leave battle (`data[6] == 3`).
fn handle_leave_battle(
    conn: &mut Conn,
    payload: &[u8],
    service: &BattleService,
    out: &mut HandleOutcome,
) {
    let confirm = payload.first().copied().unwrap_or(0);
    if confirm != 3 {
        // Contract from `00590ba8:160`: the leave-battle button sends `03`.
        // Any other value is logged and ignored — never treated as a leave.
        tracing::debug!(
            sub = 1,
            confirm,
            "0x0B sub 1 ignored: confirm byte != 3"
        );
        return;
    }
    if conn.session.battle_id == 0 {
        return;
    }
    service.leave_battle(&mut conn.session);
    let smoke_clear = crate::battle::packets::hide_from_map(conn.session.id);
    service.send_map(i64::from(conn.session.id), smoke_clear.clone());
    out.send(smoke_clear);
    out.send(crate::battle::packets::battle_exit_move());
    out.send(crate::battle::packets::battle_exit_talk());
}

/// Sub 2 — inner sub 2 (PK challenge) / inner sub 3 (attack NPC) / inner sub 4 (spectate) / inner sub 5 (jam).
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
        4 => {
            if payload.len() >= 5 {
                let leader = encoder::u32_le_slice(&payload[1..5]) as i32;
                service.join_battle(&mut conn.session, leader);
            }
        }
        5 => {
            // JamPlayerToBattle
            if payload.len() >= 5 {
                let _target = encoder::u32_le_slice(&payload[1..5]);
            }
        }
        other => tracing::debug!(kind = other, "0x0B sub 2 unknown kind (ignored)"),
    }
}

/// Sub 2 sub 2 — PK challenge. Gates: not in battle, `_My_Pk == 1`, target
/// online + not in battle. Target `Pk == 0` → `F4440300210101`; `Pk == 1` →
/// start a PK battle (DiaHinh from data[7..9] / payload[5..7], default 112).
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
    let ground = if payload.len() >= 7 {
        encoder::u16_le(payload[5], payload[6]) as i64
    } else {
        112
    };
    if !target_pk {
        out.send("F4440300210101");
    } else {
        service.start_pk_battle(&mut conn.session, i64::from(target_id), ground);
    }
}

/// Sub 2 sub 3 — attack NPC. Gates: not in battle; blocked for quest-flag/doll
/// NPC ranges; else start an NPC battle (DiaHinh from data[7..9] / payload[5..7], default 112).
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
    let ground = if payload.len() >= 7 {
        encoder::u16_le(payload[5], payload[6]) as i64
    } else {
        112
    };
    service.start_npc_battle(&mut conn.session, i64::from(npc_id), 0, ground);
}

/// Dispatch Opcode 0x32 — the battle-command channel (Ticket 19 / ADR 0006).
///
/// Binary frame layout (10 bytes C→S):
/// [32] [sub:1B] [srcGrp:1B] [srcSlot:1B] [dstGrp:1B] [dstSlot:1B] [skillOrItemId:2B LE] [rand1:1B] [rand2:1B]
///
/// sub 0x01: Action/Skill/NormalAttack/Defend/Flee/Capture
/// sub 0x02: Use Item on battle grid
pub fn handle_battle_command(ctx: &mut OpcodeCtx) {
    let conn = &mut ctx.conn;
    let service = ctx.service;
    let sub = ctx.sub;
    let payload = ctx.payload;

    if payload.len() < 6 || conn.session.battle_id == 0 {
        return;
    }

    let src_grp = payload[0];
    let src_slot = payload[1];
    let dst_grp = payload[2];
    let dst_slot = payload[3];
    let code = encoder::u16_le(payload[4], payload[5]) as i64;

    if src_grp > 3 || src_slot > 4 || dst_grp > 3 || dst_slot > 4 {
        return;
    }

    match sub {
        // Sub 1: Normal attack, Skill, Defend, Flee, Capture
        1 => {
            if code == 0 {
                return;
            }
            let action = match code {
                10000 => BattleAction::NormalAttack,
                17001 => BattleAction::Defend,
                18001 => BattleAction::Flee,
                15001 => BattleAction::Capture,
                skill_id => {
                    let skill_lv = skill_level_for(&conn.session, skill_id, src_grp);
                    BattleAction::CastSkill { skill_id, skill_lv }
                }
            };
            let cmd = BattleCommand {
                row: src_grp,
                col: src_slot,
                target_row: dst_grp,
                target_col: dst_slot,
                action,
            };
            if service.submit_command(&conn.session, cmd) {
                // Broadcast acting indicator `F44404003505`+row+col (SendSKillingToParty).
                service.broadcast(format!("F44404003505{:02X}{:02X}", src_grp, src_slot));
            }
        }
        // Sub 2: In-battle item use (26001..=27165)
        2 => {
            let item_id = code;
            if !(26001..=27165).contains(&item_id) {
                return;
            }
            let cmd = BattleCommand {
                row: src_grp,
                col: src_slot,
                target_row: dst_grp,
                target_col: dst_slot,
                action: BattleAction::UseItem { item_id },
            };
            if service.submit_command(&conn.session, cmd) {
                // Accepted by the engine: consume exactly one from the bag.
                conn.session.remove_homdo_item(item_id as u16, 1);
            } else {
                tracing::debug!(
                    item_id,
                    "0x32 sub 2 use-item rejected by engine; inventory untouched"
                );
            }
        }
        other => {
            tracing::debug!(sub = other, "0x32 unknown sub (ignored)");
        }
    }
}

/// Resolve the skill level for a submitted command: player skill (`SkillGet`)
/// for player cells, or the active pet's matching skill for pet cells.
fn skill_level_for(session: &Session, skill_id: i64, row: u8) -> i64 {
    if row == 1 || row == 2 {
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
