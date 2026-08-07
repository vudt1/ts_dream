//! Reborn-by-item (C# 46170, 46247-46250) — `Client.cs:5052-5124`.
//!
//! These do a hard `UPDATE players SET …` (level reset to 1, rebuilds HP/SP,
//! sets Reborn) and then CLOSE the client socket. The Rust port mutates the
//! session in-memory, persists the columns, and sets `out.shutdown = true` so
//! the connection loop tears the socket down. Runs first in the use-item chain
//! (matching C# order, where the reborn gate precedes the skill-book family).

use super::UseCtx;
use crate::db::persist;

/// Handle the reborn family. Returns true when the item id was a reborn item
/// (whether or not its gate passed — the C# branch always `break`s out).
async fn reborn1(ctx: &mut UseCtx<'_>) -> bool {
    if ctx.id != 46170 {
        return false;
    }
    let ok = u16::from(ctx.conn.session.level) >= 120 && ctx.conn.session.reborn == 0;
    if !ok {
        return true; // gate failed: C# still breaks, nothing happens
    }
    ctx.consume().await;
    let pid = ctx.conn.session.id;
    // C# `UPDATE Player SET Lv=1, Point=20, Hp=181, HpMax=181, Sp=111,
    // SpMax=111, Texp=13, Reborn=1, Hair=10 WHERE Id = …`.
    ctx.conn.session.level = 1;
    ctx.conn.session.point = 20;
    ctx.conn.session.hp = 181;
    ctx.conn.session.hp_max = 181;
    ctx.conn.session.sp = 111;
    ctx.conn.session.sp_max = 111;
    ctx.conn.session.texp = 13;
    ctx.conn.session.reborn = 1;
    ctx.conn.session.hair = 10;
    persist::update_player(ctx.pool, pid, "Lv", 1).await;
    persist::update_player(ctx.pool, pid, "Point", 20).await;
    persist::update_player(ctx.pool, pid, "Hp", 181).await;
    persist::update_player(ctx.pool, pid, "HpMax", 181).await;
    persist::update_player(ctx.pool, pid, "Sp", 111).await;
    persist::update_player(ctx.pool, pid, "SpMax", 111).await;
    persist::update_player(ctx.pool, pid, "Texp", 13).await;
    persist::update_player(ctx.pool, pid, "Reborn", 1).await;
    persist::update_player(ctx.pool, pid, "Hair", 10).await;
    ctx.out.shutdown = true;
    true
}

/// Reborn-2 items (C# 46247-46250): require `Lv>=120 && Reborn==1`, each maps
/// to a job (1..4) with its own Hp/Sp reset values, and closes the socket.
async fn reborn2(ctx: &mut UseCtx<'_>) -> bool {
    let (hpmax, spmax, job) = match ctx.id {
        46247 => (281, 161, 1),
        46248 => (381, 161, 2),
        46249 => (181, 311, 3),
        46250 => (181, 411, 4),
        _ => return false,
    };
    let ok = u16::from(ctx.conn.session.level) >= 120 && ctx.conn.session.reborn == 1;
    if !ok {
        return true; // gate failed → C# break, nothing happens
    }
    ctx.consume().await;
    let pid = ctx.conn.session.id;
    // C# hard reset: Lv=1, Point=40, Hp/HpMax=…, Sp/SpMax=…, Texp=13, Reborn=2, Job=N.
    ctx.conn.session.level = 1;
    ctx.conn.session.point = 40;
    ctx.conn.session.hp = hpmax;
    ctx.conn.session.hp_max = hpmax;
    ctx.conn.session.sp = spmax;
    ctx.conn.session.sp_max = spmax;
    ctx.conn.session.texp = 13;
    ctx.conn.session.reborn = 2;
    ctx.conn.session.job = job;
    persist::update_player(ctx.pool, pid, "Lv", 1).await;
    persist::update_player(ctx.pool, pid, "Point", 40).await;
    persist::update_player(ctx.pool, pid, "Hp", i64::from(hpmax)).await;
    persist::update_player(ctx.pool, pid, "HpMax", i64::from(hpmax)).await;
    persist::update_player(ctx.pool, pid, "Sp", i64::from(spmax)).await;
    persist::update_player(ctx.pool, pid, "SpMax", i64::from(spmax)).await;
    persist::update_player(ctx.pool, pid, "Texp", 13).await;
    persist::update_player(ctx.pool, pid, "Reborn", 2).await;
    persist::update_player(ctx.pool, pid, "Job", i64::from(job)).await;
    ctx.out.shutdown = true;
    true
}

/// Entry: handle the reborn family (in C# order: 46170 first, then 46247-46250).
pub async fn handle(ctx: &mut UseCtx<'_>) -> bool {
    if reborn1(ctx).await {
        return true;
    }
    reborn2(ctx).await
}