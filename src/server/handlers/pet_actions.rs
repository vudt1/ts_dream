//! Pet actions (Opcode 0x0F), Pet stable (Opcode 0x1F), & Pet summon/recall (Opcode 0x13) handlers.
//!
//! Wire semantics follow the C# authority (`Client.cs:1776-2074`) — the ticket
//! checklist names are NOT authoritative: the review corrected the subcode
//! mapping:
//!
//! - `0x0F sub 3` / `0x1F sub 2`: **Stable → Roster** (take out of the stable;
//!   `Client.cs:1790-1812, 6006-6028`). Request `packet[6]` names a stable
//!   slot; the source row is `packet[6] + 4`.
//! - `0x0F sub 7` / `0x1F sub 3`: **Roster → Stable** (store; has the
//!   active-pet guard; `Client.cs:1814-1850, 6030-6064`). `packet[6]` is the
//!   roster slot.
//! - `0x0F sub 8` / `0x1F sub 4`: **swap** a stable slot (`packet[6]+4`) with a
//!   roster slot (`packet[7]`); active-pet guard on the roster slot
//!   (`Client.cs:1852-1878, 6068-6094`).
//!
//! Every roster/stable mutation is a slot operation on the composite
//! `(player_id, stt)` and also relocates the pet equipment (`trangbi`
//! slots `stt*10+1..6`). The state is persisted through the write-through pool
//! scoped by `player_id`.

use crate::db;
use crate::protocol::encoder;
use crate::server::handler::OpcodeCtx;
use crate::server::session::{Conn, PetState};
use sqlx::MySqlPool;

/// Stable-slot scan bound (C# scans `5..10`, `Client.cs:1825`); the invariant
/// is set once here and used by every stable operation.
const STABLE_MIN: u8 = 5;
const STABLE_MAX: u8 = 10;

/// True when `stt` lies in the player's fight roster (`1..=4`).
fn is_roster(stt: u8) -> bool {
    (1..=4).contains(&stt)
}

/// The next free slot in `[lo..=hi]`, mirroring the C# first-fit scans.
fn find_free(pets: &[PetState], lo: u8, hi: u8) -> Option<u8> {
    let used: Vec<u8> = pets.iter().map(|p| p.stt).collect();
    (lo..=hi).find(|s| !used.contains(s))
}

/// Move one pet row + its six equipment slots to another slot (a pet only on
/// the source side moves; two present rows keep the swap semantics of C#
/// `Data.SwitchPet`).
fn move_pet_slot(conn: &mut Conn, from: u8, to: u8) {
    if let Some(pos) = conn.session.pets.iter().position(|p| p.stt == from) {
        conn.session.pets[pos].stt = to;
    }
    for sub in 1..=6u16 {
        let src_slot = from as u16 * 10 + sub;
        if let Some(pos) = conn
            .session
            .trangbi
            .iter()
            .position(|i| u16::from(i.slot) == src_slot)
        {
            conn.session.trangbi[pos].slot = to;
        }
    }
}

/// Persist the pet + equipment mutation in one transaction (the shared schema
/// needs both `pet` and `trangbi` to reflect a slot move atomically).
async fn persist_pet_state(pool: Option<&MySqlPool>, conn: &Conn) {
    if let Some(pool) = pool {
        db::persist::persist_sessions_transaction(Some(pool), &[&conn.session], &["pet", "trangbi"])
            .await;
    }
}

/// Handle Opcode 0x0F — Pet actions.
pub async fn handle_pet_actions(ctx: &mut OpcodeCtx<'_>) {
    let (sub, payload) = (ctx.sub, ctx.payload);
    if payload.is_empty() {
        return;
    }

    match sub {
        // Sub 2: Release pet (delete row + equipment + broadcast).
        2 => {
            let stt = payload[0];
            if ctx.conn.session.active_pet_stt == stt {
                ctx.conn.session.active_pet_stt = 0;
            }
            ctx.conn.session.pets.retain(|p| p.stt != stt);
            let lo = stt as u16 * 10 + 1;
            let hi = stt as u16 * 10 + 6;
            ctx.conn
                .session
                .trangbi
                .retain(|i| !(lo..=hi).contains(&u16::from(i.slot)));
            persist_pet_state(ctx.env.pool, ctx.conn).await;
            let id4 = encoder::le32(ctx.conn.session.id);
            let frame = format!("F44407000F02{}{:02X}", id4, stt);
            ctx.out.send(frame.clone());
            ctx.out.broadcast(ctx.conn.session.id, frame);
        }
        // Sub 3: Stable → Roster (C# Client.cs:1790-1812).
        3 => {
            let src_stable = payload[0];
            let source_stt = src_stable + 4; // stable slot (client index + 4)
            let Some(free) = find_free(&ctx.conn.session.pets, 1, 4) else {
                ctx.out.send("F44402001F09"); // roster full -> close menu
                return;
            };
            move_pet_slot(ctx.conn, source_stt, free);
            ctx.out
                .send(format!("F44405001F06{:02X}0000", src_stable));
            let pet_id = ctx
                .conn
                .session
                .pets
                .iter()
                .find(|p| p.stt == free)
                .map(|p| p.id)
                .unwrap_or(0);
            if pet_id > 0 {
                let id4 = encoder::le32(ctx.conn.session.id);
                let frame = format!(
                    "F4440C000F01{}{:02X}{}01",
                    id4,
                    free,
                    encoder::le32(u32::from(pet_id))
                );
                ctx.out.send(frame.clone());
                ctx.out.broadcast(ctx.conn.session.id, frame);
            }
            ctx.out.send("F44402001F0C");
            persist_pet_state(ctx.env.pool, ctx.conn).await;
        }
        // Sub 7: Roster → Stable (C# Client.cs:1814-1850).
        7 => {
            let stt = payload[0];
            if ctx.conn.session.active_pet_stt == stt {
                ctx.out.send(crate::server::spawn::sys_msg_frame(
                    "PET dang xuat chien khong the nhap chong!",
                ));
                ctx.out.send("F44402001F09");
                return;
            }
            let Some(free) = find_free(&ctx.conn.session.pets, STABLE_MIN, STABLE_MAX) else {
                ctx.out.send("F44402001F09"); // stable full
                return;
            };
            move_pet_slot(ctx.conn, stt, free);
            let id4 = encoder::le32(ctx.conn.session.id);
            let frame = format!("F44407000F02{}{:02X}", id4, stt);
            ctx.out.send(frame.clone());
            ctx.out.broadcast(ctx.conn.session.id, frame);
            ctx.out.send("F44402001F09");
            persist_pet_state(ctx.env.pool, ctx.conn).await;
        }
        // Sub 8: Swap stable ↔ roster (C# Client.cs:1852-1878).
        8 => {
            if payload.len() < 2 {
                return;
            }
            let stable_idx = payload[0];
            let roster_stt = payload[1];
            if ctx.conn.session.active_pet_stt == roster_stt {
                ctx.out.send(crate::server::spawn::sys_msg_frame(
                    "PET dang xuat chien khong the doi cho!",
                ));
                ctx.out.send("F44402001F09F44402001F0C");
                return;
            }
            let stable_stt = stable_idx + 4;
            move_pet_slot(ctx.conn, stable_stt, roster_stt);
            let pet_id = ctx
                .conn
                .session
                .pets
                .iter()
                .find(|p| p.stt == roster_stt)
                .map(|p| p.id)
                .unwrap_or(0);
            if pet_id > 0 {
                let id4 = encoder::le32(ctx.conn.session.id);
                let frame = format!(
                    "F4440C000F01{}{:02X}{}00",
                    id4,
                    roster_stt,
                    encoder::le32(u32::from(pet_id))
                );
                ctx.out.send(frame.clone());
                ctx.out.broadcast(ctx.conn.session.id, frame);
            }
            ctx.out.send("F44402001F09F44402001F0C");
            persist_pet_state(ctx.env.pool, ctx.conn).await;
        }
        // Sub 4 (mount horse): `packet[6..9]` LE32 pet id, 18000 < id < 19000.
        4 => {
            if payload.len() < 4 {
                return;
            }
            let pet_id = encoder::u32_le(payload[0], payload[1], payload[2], payload[3]) as u16;
            if !(18000..19000).contains(&pet_id) {
                return;
            }
            let owned = ctx
                .conn
                .session
                .pets
                .iter()
                .any(|p| p.id == pet_id && is_roster(p.stt));
            if !owned || ctx.conn.session.horse_pet_id == pet_id {
                return;
            }
            ctx.conn.session.horse_pet_id = pet_id;
            let id4 = encoder::le32(ctx.conn.session.id);
            let frame = format!(
                "F4440E000F05{}{}0000",
                id4,
                encoder::le32(u32::from(pet_id))
            );
            ctx.out.send(frame.clone());
            ctx.out.broadcast(ctx.conn.session.id, frame);
        }
        // Sub 5: Unmount horse (only ack when mounted, Client.cs:1899-1905).
        5 => {
            if ctx.conn.session.horse_pet_id == 0 {
                return;
            }
            ctx.conn.session.horse_pet_id = 0;
            let id4 = encoder::le32(ctx.conn.session.id);
            let frame = format!("F44406000F06{}", id4);
            ctx.out.send(frame.clone());
            ctx.out.broadcast(ctx.conn.session.id, frame);
        }
        // Sub 6: Rename pet (keep raw VISCII bytes; broadcast 0F09).
        6 => {
            let stt = payload[0];
            let name = &payload[1..];
            if name.is_empty() {
                return;
            }
            let Some(pos) = ctx
                .conn
                .session
                .pets
                .iter()
                .position(|p| p.stt == stt)
            else {
                return;
            };
            ctx.conn.session.pets[pos].name = name.to_vec();
            let id4 = encoder::le32(ctx.conn.session.id);
            let body = format!("{}{:02X}{}", id4, stt, encoder::strhex(name));
            let frame = crate::protocol::frame("0F09", &body);
            ctx.out.send(frame.clone());
            ctx.out.broadcast(ctx.conn.session.id, frame);
            persist_pet_state(ctx.env.pool, ctx.conn).await;
        }
        _ => {}
    }
}

/// Handle Opcode 0x1F — Pet stable menu (C# `Update_H1F`, Client.cs:6002-6097).
///
/// 0x1F sub 2/3/4 are the stable-menu equivalents of the 0x0F sub 3/7/8 flows;
/// they must NOT be forwarded by numeric coincidence to the 0x0F handler.
pub async fn handle_pet_stable(ctx: &mut OpcodeCtx<'_>) {
    let saved = ctx.sub;
    match saved {
        2 => ctx.sub = 3, // stable → roster
        3 => ctx.sub = 7, // roster → stable
        4 => ctx.sub = 8, // swap
        _ => return,
    }
    handle_pet_actions(ctx).await;
    ctx.sub = saved;
}

/// Handle Opcode 0x13 — Pet summon / recall.
///
/// Out of battle (C# `Update_H13`, Client.cs:1926-1964): sub 1 requires the
/// LE32 pet id, a roster slot (`stt <= 4`) and not being mounted; sub 2 only
/// acknowledges when an active pet exists. In battle the owner battle task is
/// the only authority for grid mutation (Ch2 §2.3.12); the handler stays quiet
/// there (ticket 17/20 seam).
pub async fn handle_pet_summon(ctx: &mut OpcodeCtx<'_>) {
    let (sub, payload) = (ctx.sub, ctx.payload);
    match sub {
        1 => {
            if payload.len() < 4 {
                return;
            }
            let pet_id = u32_le(payload[0], payload[1], payload[2], payload[3]);
            if ctx.conn.session.horse_pet_id == pet_id as u16 {
                return;
            }
            let pet_id = pet_id as u16;
            let Some(pet) = ctx.conn.session.pets.iter().find(|p| p.id == pet_id) else {
                return;
            };
            if !is_roster(pet.stt) {
                return;
            }
            if ctx.conn.session.battle_id == 0 {
                ctx.conn.session.active_pet_stt = pet.stt;
                ctx.out
                    .send(format!("F44406001301{}", encoder::le32(u32::from(pet_id))));
                db::persist::update_player(
                    ctx.env.pool,
                    ctx.conn.session.id,
                    "SttPetXuatchien",
                    i64::from(pet.stt),
                )
                .await;
            }
            // In battle: the battle task owns the grid — no direct mutation.
        }
        2 => {
            if ctx.conn.session.active_pet_stt == 0 {
                return;
            }
            ctx.conn.session.active_pet_stt = 0;
            ctx.out.send("F44402001302");
            db::persist::update_player(ctx.env.pool, ctx.conn.session.id, "SttPetXuatchien", 0)
                .await;
        }
        _ => {}
    }
}

/// LE32 read of the four payload bytes.
fn u32_le(p0: u8, p1: u8, p2: u8, p3: u8) -> u32 {
    u32::from_le_bytes([p0, p1, p2, p3])
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::battle::service::BattleService;
    use crate::data::loader::GameData;
    use crate::server::handler::{test_ctx, HandleOutcome};
    use crate::server::session::Conn;
    use std::sync::Arc;

    fn fixture() -> (Conn, GameData, BattleService) {
        let mut conn = Conn::new();
        conn.session.id = 300001;
        conn.session.pets = vec![
            PetState {
                stt: 1,
                id: 18001,
                ..Default::default()
            },
            PetState {
                stt: 5,
                id: 18002,
                ..Default::default()
            },
        ];
        (
            conn,
            GameData::default(),
            BattleService::new(Arc::new(GameData::default())),
        )
    }

    #[test]
    fn find_free_honors_range() {
        let pets: Vec<PetState> = vec![PetState {
            stt: 2,
            ..Default::default()
        }];
        assert_eq!(find_free(&pets, 1, 4), Some(1));
        let full: Vec<PetState> = (1..=4)
            .map(|stt| PetState {
                stt,
                ..Default::default()
            })
            .collect();
        assert_eq!(find_free(&full, 1, 4), None);
    }

    #[tokio::test]
    async fn sub3_stable_to_roster_moves_pet() {
        let (mut conn, data, service) = fixture();
        let mut out = HandleOutcome::default();
        let mut ctx = test_ctx(&mut conn, &data, &service, &mut out, 3, &[1]);
        handle_pet_actions(&mut ctx).await;

        assert!(
            conn.session
                .pets
                .iter()
                .any(|p| p.id == 18002 && is_roster(p.stt)),
            "pet moved into a free roster slot"
        );
        assert!(out.outgoing.iter().any(|f| f.contains("1F06")));
    }

    #[tokio::test]
    async fn sub7_roster_to_stable_guards_active() {
        let (mut conn, data, service) = fixture();
        conn.session.active_pet_stt = 1;
        let mut out = HandleOutcome::default();
        let mut ctx = test_ctx(&mut conn, &data, &service, &mut out, 7, &[1]);
        handle_pet_actions(&mut ctx).await;
        // Active pet cannot be stored: red message + `F44402001F09`, no move.
        assert!(out.outgoing.iter().any(|f| f.ends_with("1F09")));
        assert!(
            conn.session.pets.iter().any(|p| p.id == 18001 && p.stt == 1),
            "active pet must not move to the stable"
        );
    }

    #[tokio::test]
    async fn sub7_non_active_roster_to_stable_moves() {
        let (mut conn, data, service) = fixture();
        // Only the 18001 roster pet is active-able; store pet 18001 (stt 1).
        let mut out = HandleOutcome::default();
        let mut ctx = test_ctx(&mut conn, &data, &service, &mut out, 7, &[1]);
        handle_pet_actions(&mut ctx).await;
        assert!(out.outgoing.iter().any(|f| f.starts_with("F44407000F02")));
        assert!(
            conn.session.pets.iter().any(|p| p.id == 18001 && p.stt >= 5),
            "roster pet stored into the stable"
        );
    }

    #[tokio::test]
    async fn summon_requires_roster() {
        let (mut conn, data, service) = fixture();
        conn.session.pets.push(PetState {
            stt: 5,
            id: 15001,
            ..Default::default()
        });
        let mut out = HandleOutcome::default();
        // 15001 (0x3A99) is in the stable — LE32 request must be rejected.
        let mut ctx = test_ctx(&mut conn, &data, &service, &mut out, 1, &[0x99, 0x3A, 0, 0]);
        handle_pet_summon(&mut ctx).await;
        assert!(out.outgoing.is_empty());
    }

    #[tokio::test]
    async fn summon_le32_picks_active() {
        let (mut conn, data, service) = fixture();
        let mut out = HandleOutcome::default();
        // 18001 (0x4651) LE32.
        let mut ctx = test_ctx(&mut conn, &data, &service, &mut out, 1, &[0x51, 0x46, 0, 0]);
        handle_pet_summon(&mut ctx).await;
        assert_eq!(conn.session.active_pet_stt, 1);
        assert!(out.outgoing[0].contains("1301"));
    }
}