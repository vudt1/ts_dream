//! Pet actions (Opcode 0x0F), Pet stable (Opcode 0x1F), & Pet summon/recall (Opcode 0x13) handlers.
//!
//! Subcode mapping (corrected during review — earlier checklist names were
//! wrong):
//!
//! - `0x0F sub 3` / `0x1F sub 2`: **Stable → Roster** (take out of the stable).
//!   Request `packet[6]` names a stable slot; the source row is `packet[6] + 4`.
//! - `0x0F sub 7` / `0x1F sub 3`: **Roster → Stable** (store; has the
//!   active-pet guard). `packet[6]` is the roster slot.
//! - `0x0F sub 8` / `0x1F sub 4`: **swap** a stable slot (`packet[6]+4`) with a
//!   roster slot (`packet[7]`); active-pet guard on the roster slot.
//!
//! Every roster/stable mutation is a slot operation on the composite
//! `(player_id, stt)` and also relocates the pet equipment (`trangbi`
//! slots `stt*10+1..6`). The state is persisted through the write-through pool
//! scoped by `player_id`.

use crate::db;
use crate::db::pool::DbPool;
use crate::protocol::encoder;
use crate::server::dispatcher::OpcodeCtx;
use crate::server::pet_box::{ACTIVE_SLOTS, STABLE_SLOTS};
use crate::server::session::{Conn, PetState};

/// True when `stt` lies in the player's fight roster (`1..=4`).
pub fn is_roster(stt: u8) -> bool {
    ACTIVE_SLOTS.contains(&stt)
}

/// The next free slot in `[lo..=hi]` (first-fit scan).
pub fn find_free(pets: &[PetState], lo: u8, hi: u8) -> Option<u8> {
    let used: Vec<u8> = pets.iter().map(|p| p.stt).collect();
    (lo..=hi).find(|s| !used.contains(s))
}

/// Move one pet row + its six equipment slots to another slot (a pet only on
/// the source side moves; two present rows swap instead).
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

/// Swap two pet slots + their pet equipment atomically.
///
/// When both slots hold a pet the two composite `(player_id, stt)` identities
/// exchange (a one-way move would leave two pets sharing one `stt`); when only
/// one side is populated the pet moves to the other slot. The six `trangbi`
/// slots (`stt*10+1..6`) relocate along with their owner.
fn swap_pet_slots(conn: &mut Conn, a: u8, b: u8) {
    if a == b {
        return;
    }
    let pa = conn.session.pets.iter().position(|p| p.stt == a);
    let pb = conn.session.pets.iter().position(|p| p.stt == b);
    match (pa, pb) {
        (Some(i), Some(j)) => {
            let sa = conn.session.pets[i].stt;
            conn.session.pets[i].stt = conn.session.pets[j].stt;
            conn.session.pets[j].stt = sa;
        }
        (Some(i), None) => conn.session.pets[i].stt = b,
        (None, Some(j)) => conn.session.pets[j].stt = a,
        (None, None) => {}
    }
    for sub in 1..=6u16 {
        let ta = u16::from(a) * 10 + sub;
        let tb = u16::from(b) * 10 + sub;
        let ia = conn
            .session
            .trangbi
            .iter()
            .position(|i| u16::from(i.slot) == ta);
        let ib = conn
            .session
            .trangbi
            .iter()
            .position(|i| u16::from(i.slot) == tb);
        match (ia, ib) {
            (Some(ia), Some(ib)) => {
                let s = conn.session.trangbi[ia].slot;
                conn.session.trangbi[ia].slot = conn.session.trangbi[ib].slot;
                conn.session.trangbi[ib].slot = s;
            }
            (Some(ia), None) => conn.session.trangbi[ia].slot = tb as u8,
            (None, Some(ib)) => conn.session.trangbi[ib].slot = ta as u8,
            (None, None) => {}
        }
    }
}

/// Persist the pet + equipment mutation in one transaction (the shared schema
/// needs both `pet` and `trangbi` to reflect a slot move atomically).
async fn persist_pet_state(pool: Option<&DbPool>, conn: &Conn) {
    if let Some(pool) = pool {
        db::persist::persist_sessions_transaction(
            Some(pool),
            &[&conn.session],
            &["pet", "trangbi"],
        )
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
        // Sub 3: Stable → Roster.
        3 => {
            let src_stable = payload[0];
            let source_stt = src_stable + 4; // stable slot (client index + 4)
            let Some(free) = find_free(
                &ctx.conn.session.pets,
                *ACTIVE_SLOTS.start(),
                *ACTIVE_SLOTS.end(),
            ) else {
                ctx.out.send("F44402001F09"); // roster full -> close menu
                return;
            };
            move_pet_slot(ctx.conn, source_stt, free);
            ctx.out.send(format!("F44405001F06{:02X}0000", src_stable));
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
        // Sub 7: Roster → Stable.
        7 => {
            let stt = payload[0];
            if ctx.conn.session.active_pet_stt == stt {
                ctx.out.send(crate::server::spawn::sys_msg_frame(
                    "PET dang xuat chien khong the nhap chong!",
                ));
                ctx.out.send("F44402001F09");
                return;
            }
            let Some(free) = find_free(
                &ctx.conn.session.pets,
                *STABLE_SLOTS.start(),
                *STABLE_SLOTS.end(),
            ) else {
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
        // Sub 8: Swap stable ↔ roster.
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
            swap_pet_slots(ctx.conn, stable_stt, roster_stt);
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
        // Sub 4 (mount horse): LE16 pet id, 18000 < id < 19000.
        4 => {
            if payload.len() < 2 {
                return;
            }
            let pet_id = encoder::u16_le(payload[0], payload[1]);
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
        // Sub 5: Unmount horse (only ack when mounted).
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
            let Some(pos) = ctx.conn.session.pets.iter().position(|p| p.stt == stt) else {
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

/// Handle Opcode 0x1F — Pet stable menu.
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
/// Out of battle: sub 1 requires the
/// LE32 pet id, a roster slot (`stt <= 4`) and not being mounted; sub 2 only
/// acknowledges when an active pet exists. In battle the owner battle task is
/// the only authority for grid mutation (Ch2 §2.3.12); the handler stays quiet
/// there (ticket 17/20 seam).
pub async fn handle_pet_summon(ctx: &mut OpcodeCtx<'_>) {
    let (sub, payload) = (ctx.sub, ctx.payload);
    match sub {
        1 => {
            if payload.len() < 2 {
                return;
            }
            let pet_id = encoder::u16_le(payload[0], payload[1]);
            if ctx.conn.session.horse_pet_id == pet_id {
                return;
            }
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
