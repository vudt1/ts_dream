//! Inventory base (Op 0x17 subs 2, 3, 10, 11, 12) & Use items (Op 0x17 sub 15)
//! & Reborn (sub 46). Live-server path persists homdo/trangbi mutations through
//! `persist`; golden replay keeps everything in-memory over the seeded session.

use crate::db::persist;
use crate::protocol::encoder;
use crate::server::handler::{hex_of, HandleOutcome, OpcodeCtx};
use crate::server::handlers::stats::build_stat_update;
use crate::server::map_drops;
use crate::server::session::{Conn, InventoryItem};

/// Pickup range (C# `Update_H17` case 2: `-150 <= dx <= 150`, same for dy).
const PICKUP_RANGE: i32 = 150;

/// Dispatch Opcode 0x17 — Inventory operations.
pub async fn handle_inventory(ctx: &mut OpcodeCtx<'_>) {
    let conn = &mut ctx.conn;
    let out = &mut ctx.out;
    let pool = ctx.env.pool;
    let hub = ctx.env.hub;
    let (sub, payload) = (ctx.sub, ctx.payload);
    let decoded = ctx.decoded;
    match sub {
        // Sub 2: Pick up map drop
        2 => handle_pickup(conn, payload, out, pool, hub).await,
        // Sub 3: Drop item
        3 => handle_drop(conn, payload, out, pool, hub).await,
        // Sub 10: Move / stack item (echo raw packet back on success)
        10 => handle_move_stack(conn, payload, decoded, out),
        // Sub 11: Equip player item
        11 => handle_equip(conn, payload, out, pool, hub).await,
        // Sub 12: Unequip player item
        12 => handle_unequip(conn, payload, out, pool, hub).await,
        // Sub 15: Use item
        15 => handle_use_item(conn, payload, out, pool, ctx.data).await,
        // Sub 46: Player reborn
        46 => handle_reborn(conn, payload, out, pool).await,
        _ => {}
    }
}

/// Build the `1706` add-item frame (C# `PickupItemOnMap`).
fn item_added_frame(item: &InventoryItem) -> String {
    // F4440E001706 + le16(id) + count + 00 + doben + long + (giatriLong+100) + khang + le32(texp)
    let body = format!(
        "{:02X}00{:02X}{:02X}{:02X}{:02X}{}",
        item.count,
        item.doben,
        item.long_val,
        (item.giatri_long as u16 + 100) as u8,
        item.khang,
        encoder::le32(item.texp)
    );
    let mut full = String::from("1706");
    full.push_str(&encoder::le16(item.id));
    full.push_str(&body);
    crate::protocol::frame("1706", &full[4..])
}

async fn handle_pickup(
    conn: &mut Conn,
    payload: &[u8],
    out: &mut HandleOutcome,
    pool: Option<&sqlx::MySqlPool>,
    hub: Option<&crate::web::server_control::ServerControl>,
) {
    if conn.session.battle_id > 0 || payload.is_empty() {
        return;
    }
    let map_id = conn.session.map_id;
    let slot = payload[0];
    let Some(drop) = map_drops::get(map_id, slot) else {
        return; // nothing on that map slot
    };
    // Distance gate (C# case 2): within ±150 map units of the player.
    let dx = i32::from(drop.map_x) - i32::from(conn.session.map_x);
    let dy = i32::from(drop.map_y) - i32::from(conn.session.map_y);
    if !(-PICKUP_RANGE..=PICKUP_RANGE).contains(&dx)
        || !(-PICKUP_RANGE..=PICKUP_RANGE).contains(&dy)
    {
        return; // out of range: the drop stays on the map
    }
    // A full bag must leave the drop untouched and reply with nothing (C#
    // `PickupItemOnMap`, Data.cs:3788-3872) — probe on a copy first.
    let mut probe = conn.session.homdo.clone();
    if crate::server::inventory::add_item(&mut probe, drop.item.clone()).is_empty() {
        return;
    }
    let Some(drop) = map_drops::take(map_id, slot) else {
        return;
    };
    let affected = conn.session.add_homdo_item(drop.item.clone());
    // Persist every slot the (possibly straddling) add touched so the merge
    // increment into an existing stack is not lost on reload.
    for hslot in &affected {
        if let Some(added) = conn.session.homdo.iter().find(|i| i.slot == *hslot) {
            persist::upsert_item(pool, conn.session.id, "homdo", added).await;
        }
    }
    {
        // C# `PickupItemOnMap` acks: 1702 (2-byte LE slot) + 1706 to the picker,
        // and a `04001702` removal broadcast to the map. No dump is emitted.
        out.send(format!("F44405001702{}01", encoder::le16(u16::from(slot))));
        out.send(item_added_frame(&drop.item));
        if let Some(hub) = hub {
            hub.broadcast_except(
                conn.session.id,
                &format!("F44404001702{}", encoder::le16(u16::from(slot))),
            )
            .await;
        }
    }
}

async fn handle_drop(
    conn: &mut Conn,
    payload: &[u8],
    out: &mut HandleOutcome,
    pool: Option<&sqlx::MySqlPool>,
    hub: Option<&crate::web::server_control::ServerControl>,
) {
    if conn.session.battle_id > 0 || payload.len() < 2 {
        return;
    }
    let hdslot = payload[0];
    let count = payload[1] as u16;
    let Some(pos) = conn.session.homdo.iter().position(|i| i.slot == hdslot) else {
        return;
    };
    let item = conn.session.homdo[pos].clone();
    if item.id == 0 || item.count == 0 || u16::from(item.count) < count {
        return; // C# `HomdoDropItem`: `count2 >= count && count2 > 0`
    }
    let map_id = conn.session.map_id;
    let x = conn.session.map_x;
    let y = conn.session.map_y;
    // The drop is placed under a per-map allocated slot (1..=255), NOT the homdo
    // slot, so two players can drop onto the same map without colliding (C#
    // `HomdoDropItem`, Data.cs:3511-3562). The client correlates it by (x,y) and
    // echoes that slot back on the pickup. On a full map we refuse silently: the
    // item stays in the bag (C# `num3 > 255` return).
    let mut dropped = item.clone();
    dropped.count = count.min(255) as u8;
    // The map slot is deliberately unused here: the client correlates the drop
    // by (x,y), not by the slot number (the wire carries no slot).
    let Some(_drop_slot) = map_drops::allocate(map_id, dropped.clone(), x, y) else {
        return;
    };

    let rem = item.count - dropped.count;
    if rem > 0 {
        conn.session.homdo[pos].count = rem;
    } else {
        conn.session.homdo.remove(pos);
    }
    // C# `HomdoDropItem`: self 1703 (drop at x,y) + 1709 (slot, remaining);
    // map peers see the 1703-only variant.
    out.send(format!(
        "F44409001703{}{}{}01",
        encoder::le16(item.id),
        encoder::le16(x),
        encoder::le16(y)
    ));
    out.send(format!("F44404001709{:02X}{:02X}", hdslot, rem));
    if let Some(hub) = hub {
        let frame = format!(
            "F44408001703{}{}{}",
            encoder::le16(item.id),
            encoder::le16(x),
            encoder::le16(y)
        );
        hub.broadcast_except(conn.session.id, &frame).await;
    }
    // Persist either the reduced count or the cleared slot.
    match conn.session.homdo.iter().find(|i| i.slot == hdslot) {
        Some(kept) => persist::upsert_item(pool, conn.session.id, "homdo", kept).await,
        None => {
            let empty = InventoryItem {
                slot: hdslot,
                ..Default::default()
            };
            persist::upsert_item(pool, conn.session.id, "homdo", &empty).await;
        }
    }
}

fn handle_move_stack(conn: &mut Conn, payload: &[u8], decoded: &[u8], out: &mut HandleOutcome) {
    // C# `HomdoMoveItem(oldslot, count, newslot)`: payload[0]=old, [1]=count, [2]=new.
    if payload.len() < 3 {
        return;
    }
    let oldslot = payload[0];
    let count = payload[1] as u16;
    let newslot = payload[2];
    let Some(src_idx) = conn.session.homdo.iter().position(|i| i.slot == oldslot) else {
        return;
    };
    let src = conn.session.homdo[src_idx].clone();
    if src.id == 0 || src.count == 0 {
        return;
    }
    let dst_idx = conn.session.homdo.iter().position(|i| i.slot == newslot);
    let src_count = u16::from(src.count);
    if (1..=6).contains(&src.loai) {
        // Equipment: only moves to an empty slot.
        if dst_idx.is_none() {
            conn.session.homdo[src_idx].slot = newslot;
            out.send(hex_of(decoded));
        }
        return;
    }
    // Stackable: dst empty or same id, dst count < 50, and merge ≤ 50.
    let dst = dst_idx.map(|i| conn.session.homdo[i].clone());
    match dst {
        None => {
            conn.session.homdo[src_idx].slot = newslot;
            out.send(hex_of(decoded));
        }
        Some(d) => {
            if d.id != src.id || d.count >= 50 {
                return;
            }
            let dst_count = u16::from(d.count);
            if count + dst_count > 50 {
                return;
            }
            if count == src_count {
                // Move all: swap the two slots.
                conn.session.homdo[src_idx].slot = newslot;
                if let Some(di) = dst_idx {
                    conn.session.homdo[di].slot = oldslot;
                }
                conn.session.homdo.swap(src_idx, dst_idx.unwrap());
            } else {
                // Split: dst = count + dst_count, src = src_count - count.
                conn.session.homdo[src_idx].count = (src_count - count) as u8;
                conn.session.homdo[dst_idx.unwrap()].count = (count + dst_count) as u8;
            }
            out.send(hex_of(decoded));
        }
    }
}

async fn handle_equip(
    conn: &mut Conn,
    payload: &[u8],
    out: &mut HandleOutcome,
    pool: Option<&sqlx::MySqlPool>,
    hub: Option<&crate::web::server_control::ServerControl>,
) {
    if conn.session.battle_id > 0 || payload.is_empty() {
        return;
    }
    let homdo_slot = payload[0];
    let Some(pos) = conn.session.homdo.iter().position(|i| i.slot == homdo_slot) else {
        return;
    };
    let item = conn.session.homdo[pos].clone();
    let loai = item.loai;
    // C# case 11 gates: id > 0, loai 1..=6, and player level >= item level.
    if item.id == 0 || !(1..=6).contains(&loai) || conn.session.level < item.lv {
        return;
    }

    conn.session.homdo.remove(pos);
    // Unequip the current occupant of that equip slot into homdo.
    if let Some(old_pos) = conn.session.trangbi.iter().position(|i| i.slot == loai) {
        let mut old_item = conn.session.trangbi.remove(old_pos);
        old_item.slot = homdo_slot;
        persist::upsert_item(pool, conn.session.id, "homdo", &old_item).await;
        conn.session.homdo.push(old_item);
    }

    let mut equipped = item;
    equipped.slot = loai;
    conn.session.trangbi.push(equipped.clone());
    persist::upsert_item(pool, conn.session.id, "trangbi", &equipped).await;

    out.send(format!("F44403001711{:02X}", homdo_slot));
    conn.session.recompute_stats();
    out.send(conn.session.dump_trangbi());
    // C# `UpdateStatusWhenUseItem` → PlayerUpdateDataId sends HP/SP max + gear 2-stats.
    out.send(build_stat_update(0x19, conn.session.hp_max as i32));
    out.send(build_stat_update(0x1A, conn.session.sp_max as i32));
    out.send(build_stat_update(0xCF, conn.session.hpx2 as i32));
    out.send(build_stat_update(0xD0, conn.session.spx2 as i32));
    out.send(build_stat_update(0xD2, conn.session.atk2 as i32));
    out.send(build_stat_update(0xD3, conn.session.def2 as i32));
    out.send(build_stat_update(0xD4, conn.session.int2 as i32));
    out.send(build_stat_update(0xD6, conn.session.agi2 as i32));
    if let Some(hub) = hub {
        // C# `ServerSend_EquitItem`: `F44408000502` + id + item.
        hub.broadcast_except(
            conn.session.id,
            &format!(
                "{}0502{}{}",
                "F4440800",
                encoder::le32(conn.session.id),
                encoder::le16(equipped.id)
            ),
        )
        .await;
    }
}

async fn handle_unequip(
    conn: &mut Conn,
    payload: &[u8],
    out: &mut HandleOutcome,
    pool: Option<&sqlx::MySqlPool>,
    hub: Option<&crate::web::server_control::ServerControl>,
) {
    if conn.session.battle_id > 0 || payload.len() < 2 {
        return;
    }
    let trangbi_slot = payload[0];
    let homdo_slot = payload[1];
    // C# case 12 gate: the target homdo slot must be empty.
    if conn
        .session
        .homdo
        .iter()
        .any(|i| i.slot == homdo_slot && i.id > 0)
    {
        return;
    }
    let Some(pos) = conn
        .session
        .trangbi
        .iter()
        .position(|i| i.slot == trangbi_slot)
    else {
        return;
    };
    let mut item = conn.session.trangbi.remove(pos);
    item.slot = homdo_slot;
    conn.session.homdo.push(item.clone());
    persist::upsert_item(pool, conn.session.id, "trangbi", &item).await;
    let empty = InventoryItem {
        slot: trangbi_slot,
        ..Default::default()
    };
    persist::upsert_item(pool, conn.session.id, "trangbi", &empty).await;
    persist::upsert_item(pool, conn.session.id, "homdo", &item).await;

    out.send(format!(
        "F44404001710{:02X}{:02X}",
        trangbi_slot, homdo_slot
    ));
    conn.session.recompute_stats();
    out.send(conn.session.dump_trangbi());
    out.send(conn.session.dump_homdo());
    out.send(build_stat_update(0x19, conn.session.hp_max as i32));
    out.send(build_stat_update(0x1A, conn.session.sp_max as i32));
    out.send(build_stat_update(0xCF, conn.session.hpx2 as i32));
    out.send(build_stat_update(0xD0, conn.session.spx2 as i32));
    out.send(build_stat_update(0xD2, conn.session.atk2 as i32));
    out.send(build_stat_update(0xD3, conn.session.def2 as i32));
    out.send(build_stat_update(0xD4, conn.session.int2 as i32));
    out.send(build_stat_update(0xD6, conn.session.agi2 as i32));
    if let Some(hub) = hub {
        // C# `ServerSend_UnEquitItem`: `F44408000501` + id + item.
        hub.broadcast_except(
            conn.session.id,
            &format!(
                "F44408000501{}{}",
                encoder::le32(conn.session.id),
                encoder::le16(item.id)
            ),
        )
        .await;
    }
}

async fn handle_use_item(
    conn: &mut Conn,
    payload: &[u8],
    out: &mut HandleOutcome,
    pool: Option<&sqlx::MySqlPool>,
    data: &crate::data::loader::GameData,
) {
    super::use_item::use_item(conn, payload, out, pool, data).await;
}

pub fn is_reborn_special_skill(id: u16) -> bool {
    matches!(id, 10016..=10019 | 11016..=11019 | 12016..=12019 | 13015..=13018)
}

async fn handle_reborn(
    conn: &mut Conn,
    payload: &[u8],
    out: &mut HandleOutcome,
    pool: Option<&sqlx::MySqlPool>,
) {
    // Requires no equipment in trangbi slots 1..6
    if conn
        .session
        .trangbi
        .iter()
        .any(|i| (1..=6).contains(&i.slot) && i.id > 0)
    {
        let npc_id = encoder::le16(conn.session.idtalking as u16);
        out.send(format!("F44411001401000000010103{}00000000002451", npc_id));
        conn.session.select_menu = 40;
        return;
    }

    if !payload.is_empty() {
        conn.session.hair = u16::from(payload[0]);
    }
    if payload.len() >= 9 {
        conn.session.color = crate::server::handler::hex_of(&payload[1..9]);
    }

    let (point_base, skill_point_base, new_reborn, new_job) = if conn.session.reborn == 0 {
        (0u32, 24u32, 1u8, conn.session.job)
    } else {
        let job = if conn.session.select_menu >= 30 {
            (conn.session.select_menu - 30 + 1) as u8
        } else {
            conn.session.job.max(1)
        };
        (24u32, 118u32, 2u8, job)
    };

    let extra_levels = (u32::from(conn.session.level).saturating_sub(120)) / 5;
    conn.session.level = 1;
    conn.session.reborn = new_reborn;
    conn.session.job = new_job;
    conn.session.point = (point_base + extra_levels) as u16;
    conn.session.skill_point = (skill_point_base + extra_levels) as u16;
    conn.session.hp_max = 181;
    conn.session.hp = 181;
    conn.session.sp_max = 181;
    conn.session.sp = 181;
    conn.session.int1 = 0;
    conn.session.atk = 0;
    conn.session.def = 0;
    conn.session.hpx = 0;
    conn.session.spx = 0;
    conn.session.agi = 0;
    conn.session.texp = 13;

    // Retain only special reborn/passive skills
    conn.session
        .skills
        .retain(|(id, _)| is_reborn_special_skill(*id));

    conn.session.recompute_stats();

    persist::update_player(pool, conn.session.id, "Lv", 1).await;
    persist::update_player(
        pool,
        conn.session.id,
        "Point",
        i64::from(conn.session.point),
    )
    .await;
    persist::update_player(
        pool,
        conn.session.id,
        "SkillPoint",
        i64::from(conn.session.skill_point),
    )
    .await;
    persist::update_player(pool, conn.session.id, "Hp", 181).await;
    persist::update_player(pool, conn.session.id, "HpMax", 181).await;
    persist::update_player(pool, conn.session.id, "Sp", 181).await;
    persist::update_player(pool, conn.session.id, "SpMax", 181).await;
    persist::update_player(pool, conn.session.id, "Int", 0).await;
    persist::update_player(pool, conn.session.id, "Atk", 0).await;
    persist::update_player(pool, conn.session.id, "Def", 0).await;
    persist::update_player(pool, conn.session.id, "Hpx", 0).await;
    persist::update_player(pool, conn.session.id, "Spx", 0).await;
    persist::update_player(pool, conn.session.id, "Agi", 0).await;
    persist::update_player(pool, conn.session.id, "Texp", 13).await;
    persist::delete_reborn_skills(pool, conn.session.id).await;

    out.send("F44402002C01");
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::battle::service::BattleService;
    use crate::data::loader::GameData;
    use crate::server::handler::{test_ctx, HandleOutcome};
    use crate::server::session::Conn;
    use std::sync::Arc;

    fn bag_item(slot: u8, id: u16, count: u8, loai: u8) -> InventoryItem {
        InventoryItem {
            slot,
            id,
            count,
            loai,
            ..Default::default()
        }
    }

    async fn run(conn: &mut Conn, sub: u8, payload: &[u8]) -> HandleOutcome {
        let data = GameData::default();
        let service = BattleService::new(Arc::new(GameData::default()));
        let mut out = HandleOutcome::default();
        let mut ctx = test_ctx(conn, &data, &service, &mut out, sub, payload);
        handle_inventory(&mut ctx).await;
        out
    }

    #[tokio::test]
    async fn equip_respects_level_gate() {
        let mut conn = Conn::new();
        conn.session.level = 5;
        conn.session.homdo.push(InventoryItem {
            slot: 1,
            id: 12001,
            count: 1,
            lv: 10,
            loai: 1,
            ..Default::default()
        });
        let out = run(&mut conn, 11, &[1]).await;
        assert!(conn.session.trangbi.is_empty(), "below level: no equip");
        assert!(out.outgoing.is_empty());
    }

    #[tokio::test]
    async fn equip_moves_to_trangbi_and_sends_stats() {
        let mut conn = Conn::new();
        conn.session.level = 10;
        conn.session.homdo.push(InventoryItem {
            slot: 1,
            id: 12001,
            count: 1,
            lv: 1,
            loai: 1,
            atk1: 15,
            ..Default::default()
        });
        let out = run(&mut conn, 11, &[1]).await;
        assert_eq!(conn.session.trangbi.len(), 1);
        assert_eq!(conn.session.trangbi[0].id, 12001);
        assert!(conn.session.homdo.is_empty());
        assert_eq!(conn.session.atk2, 15);
        assert!(out.outgoing.iter().any(|f| f.contains("171101")));
        // Gear stat packets include the HP/SP max recompute frames.
        assert!(out.outgoing.iter().any(|f| f.contains("08011A")));
        assert!(out.outgoing.iter().any(|f| f.contains("080119")));
    }

    #[tokio::test]
    async fn unequip_requires_empty_destination_slot() {
        let mut conn = Conn::new();
        conn.session.homdo.push(bag_item(2, 5001, 1, 0));
        conn.session.trangbi.push(InventoryItem {
            slot: 1,
            id: 12001,
            count: 1,
            lv: 1,
            loai: 1,
            ..Default::default()
        });
        // Destination homdo slot 2 is occupied -> rejected.
        let out = run(&mut conn, 12, &[1, 2]).await;
        assert_eq!(
            conn.session.trangbi.len(),
            1,
            "must not unequip onto a full slot"
        );
        assert!(out.outgoing.is_empty());
    }

    #[tokio::test]
    async fn unequip_moves_to_empty_homdo_slot() {
        let mut conn = Conn::new();
        conn.session.homdo.push(bag_item(2, 0, 0, 0));
        conn.session.trangbi.push(InventoryItem {
            slot: 1,
            id: 12001,
            count: 1,
            lv: 1,
            loai: 1,
            atk1: 15,
            ..Default::default()
        });
        let out = run(&mut conn, 12, &[1, 2]).await;
        assert!(conn.session.trangbi.is_empty());
        assert_eq!(conn.session.homdo[0].slot, 2);
        assert_eq!(conn.session.atk2, 0);
        assert!(out.outgoing.iter().any(|f| f.contains("17100102")));
    }

    #[tokio::test]
    async fn move_stack_splits_counts() {
        let mut conn = Conn::new();
        conn.session.homdo.push(bag_item(1, 100, 20, 0));
        conn.session.homdo.push(bag_item(2, 100, 10, 0));
        // move 5 from slot 1 to slot 2 (loai 0 -> stackable, total 15 <= 50)
        let decoded = encoder::bytes("F4440700170A010502").unwrap();
        let data = GameData::default();
        let service = BattleService::new(Arc::new(GameData::default()));
        let mut out = HandleOutcome::default();
        let mut ctx = test_ctx(&mut conn, &data, &service, &mut out, 10, &decoded[6..]);
        // decoded is only used for the echo, so feed it via a raw frame anyway.
        ctx.decoded = &decoded;
        handle_inventory(&mut ctx).await;

        assert_eq!(
            conn.session
                .homdo
                .iter()
                .find(|i| i.slot == 1)
                .unwrap()
                .count,
            15
        );
        assert_eq!(
            conn.session
                .homdo
                .iter()
                .find(|i| i.slot == 2)
                .unwrap()
                .count,
            15
        );
        assert_eq!(out.outgoing, vec![encoder::hex(&decoded)]);
    }

    #[tokio::test]
    async fn pickup_requires_distance_gate() {
        crate::server::map_drops::clear_all();
        let mut conn = Conn::new();
        conn.session.map_id = 12009;
        conn.session.map_x = 1000;
        conn.session.map_y = 1000;
        crate::server::map_drops::drop(12009, 1, bag_item(1, 1001, 1, 0), 400, 500);
        let out = run(&mut conn, 2, &[1]).await;
        assert!(conn.session.homdo.is_empty(), "out of range: no pickup");
        assert!(out.outgoing.is_empty());
        assert!(
            crate::server::map_drops::get(12009, 1).is_some(),
            "drop must stay on the map when out of range"
        );
        crate::server::map_drops::clear_all();
    }

    #[tokio::test]
    async fn pickup_adds_item_to_homdo() {
        crate::server::map_drops::clear_all();
        let mut conn = Conn::new();
        conn.session.map_id = 12010;
        conn.session.map_x = 400;
        conn.session.map_y = 500;
        crate::server::map_drops::drop(12010, 1, bag_item(1, 1001, 2, 0), 400, 500);
        let out = run(&mut conn, 2, &[1]).await;
        assert_eq!(conn.session.homdo.len(), 1);
        assert_eq!(conn.session.homdo[0].id, 1001);
        assert_eq!(conn.session.homdo[0].count, 2);
        assert!(
            crate::server::map_drops::get(12010, 1).is_none(),
            "drop consumed by pickup"
        );
        assert!(out.outgoing.iter().any(|f| f.contains("1702")));
        assert!(out.outgoing.iter().any(|f| f.contains("1706")));
        crate::server::map_drops::clear_all();
    }

    #[tokio::test]
    async fn drop_creates_map_drop_and_reduces_count() {
        crate::server::map_drops::clear_all();
        let mut conn = Conn::new();
        conn.session.map_id = 12011;
        conn.session.map_x = 400;
        conn.session.map_y = 500;
        conn.session.homdo.push(bag_item(3, 1001, 5, 0));
        let out = run(&mut conn, 3, &[3, 2]).await;
        assert_eq!(conn.session.homdo[0].count, 3, "dropped 2 of 5");
        // The drop lands under a freshly allocated per-map slot (first free = 1),
        // not under the homdo slot, and carries the dropped count (2).
        let drop = crate::server::map_drops::get(12011, 1).expect("drop on map");
        assert_eq!(drop.item.id, 1001);
        assert_eq!(drop.item.count, 2);
        assert!(out.outgoing.iter().any(|f| f.contains("17090303")));
        crate::server::map_drops::clear_all();
    }

    #[tokio::test]
    async fn drop_refuses_when_map_full_keeps_item() {
        crate::server::map_drops::clear_all();
        let mut conn = Conn::new();
        conn.session.map_id = 12012;
        conn.session.map_x = 400;
        conn.session.map_y = 500;
        conn.session.homdo.push(bag_item(3, 1001, 5, 0));
        // Fill every map slot 1..=255.
        for slot in 1..=255u8 {
            crate::server::map_drops::drop(12012, slot, bag_item(slot, 7000, 1, 0), 0, 0);
        }
        let out = run(&mut conn, 3, &[3, 2]).await;
        assert_eq!(conn.session.homdo[0].count, 5, "full map: item kept");
        assert!(out.outgoing.is_empty(), "full map: silent, no frames");
        crate::server::map_drops::clear_all();
    }

    #[tokio::test]
    async fn reborn_rejected_if_equipped() {
        let mut conn = Conn::new();
        conn.session.trangbi.push(InventoryItem {
            slot: 1,
            id: 12001,
            count: 1,
            lv: 1,
            loai: 1,
            ..Default::default()
        });
        let out = run(&mut conn, 46, &[]).await;
        assert_eq!(conn.session.reborn, 0);
        assert!(out.outgoing.iter().any(|f| f.contains("1401")));
    }

    #[tokio::test]
    async fn reborn_resets_stats_retains_special_skills() {
        let mut conn = Conn::new();
        conn.session.level = 125;
        conn.session.reborn = 0;
        conn.session.skills = vec![(10001, 10), (10016, 10)]; // 10001 normal, 10016 special
        let out = run(&mut conn, 46, &[10, 0, 0, 0, 0, 0, 0, 0, 0]).await;

        assert_eq!(conn.session.reborn, 1);
        assert_eq!(conn.session.level, 1);
        assert_eq!(conn.session.point, 1); // 0 + (125-120)/5 = 1
        assert_eq!(conn.session.skill_point, 25); // 24 + (125-120)/5 = 25
        assert_eq!(conn.session.skills.len(), 1);
        assert_eq!(conn.session.skills[0], (10016, 10)); // special skill retained
        assert!(out.outgoing.contains(&"F44402002C01".to_string()));
    }
}
