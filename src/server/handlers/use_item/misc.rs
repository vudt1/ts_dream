//! Doll summon, dice items, special frames, sleep, full-heal and no-op ids —
//! C# case 15 (`Client.cs:4718-4889`). Runs after `rewards`, before the
//! point-books / party-buff / potion tail in `mod.rs`.

use super::UseCtx;
use crate::db::persist;
use crate::protocol::encoder;

/// Dice items (C# 46048/46049/46173/46174): `Random.Next(1,6)` then broadcast
/// `F44407001737` + id + num.
async fn dice(ctx: &mut UseCtx<'_>) {
    let n = ctx.rng.next_range(1, 6);
    ctx.out.send(format!(
        "F44407001737{}{:02X}",
        encoder::le32(ctx.conn.session.id),
        n
    ));
    ctx.consume().await;
}

/// Doll summon frame `F44408000505` + id + le16(npcid) + `F444040017091301F4440200170F`
/// (C# 48001-48097; the trailing `1301` is a fixed slot/count the client reads).
fn doll_frame(ctx: &UseCtx<'_>, npc_id: u16) -> String {
    format!(
        "F44408000505{}{}F444040017091301F4440200170F",
        encoder::le32(ctx.conn.session.id),
        encoder::le16(npc_id)
    )
}

/// Handle the misc family. Returns true when the item id was handled.
pub async fn handle(ctx: &mut UseCtx<'_>) -> bool {
    let id = ctx.id;
    let pid = ctx.conn.session.id;

    // --- No-op ids (C# breaks to the shared `text17` tail, no consume). ---
    if matches!(id, 46013 | 46014 | 46015 | 46042 | 46091) {
        ctx.end_feedback();
        return true;
    }

    // --- Sleep item 46036 (C# `Sleep()` + consume). ---
    if id == 46036 {
        ctx.sleep().await;
        ctx.consume().await;
        return true;
    }

    // --- Dice items. ---
    if matches!(id, 46048 | 46049 | 46173 | 46174) {
        dice(ctx).await;
        return true;
    }

    // --- 46018: consume + a special end frame. ---
    if id == 46018 {
        ctx.consume().await;
        ctx.out.send(format!(
            "F44402001726F44404001709{:02X}00F4440200170F",
            ctx.slot
        ));
        return true;
    }

    // --- 46089/46179: consume + special frame `F444040017090D01F4440B001748`. ---
    if matches!(id, 46089 | 46179) {
        ctx.consume().await;
        ctx.out.send(format!(
            "F444040017090D01F4440B001748{}{}00",
            encoder::le32(pid),
            encoder::le32(10000)
        ));
        return true;
    }

    // --- Full-heal 46068 (C#: pet Hp/Sp to max + player Hp/Sp to max). ---
    if id == 46068 {
        let stt = ctx.use_type;
        let snap = ctx
            .conn
            .session
            .pets
            .iter_mut()
            .find(|p| p.stt == stt)
            .map(|pet| {
                pet.hp = pet.hp_max;
                pet.sp = pet.sp_max;
                pet.clone()
            });
        if let Some(snap) = snap {
            ctx.pet_stat(stt, 0x19, i32::from(snap.hp));
            ctx.pet_stat(stt, 0x1A, i32::from(snap.sp));
            persist::upsert_pet(ctx.pool, pid, &snap).await;
        }
        ctx.conn.session.hp = ctx.conn.session.hp_max;
        ctx.conn.session.sp = ctx.conn.session.sp_max;
        ctx.stat(0x19, i32::from(ctx.conn.session.hp));
        ctx.stat(0x1A, i32::from(ctx.conn.session.sp));
        persist::update_player(ctx.pool, pid, "Hp", i64::from(ctx.conn.session.hp)).await;
        persist::update_player(ctx.pool, pid, "Sp", i64::from(ctx.conn.session.sp)).await;
        ctx.consume().await;
        return true;
    }

    // --- Doll items 48001-48097 (summon only) and 48101 (summon + stat boost). ---
    if (48001..=48097).contains(&id) {
        let npc = ctx
            .data
            .dolls
            .get(&i64::from(id))
            .map(|d| d.npc_id as u16)
            .unwrap_or(0);
        ctx.out.send(doll_frame(ctx, npc));
        ctx.consume().await;
        return true;
    }
    if id == 48101 {
        let npc = ctx
            .data
            .dolls
            .get(&i64::from(id))
            .map(|d| d.npc_id as u16)
            .unwrap_or(0);
        ctx.out.send(doll_frame(ctx, npc));
        ctx.consume().await;
        // C# stat boost: Int2+20, Hpx2+200, Spx2+200, Hpmax+200, Spmax+200
        // (Hpmax/Spmax writes are silent — no stat packet).
        ctx.conn.session.int2 = ctx.conn.session.int2.saturating_add(20);
        ctx.conn.session.hpx2 = ctx.conn.session.hpx2.saturating_add(200);
        ctx.conn.session.spx2 = ctx.conn.session.spx2.saturating_add(200);
        ctx.conn.session.hp_max = ctx.conn.session.hp_max.saturating_add(200);
        ctx.conn.session.sp_max = ctx.conn.session.sp_max.saturating_add(200);
        ctx.stat(0xD4, ctx.conn.session.int2 as i32);
        ctx.stat(0xCF, ctx.conn.session.hpx2 as i32);
        ctx.stat(0xD0, ctx.conn.session.spx2 as i32);
        persist::update_player(ctx.pool, pid, "Int2", i64::from(ctx.conn.session.int2)).await;
        persist::update_player(ctx.pool, pid, "Hpx2", i64::from(ctx.conn.session.hpx2)).await;
        persist::update_player(ctx.pool, pid, "Spx2", i64::from(ctx.conn.session.spx2)).await;
        persist::update_player(ctx.pool, pid, "HpMax", i64::from(ctx.conn.session.hp_max)).await;
        persist::update_player(ctx.pool, pid, "SpMax", i64::from(ctx.conn.session.sp_max)).await;
        return true;
    }

    false
}