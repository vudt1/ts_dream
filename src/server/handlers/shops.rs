//! NPC shop buy/sell (Opcode 0x1B) & Player shop (Opcode 0x17 subs 30–33) handlers.

use crate::db::persist;
use crate::protocol::{encoder, GOLD_CAP};
use crate::server::dispatcher::OpcodeCtx;
use crate::server::session::{Conn, InventoryItem};

/// `F44406001A04` + gold — the gold-update frame sent after
/// every successful shop transaction (also used by the Eve gold action in
/// `handlers::talk`). Gold is clamped to GOLD_CAP (9,999,999) to prevent client desync.
pub fn gold_frame(gold: u32) -> String {
    let clamped_gold = gold.min(GOLD_CAP);
    format!(
        "F44406001A04{}",
        encoder::le32(clamped_gold)
    )
}

/// `F44406001A03` + point — shop point sync frame (6 bytes wire).
pub fn point_frame(point: u32) -> String {
    format!(
        "F44406001A03{}",
        encoder::le32(point)
    )
}

/// Helper building S->C `1B 03` frame opening the NPC shop dialogue form.
pub fn open_npc_shop_frame() -> &'static str {
    "F44402001B03"
}

/// Helper building NPC shop Buy result packet S->C `1B 01 <code>`.
pub fn npc_shop_buy_result(code: u8) -> String {
    format!("F44403001B01{:02X}", code)
}

/// Helper building NPC shop Sell result packet S->C `1B 02 <code>`.
pub fn npc_shop_sell_result(code: u8) -> String {
    format!("F44403001B02{:02X}", code)
}

/// VISCII-encoded system message frame via channel `020102`.
/// Delegates to standard `crate::server::spawn::sys_msg_frame`.
fn red_message(msg: &str) -> String {
    crate::server::spawn::sys_msg_frame(msg)
}

/// Lookup NPC shop entry by (idtalking, map_id, menu) → (item_id, price).
/// The table below is transcribed verbatim from the legacy price list.
pub fn get_npc_shop_price(idtalking: i32, map_id: u16, menu: u8) -> Option<(u16, u32)> {
    let row = (idtalking, map_id, menu);
    let price = match row {
        // (1, 12223) — NPC 1 shop thuốc hp
        (1, 12223, 0) => (26041, 5),
        (1, 12223, 1) => (27017, 5),
        (1, 12223, 2) => (27003, 10),
        (1, 12223, 3) => (27032, 15),
        // (1, 19241)
        (1, 19241, 0) => (26041, 5),
        (1, 19241, 1) => (26047, 10),
        (1, 19241, 2) => (26051, 5),
        (1, 19241, 3) => (26053, 5),
        (1, 19241, 4) => (26028, 10),
        // (4, 12002)
        (4, 12002, 0) => (26016, 5),
        (4, 12002, 1) => (26044, 10),
        (4, 12002, 2) => (26068, 20),
        // (16, 12002) — shop trang bị
        (16, 12002, 0) => (20023, 58800),
        (16, 12002, 1) => (19723, 58800),
        (16, 12002, 2) => (19755, 58800),
        (16, 12002, 3) => (19759, 58800),
        (16, 12002, 4) => (22023, 58800),
        (16, 12002, 5) => (21723, 58800),
        (16, 12002, 6) => (19023, 58800),
        (16, 12002, 7) => (20423, 58800),
        (16, 12002, 8) => (22423, 58800),
        (16, 12002, 9) => (21423, 58800),
        (16, 12002, 10) => (21218, 58800),
        (16, 12002, 11) => (22723, 58800),
        (16, 12002, 12) => (21023, 58800),
        (16, 12002, 13) => (20723, 58800),
        (16, 12002, 14) => (19423, 58800),
        // (15, 12002)
        (15, 12002, 0) => (20420, 19900),
        (15, 12002, 1) => (19420, 19900),
        (15, 12002, 2) => (20720, 19900),
        (15, 12002, 3) => (22707, 19900),
        (15, 12002, 4) => (21020, 19900),
        (15, 12002, 5) => (21215, 19900),
        (15, 12002, 6) => (21420, 19900),
        (15, 12002, 7) => (21720, 19900),
        (15, 12002, 8) => (22020, 19900),
        (15, 12002, 9) => (22420, 19900),
        (15, 12002, 10) => (13020, 19900),
        (15, 12002, 11) => (19020, 19900),
        (15, 12002, 12) => (19756, 19900),
        (15, 12002, 13) => (19720, 19900),
        (15, 12002, 14) => (19752, 19900),
        (15, 12002, 15) => (20020, 19900),
        // (8, 12990)
        (8, 12990, 0) => (19001, 10),
        (8, 12990, 1) => (19701, 10),
        (8, 12990, 2) => (20001, 10),
        (8, 12990, 3) => (20011, 10),
        (8, 12990, 4) => (20401, 10),
        (8, 12990, 5) => (20411, 10),
        (8, 12990, 6) => (22701, 10),
        (8, 12990, 7) => (22711, 10),
        // (1, 12201) — shop mua vũ khí
        (1, 12201, 0) => (10001, 10),
        (1, 12201, 1) => (12011, 10),
        (1, 12201, 2) => (10013, 20),
        (1, 12201, 3) => (13012, 20),
        // (3, 12244)
        (3, 12244, 0) => (26001, 5),
        (3, 12244, 1) => (26004, 10),
        (3, 12244, 2) => (26005, 15),
        (3, 12244, 3) => (26026, 5),
        // (1, 12007)
        (1, 12007, 0) => (26075, 5),
        (1, 12007, 1) => (26042, 10),
        (1, 12007, 2) => (26040, 5),
        (1, 12007, 3) => (26037, 15),
        // (1, 12204) — shop vũ khí
        (1, 12204, 0) => (10001, 10),
        (1, 12204, 1) => (10002, 10),
        (1, 12204, 2) => (15001, 10),
        // (2, 12204) — shop trang bị
        (2, 12204, 0) => (19011, 10),
        (2, 12204, 1) => (19401, 10),
        (2, 12204, 2) => (19411, 10),
        (2, 12204, 3) => (21011, 10),
        (2, 12204, 4) => (21411, 10),
        (2, 12204, 5) => (21401, 10),
        (2, 12204, 6) => (22001, 10),
        (2, 12204, 7) => (22011, 10),
        // (7, 12001)
        (7, 12001, 0) => (18001, 10),
        (7, 12001, 1) => (27156, 115),
        (7, 12001, 2) => (52015, 1),
        // (2, 20001)
        (2, 20001, 0) => (18001, 10),
        (2, 20001, 1) => (18002, 20),
        (2, 20001, 2) => (18003, 50),
        (2, 20001, 3) => (46103, 100),
        // (26, 11011)
        (26, 11011, 0) => (19402, 20),
        (26, 11011, 1) => (19412, 20),
        (26, 11011, 2) => (20702, 20),
        (26, 11011, 3) => (20712, 20),
        (26, 11011, 4) => (21002, 20),
        (26, 11011, 5) => (21012, 20),
        (26, 11011, 6) => (22402, 20),
        (26, 11011, 7) => (22412, 20),
        _ => return None,
    };
    Some(price)
}

/// Resolve NPC shop entry by querying `scene_eve_data` SurfaceData or falling back to `get_npc_shop_price`.
/// Lookup is strictly narrowed to the interacting NPC's surface (`last_surface_id` or `idtalking`)
/// to prevent crosstalk between different shops on the same map.
pub fn resolve_npc_shop_item(
    data: &crate::data::loader::GameData,
    conn: &Conn,
    map_id: u16,
    idtalking: i32,
    item_index: u8,
) -> Option<(u16, u32)> {
    // 1. Check scene_eve_data surface data for the interacting NPC
    if let Some(scene) = data.scene_eve_data.get(&u32::from(map_id)) {
        let surface_id = conn
            .session
            .current_event_session
            .as_ref()
            .and_then(|ev| if ev.last_surface_id >= 0 { Some(ev.last_surface_id as u16) } else { None })
            .or_else(|| if idtalking > 0 { Some(idtalking as u16) } else { None });

        if let Some(sid) = surface_id {
            if let Some(surface) = scene.surface_datas.get(&sid) {
                if let Some(sentence) = surface.sentences.get(&item_index) {
                    let item_id = sentence.data_id;
                    let price = data.item_defs.get(&item_id).map(|d| d.price as u32).unwrap_or(0);
                    if price > 0 {
                        return Some((item_id, price));
                    }
                }
            }
        }
    }
    // 2. Fallback to transcribed price list
    if let Some((item_id, fallback_price)) = get_npc_shop_price(idtalking, map_id, item_index) {
        let price = data.item_defs.get(&item_id).map(|d| d.price as u32).filter(|&p| p > 0).unwrap_or(fallback_price);
        return Some((item_id, price));
    }
    None
}

/// Handle Opcode 0x1B — NPC shop buy/sell.
///
/// Dispatches on `ctx.sub`:
/// - `NPC_SHOP_SUB_BUY` (1): Buy item. Reads `item_index: u8` and `count: u16 LE`.
///   Resolves item/price from `scene_eve_data` and `item_defs`, deducting gold and sending
///   `1A 02`, `1A 04`, `17 06`, and `1B 01 00`.
/// - `NPC_SHOP_SUB_SELL` (2): Sell item. Reads `slot: u8` and `qty: u8`.
///   Credits gold based on `qty * sell_price` (clamped to 9,999,999) and sends
///   `1A 01`, `1A 04`, `17 09`, and `1B 02 00`.
///
/// NOTE: The starter bundle branch for NPC ID 7 on Map 9999 is retained as an intentional
/// developer/testing and local newbie convenience fallback when NPC 7 does not have
/// surface entries in `eve.emg`.
pub async fn handle_npc_shop(ctx: &mut OpcodeCtx<'_>) {
    let sub = ctx.sub;
    let payload = ctx.payload;
    let conn = &mut ctx.conn;
    let out = &mut ctx.out;
    let pool = ctx.env.pool;
    let data = ctx.data;
    let idtalking = conn.session.idtalking;
    let map_id = conn.session.map_id;

    if payload.is_empty() {
        return;
    }

    match sub {
        crate::protocol::NPC_SHOP_SUB_BUY => {
            let item_index = payload.first().copied().unwrap_or(0);
            let count = if payload.len() >= 3 {
                encoder::u16_le(payload[1], payload[2])
            } else {
                payload.get(1).copied().unwrap_or(1) as u16
            };
            if count == 0 {
                out.send(npc_shop_buy_result(crate::protocol::NPC_SHOP_BUY_DISRUPTED));
                return;
            }

            if count > 1250 {
                out.send(npc_shop_buy_result(crate::protocol::NPC_SHOP_BUY_QTY_FULL));
                return;
            }

            // Free starter bundle fallback (idtalking == 7 && map == 9999).
            // Retained for local testing/dev environments when NPC 7 on map 9999 lacks surface data.
            if idtalking == 7 && map_id == 9999 {
                let before = conn.session.clone();
                let mut granted_items = Vec::new();
                for (item_id, n) in [(18001u16, 1u8), (27156, 50), (52015, 50)] {
                    let item = crate::server::inventory::from_template(data, item_id, n);
                    if !crate::server::inventory::can_add_item(&conn.session.homdo, &item)
                        || conn.session.add_homdo_item(item.clone()).is_empty()
                    {
                        conn.session = before;
                        out.send(npc_shop_buy_result(crate::protocol::NPC_SHOP_BUY_BAG_FULL));
                        return;
                    }
                    granted_items.push(item);
                }
                if !persist::persist_shop_transaction(
                    pool,
                    conn.session.id,
                    conn.session.gold,
                    &conn.session.homdo,
                    None,
                    None,
                    None,
                )
                .await
                {
                    conn.session = before;
                    out.send(npc_shop_buy_result(crate::protocol::NPC_SHOP_BUY_DISRUPTED));
                    return;
                }
                for item in &granted_items {
                    out.send(crate::server::inventory::item_added_frame(item));
                }
                out.send(gold_frame(conn.session.gold));
                out.send(npc_shop_buy_result(crate::protocol::NPC_SHOP_BUY_SUCCESS));
                return;
            }

            let item_opt = resolve_npc_shop_item(data, conn, map_id, idtalking, item_index);
            let Some((item_id, unit_price)) = item_opt else {
                out.send(npc_shop_buy_result(crate::protocol::NPC_SHOP_BUY_DISRUPTED));
                return;
            };

            let total_cost = (unit_price as u64).saturating_mul(u64::from(count));
            if total_cost > u64::from(u32::MAX) || (conn.session.gold as u64) < total_cost {
                out.send(npc_shop_buy_result(crate::protocol::NPC_SHOP_BUY_INSUFFICIENT_GOLD));
                return;
            }
            let total_cost = total_cost as u32;

            let slots_needed = ((count as usize) + 49) / 50;
            let occupied_slots: std::collections::HashSet<u8> = conn
                .session
                .homdo
                .iter()
                .filter(|i| i.id > 0 && i.count > 0)
                .map(|i| i.slot)
                .collect();
            let empty_slots = (1..=crate::server::inventory::HOMDO_SLOTS)
                .filter(|s| !occupied_slots.contains(s))
                .count();

            if empty_slots < slots_needed {
                out.send(npc_shop_buy_result(crate::protocol::NPC_SHOP_BUY_BAG_FULL));
                return;
            }

            let before = conn.session.clone();
            conn.session.gold -= total_cost;

            let mut remaining = count;
            let mut added_items = Vec::new();
            while remaining > 0 {
                let chunk_size = remaining.min(50) as u8;
                remaining -= u16::from(chunk_size);
                let Some(slot) = crate::server::inventory::free_slot(&conn.session.homdo) else {
                    conn.session = before;
                    out.send(npc_shop_buy_result(crate::protocol::NPC_SHOP_BUY_BAG_FULL));
                    return;
                };
                let mut item = crate::server::inventory::from_template(data, item_id, chunk_size);
                item.slot = slot;
                conn.session.homdo.push(item.clone());
                added_items.push(item);
            }

            if !persist::persist_shop_transaction(
                pool,
                conn.session.id,
                conn.session.gold,
                &conn.session.homdo,
                None,
                None,
                None,
            )
            .await
            {
                conn.session = before;
                out.send(npc_shop_buy_result(crate::protocol::NPC_SHOP_BUY_DISRUPTED));
                return;
            }

            // Wire sequence: 1A 02 -> 1A 04 -> multiple 17 06 -> 1B 01 00
            out.send(format!("F44406001A02{}", encoder::le32(total_cost)));
            out.send(gold_frame(conn.session.gold));
            for item in &added_items {
                out.send(crate::server::inventory::item_added_frame(item));
            }
            out.send(npc_shop_buy_result(crate::protocol::NPC_SHOP_BUY_SUCCESS));
        }
        crate::protocol::NPC_SHOP_SUB_SELL => {
            let slot = payload.first().copied().unwrap_or(0);
            let qty = payload.get(1).copied().unwrap_or(0);
            if slot == 0 || qty == 0 {
                return;
            }

            let Some(pos) = conn
                .session
                .homdo
                .iter()
                .position(|i| i.slot == slot && i.id > 0)
            else {
                out.send(npc_shop_sell_result(crate::protocol::NPC_SHOP_SELL_ITEM_NOT_FOUND));
                return;
            };

            let item = conn.session.homdo[pos].clone();
            if item.count < qty {
                out.send(npc_shop_sell_result(crate::protocol::NPC_SHOP_SELL_QTY_NOT_ENOUGH));
                return;
            }

            let unit_sell_price = data
                .item_defs
                .get(&item.id)
                .map(|d| d.sell_price as u32)
                .unwrap_or(1);
            let revenue = (unit_sell_price as u64).saturating_mul(u64::from(qty));
            if (u64::from(conn.session.gold)).saturating_add(revenue) > u64::from(GOLD_CAP) {
                out.send(npc_shop_sell_result(crate::protocol::NPC_SHOP_SELL_GOLD_FULL_ALT));
                return;
            }
            let revenue = revenue as u32;

            let before = conn.session.clone();
            conn.session.gold = (conn.session.gold.saturating_add(revenue)).min(GOLD_CAP);
            if conn.session.homdo[pos].count == qty {
                conn.session.homdo.remove(pos);
            } else {
                conn.session.homdo[pos].count -= qty;
            }

            if !persist::persist_shop_transaction(
                pool,
                conn.session.id,
                conn.session.gold,
                &conn.session.homdo,
                None,
                None,
                None,
            )
            .await
            {
                conn.session = before;
                out.send(npc_shop_sell_result(crate::protocol::NPC_SHOP_SELL_DISRUPTED));
                return;
            }

            // Wire sequence: 1A 01 -> 1A 04 -> 17 09 -> 1B 02 00
            out.send(format!("F44406001A01{}", encoder::le32(revenue)));
            out.send(gold_frame(conn.session.gold));
            out.send(format!("F44404001709{:02X}{:02X}", slot, qty));
            out.send(npc_shop_sell_result(crate::protocol::NPC_SHOP_SELL_SUCCESS));
        }
        _ => {}
    }
}

/// Player shop catalog frame `1721`:
/// Header (17 bytes - 0x11): SubOp 0x21 (1B) + seller_id (4B LE) + 12 zero bytes.
/// Entries: up to 10 items, each 16 bytes:
/// slot (1B), itemId (2B LE), count (1B), long_val (1B), price (4B LE),
/// giatri_long (1B), khang (1B), pad (1B), texp (4B LE).
pub fn player_shop_catalog_frame(seller: &crate::server::session::Session) -> String {
    let mut body = format!("{}{}", encoder::le32(seller.id), "00".repeat(12));
    let mut idx = 1u8;
    for listing in seller.shop.items.iter().take(10) {
        let item = seller
            .homdo
            .iter()
            .find(|i| i.slot == listing.slot && i.id > 0)
            .cloned()
            .unwrap_or_default();
        body.push_str(&format!("{:02X}", idx));
        body.push_str(&encoder::le16(item.id));
        body.push_str(&format!("{:02X}", item.count));
        body.push_str(&format!("{:02X}", item.long_val));
        body.push_str(&encoder::le32(listing.price));
        body.push_str(&format!(
            "{:02X}",
            crate::server::inventory::biased_giatri_long(item.giatri_long)
        ));
        body.push_str(&format!("{:02X}", item.khang));
        body.push_str("00");
        body.push_str(&encoder::le32(item.texp));
        idx = idx.wrapping_add(1);
    }
    crate::protocol::frame("1721", &body)
}

/// Player-shop purchase errors (failure paths of op 0x17 sub 33).
#[derive(Debug, PartialEq, Eq)]
pub enum ShopBuyError {
    ShopClosed,
    InvalidIndex,
    NotEnoughStock,
    NotEnoughGold { total: u32 },
    NoFreeSlot,
    SellerGoldFull,
}

/// Outcome of a completed player-shop purchase.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShopBuyResult {
    pub total: u32,
    pub count: u8,
    pub equip: bool,
    pub item: InventoryItem,
}

/// Perform a player-shop purchase between two in-memory sessions.
/// Validates before mutating so a failed buy leaves both sides untouched.
/// `shop_index` is 1-based (1..10) matching the client UI catalog echo.
/// Seller gold is capped at 9,999,999.
pub fn complete_shop_buy(
    buyer: &mut crate::server::session::Session,
    seller: &mut crate::server::session::Session,
    shop_index: usize,
    count: u8,
) -> Result<ShopBuyResult, ShopBuyError> {
    if !seller.shop.active || seller.shop.items.is_empty() {
        return Err(ShopBuyError::ShopClosed);
    }
    let actual_idx = shop_index
        .checked_sub(1)
        .ok_or(ShopBuyError::InvalidIndex)?;
    let listing = seller
        .shop
        .items
        .get(actual_idx)
        .cloned()
        .ok_or(ShopBuyError::InvalidIndex)?;
    if count == 0 {
        return Err(ShopBuyError::InvalidIndex);
    }
    let seller_item = seller
        .homdo
        .iter()
        .find(|i| i.slot == listing.slot && i.id > 0)
        .cloned()
        .ok_or(ShopBuyError::NotEnoughStock)?;
    if listing.item_id != 0 && listing.item_id != seller_item.id {
        return Err(ShopBuyError::NotEnoughStock);
    }
    if seller_item.count < count || (listing.count > 0 && listing.count < count) {
        return Err(ShopBuyError::NotEnoughStock);
    }

    let total = u64::from(listing.price)
        .saturating_mul(u64::from(count))
        .min(u32::MAX as u64) as u32;
    if buyer.gold < total {
        return Err(ShopBuyError::NotEnoughGold { total });
    }

    let seller_gold_total = u64::from(seller.gold).saturating_add(u64::from(total));
    if seller_gold_total > u64::from(GOLD_CAP) {
        return Err(ShopBuyError::SellerGoldFull);
    }

    let equip = (1..=6).contains(&seller_item.loai);
    let mut purchase_item = seller_item.clone();
    purchase_item.count = count;
    if (equip && crate::server::inventory::free_slot(&buyer.homdo).is_none())
        || (!equip && !crate::server::inventory::can_add_item(&buyer.homdo, &purchase_item))
    {
        return Err(ShopBuyError::NoFreeSlot);
    }

    // Apply the swap.
    buyer.gold -= total;
    seller.gold = (u64::from(seller.gold).saturating_add(u64::from(total))).min(u64::from(GOLD_CAP)) as u32;

    let seller_pos = seller
        .homdo
        .iter()
        .position(|item| item.slot == listing.slot && item.id == seller_item.id)
        .ok_or(ShopBuyError::NotEnoughStock)?;
    seller.homdo[seller_pos].count -= count;
    let moved = count;
    if seller.homdo[seller_pos].count == 0 {
        seller.homdo.remove(seller_pos);
    }

    if equip {
        let slot = crate::server::inventory::free_slot(&buyer.homdo).unwrap();
        let mut copy = seller_item.clone();
        copy.slot = slot;
        copy.count = moved;
        buyer.homdo.push(copy);
    } else {
        let mut copy = seller_item.clone();
        copy.count = moved;
        let _ = crate::server::inventory::add_item(&mut buyer.homdo, copy);
    }

    // Drop the listing when the seller's slot is sold out or listing count is exhausted.
    let still_has = seller
        .homdo
        .iter()
        .any(|i| i.slot == listing.slot && i.id > 0 && i.count > 0);
    let new_listing_count = seller.shop.items[actual_idx].count.saturating_sub(count);
    if !still_has || new_listing_count == 0 {
        seller.shop.items.remove(actual_idx);
    } else {
        seller.shop.items[actual_idx].count = new_listing_count;
    }

    Ok(ShopBuyResult {
        total,
        count: moved,
        equip,
        item: purchase_item,
    })
}

/// Handle Opcode 0x17 subs 30..33 — Player shop.
/// Sub 30/31 also broadcast the open/close frames to the map via the hub;
/// sub 32 records `open_shop_id` and shows the seller's 1721 catalog;
/// sub 33 completes the purchase through the online-session registry.
pub async fn handle_player_shop(ctx: &mut OpcodeCtx<'_>) {
    let payload = ctx.payload;
    let sub = ctx.sub;
    let conn = &mut ctx.conn;
    let out = &mut ctx.out;
    let hub = ctx.env.hub;
    let pool = ctx.env.pool;
    let id = conn.session.id;

    match sub {
        // Sub 30: Open player shop — parse 100 bytes items first, then name + card.
        crate::protocol::ITEM_SUB_STALL_OPEN => {
            if payload.len() < 101 {
                out.send("F4440300171E02");
                return;
            }
            let name_len = payload[100] as usize;
            if name_len == 0 || payload.len() < 101 + name_len {
                out.send("F4440300171E02");
                return;
            }
            let name_bytes = payload[101..101 + name_len].to_vec();
            let image = if payload.len() > 101 + name_len {
                payload[101 + name_len]
            } else {
                0
            };

            let mut listings = Vec::new();
            let mut seen_slots = std::collections::HashSet::new();
            for i in 0..10 {
                let offset = i * 10;
                let slot = payload[offset];
                let qty = payload[offset + 1];
                let price = encoder::u32_le(
                    payload[offset + 2],
                    payload[offset + 3],
                    payload[offset + 4],
                    payload[offset + 5],
                );
                if slot == 0 || qty == 0 {
                    continue;
                }
                if !(1..=25).contains(&slot) || price == 0 || price > GOLD_CAP || !seen_slots.insert(slot) {
                    out.send("F4440300171E02");
                    return;
                }
                let Some(item) = conn
                    .session
                    .homdo
                    .iter()
                    .find(|item| item.slot == slot && item.id > 0)
                else {
                    out.send("F4440300171E02");
                    return;
                };
                if qty > item.count {
                    out.send("F4440300171E02");
                    return;
                }
                listings.push(crate::server::session::ShopItem {
                    slot,
                    item_id: item.id,
                    count: qty,
                    price,
                });
            }

            if listings.is_empty() {
                out.send("F4440300171E02");
                return;
            }

            conn.session.shop.active = true;
            conn.session.shop.image = image;
            conn.session.shop.name = name_bytes.clone();
            conn.session.shop.items = listings;

            // Self confirmation: exact 3 bytes (F4 44 03 00 17 1E 01)
            out.send("F4440300171E01");
            let bcast = crate::protocol::frame(
                "171F",
                &format!(
                    "{}{:02X}{}{:02X}",
                    encoder::le32(id),
                    name_len,
                    encoder::strhex(&name_bytes),
                    image
                ),
            );
            out.broadcast(id, bcast);
        }
        // Sub 31: Close player shop — self + broadcast 1720.
        crate::protocol::ITEM_SUB_STALL_CLOSE => {
            conn.session.shop.active = false;
            conn.session.shop.name.clear();
            conn.session.shop.items.clear();
            let close = format!("F44407001720{}00", encoder::le32(id));
            out.send(&close);
            out.broadcast(id, close);
        }
        // Sub 32: View another player's shop catalog.
        crate::protocol::ITEM_SUB_STALL_VIEW => {
            if payload.len() >= 4 {
                let target = encoder::u32_le(payload[0], payload[1], payload[2], payload[3]);
                let seller = crate::server::session::online_sessions()
                    .lock()
                    .unwrap()
                    .get(&target)
                    .cloned();
                match seller {
                    Some(s) if target != id && s.shop.active && !s.shop.items.is_empty() => {
                        conn.session.open_shop_id = target;
                        out.send(player_shop_catalog_frame(&s));
                    }
                    None => out.send(red_message("Người bán đang offline")),
                    _ => out.send(red_message("Cửa hàng đã đóng cửa")),
                }
            }
        }
        // Sub 33: Buy from the shop opened by sub 32.
        crate::protocol::ITEM_SUB_STALL_BUY => {
            if payload.len() < 6 {
                return;
            }
            let seller_id_wire = encoder::u32_le(payload[0], payload[1], payload[2], payload[3]);
            let seller_id = if conn.session.open_shop_id != 0 {
                conn.session.open_shop_id
            } else {
                seller_id_wire
            };
            if seller_id == 0 || seller_id == id {
                out.send(red_message("Không thể mua từ cửa hàng này"));
                return;
            }
            let shop_index = payload[4] as usize;
            let count = payload[5];

            // Two-Phase State Lock: lock buyer and seller in deterministic order
            let _locks = crate::server::session::lock_player_operations([id, seller_id]).await;

            let seller = crate::server::session::online_sessions()
                .lock()
                .unwrap()
                .get(&seller_id)
                .cloned();
            let Some(mut seller_session) = seller else {
                out.send(red_message("Người bán đang offline"));
                return;
            };

            let buyer_before = conn.session.clone();
            match complete_shop_buy(&mut conn.session, &mut seller_session, shop_index, count) {
                Ok(res) => {
                    if !persist::persist_shop_transaction(
                        pool,
                        conn.session.id,
                        conn.session.gold,
                        &conn.session.homdo,
                        Some(seller_id),
                        Some(seller_session.gold),
                        Some(&seller_session.homdo),
                    )
                    .await
                    {
                        conn.session = buyer_before;
                        out.send(red_message("Giao dịch không thành công"));
                        return;
                    }
                    crate::server::session::online_sessions()
                        .lock()
                        .unwrap()
                        .insert(seller_id, seller_session.clone());
                    out.send(gold_frame(conn.session.gold));
                    // Refresh the buyer's view of the seller's catalog.
                    out.send(player_shop_catalog_frame(&seller_session));
                    out.send(crate::server::inventory::item_added_frame(&res.item));
                    if let Some(hub) = hub {
                        // Notify the seller of the gold received.
                        let seller_gold = gold_frame(seller_session.gold);
                        hub.send_to(seller_id, &seller_gold).await;
                        // Gói gỡ/trừ đồ trên quầy
                        hub.send_to(seller_id, &format!("F44404001709{:02X}{:02X}", res.item.slot, res.count)).await;
                        if seller_session.shop.items.is_empty() {
                            seller_session.shop.active = false;
                            crate::server::session::online_sessions()
                                .lock()
                                .unwrap()
                                .insert(seller_id, seller_session.clone());
                            let close = format!("F44407001720{}00", encoder::le32(seller_id));
                            hub.send_to(seller_id, &close).await;
                            hub.broadcast_except(seller_id, &close).await;
                        }
                    }
                }
                Err(ShopBuyError::NotEnoughGold { total }) => {
                    out.send(red_message(&format!("Không đủ vàng để mua, cần {}", total)));
                }
                Err(ShopBuyError::SellerGoldFull) => {
                    out.send(red_message("Vàng của người bán đã đầy"));
                }
                Err(ShopBuyError::NotEnoughStock) => {
                    out.send(red_message("Người bán không đủ hàng"));
                }
                Err(ShopBuyError::ShopClosed) => {
                    out.send(red_message("Cửa hàng đã đóng cửa"));
                }
                Err(_) => out.send(red_message("Không thể mua")),
            }
        }
        _ => {}
    }
}

