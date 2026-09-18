//! NPC shop buy/sell (Opcode 0x1B) & Player shop (Opcode 0x17 subs 30–33) handlers.

use crate::db::persist;
use crate::protocol::encoder;
use crate::server::dispatcher::OpcodeCtx;
use crate::server::session::{Conn, InventoryItem};

/// `F4440A001A04` + gold + `00000000` — the gold-update frame sent after
/// every successful shop transaction.
fn gold_frame(gold: u32) -> String {
    format!("F4440A001A04{}00000000", encoder::le32(gold))
}

/// VISCII-encoded red message frame (`F444 + len + 020B + 00000000 + msg`).
/// Proper-Unicode Vietnamese text is VISCII-mapped (§4.4 item 3) so ư/ờ/đ
/// survive as single-byte VISCII instead of collapsing to `'?'`.
fn red_message(msg: &str) -> String {
    let visc = crate::encoding::viscii_encode(msg);
    let body = format!("00000000{}", encoder::strhex(&visc));
    crate::protocol::frame("020B", &body)
}

/// Sell the first item in `item_range` present in inventory: remove up to
/// `count` of it and credit that many gold. Returns true when something was
/// sold.
fn try_sell_range(conn: &mut Conn, item_range: std::ops::RangeInclusive<u16>, count: u8) -> bool {
    if count == 0 {
        return false;
    }
    for item_id in item_range {
        if conn
            .session
            .homdo
            .iter()
            .filter(|i| i.id == item_id)
            .map(|i| u32::from(i.count))
            .sum::<u32>()
            >= u32::from(count)
        {
            let removed = conn.session.remove_homdo_item(item_id, u32::from(count));
            conn.session.gold = conn.session.gold.saturating_add(u32::from(count));
            return removed == u32::from(count);
        }
    }
    false
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

/// Handle Opcode 0x1B — NPC shop buy/sell.
/// The branch is selected by `idtalking` (sell NPCs scan an item-id range; buy
/// NPCs use the transcribed `(map, menu)` price table; `(7, 9999)` grants the
/// free starter bundle).
pub async fn handle_npc_shop(ctx: &mut OpcodeCtx<'_>) {
    let conn = &mut ctx.conn;
    let out = &mut ctx.out;
    let pool = ctx.env.pool;
    let data = ctx.data;
    // payload[0] = menu, payload[1] = sell count.
    let menu = ctx.payload.first().copied().unwrap_or(0);
    let count = ctx.payload.get(1).copied().unwrap_or(0);
    let idtalking = conn.session.idtalking;
    let idnpctalking = conn.session.idnpctalking;
    let map_id = conn.session.map_id;

    let before_sell = conn.session.clone();
    let sold = if map_id > 10000 && (idnpctalking == 16005 || idnpctalking == 99999) {
        try_sell_range(conn, 26001..=26455, count)
    } else if map_id > 10000 && (idnpctalking == 16002 || idnpctalking == 99999) {
        try_sell_range(conn, 27001..=27165, count)
    } else {
        false
    };
    if sold {
        let persisted = persist::persist_shop_transaction(
            pool,
            conn.session.id,
            conn.session.gold,
            &conn.session.homdo,
            None,
            None,
            None,
        )
        .await;
        if !persisted {
            conn.session = before_sell;
            tracing::warn!("NPC sell persistence failed for player {}", conn.session.id);
        } else {
            out.send(red_message("Khách quan bán hàng thành công"));
            out.send(gold_frame(conn.session.gold));
        }
        return;
    }

    // Free starter bundle (idtalking == 7 && map == 9999).
    if idtalking == 7 && map_id == 9999 {
        let before = conn.session.clone();
        for (item_id, n) in [(18001u16, 1u8), (27156, 50), (52015, 50)] {
            let item = crate::server::inventory::from_template(data, item_id, n);
            if !crate::server::inventory::can_add_item(&conn.session.homdo, &item)
                || conn.session.add_homdo_item(item).is_empty()
            {
                conn.session = before;
                return;
            }
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
            return;
        }
        out.send(red_message("Khách quan mua hàng thành công"));
        return;
    }

    // Buy from the transcribed (map, menu) shelf.
    if let Some((item_id, price)) = get_npc_shop_price(idtalking, map_id, menu) {
        if conn.session.gold >= price {
            let item = crate::server::inventory::from_template(data, item_id, 1);
            if !crate::server::inventory::can_add_item(&conn.session.homdo, &item) {
                return;
            }
            let before = conn.session.clone();
            conn.session.gold -= price;
            if conn.session.add_homdo_item(item).is_empty()
                || !persist::persist_shop_transaction(
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
                return;
            }
            out.send(red_message("Khách quan mua hàng thành công"));
            out.send(gold_frame(conn.session.gold));
        }
    }
}

/// Player shop catalog frame `1721`:
/// 17 zero bytes + per listed item `id2 count2 price4 long giatri khang texp4 idx`.
pub fn player_shop_catalog_frame(seller: &crate::server::session::Session) -> String {
    let mut body = "0".repeat(34);
    let mut idx = 1u8;
    for listing in &seller.shop.items {
        let item = seller
            .homdo
            .iter()
            .find(|i| i.slot == listing.slot && i.id > 0)
            .cloned()
            .unwrap_or_default();
        body.push_str(&encoder::le16(item.id));
        body.push_str(&encoder::le16(u16::from(item.count)));
        body.push_str(&encoder::le32(listing.price));
        body.push_str(&format!("{:02X}", item.long_val));
        body.push_str(&format!(
            "{:02X}",
            (100u16.wrapping_add(u16::from(item.giatri_long))) & 0xFF
        ));
        body.push_str(&format!("{:02X}", item.khang));
        body.push_str(&encoder::le32(item.texp));
        body.push_str(&format!("{:02X}", idx));
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
}

/// Outcome of a completed player-shop purchase.
#[derive(Debug)]
pub struct ShopBuyResult {
    pub total: u32,
    pub count: u8,
    pub equip: bool,
    pub item: InventoryItem,
}

/// Perform a player-shop purchase between two in-memory sessions.
/// Validates before mutating so a failed buy leaves both sides untouched.
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
    let listing = seller
        .shop
        .items
        .get(shop_index)
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
    seller.gold = (u64::from(seller.gold).saturating_add(u64::from(total))).min(9_999_999) as u32;

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

    // Drop the listing when the seller's slot is sold out.
    let still_has = seller
        .homdo
        .iter()
        .any(|i| i.slot == listing.slot && i.id > 0 && i.count > 0);
    if !still_has {
        seller.shop.items.remove(shop_index);
    }

    Ok(ShopBuyResult {
        total,
        count: moved,
        equip,
        item: seller_item,
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
        // Sub 30: Open player shop — parse name + (slot, price) listings.
        30 => {
            let Some(name_len) = payload.first().copied().map(usize::from) else {
                return;
            };
            let listings_start = name_len + 2;
            if name_len == 0
                || listings_start > payload.len()
                || !(payload.len() - listings_start).is_multiple_of(5)
            {
                return;
            }
            let name_bytes = payload[1..1 + name_len].to_vec();
            let image = payload[name_len + 1];
            let mut listings = Vec::new();
            let mut seen_slots = std::collections::HashSet::new();
            let mut items_hex = String::new();
            let mut cursor = listings_start;
            while cursor + 5 <= payload.len() {
                let slot = payload[cursor];
                let price = encoder::u32_le(
                    payload[cursor + 1],
                    payload[cursor + 2],
                    payload[cursor + 3],
                    payload[cursor + 4],
                );
                let Some(item) = conn
                    .session
                    .homdo
                    .iter()
                    .find(|item| item.slot == slot && item.id > 0)
                else {
                    return;
                };
                if price == 0 || !seen_slots.insert(slot) {
                    return;
                }
                listings.push(crate::server::session::ShopItem {
                    slot,
                    item_id: item.id,
                    count: item.count,
                    price,
                });
                items_hex.push_str(&format!("{:02X}", slot));
                items_hex.push_str(&encoder::le32(price));
                cursor += 5;
            }
            conn.session.shop.active = true;
            conn.session.shop.image = image;
            conn.session.shop.name = name_bytes.clone();
            conn.session.shop.items = listings;

            // Self catalog (171E) + broadcast open (171F) to other map clients.
            let body = format!(
                "{:02X}{}{:02X}{}",
                name_len,
                encoder::strhex(&name_bytes),
                image,
                items_hex
            );
            out.send(crate::protocol::frame("171E", &body));
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
        31 => {
            conn.session.shop.active = false;
            conn.session.shop.name.clear();
            conn.session.shop.items.clear();
            let close = format!("F44406001720{}", encoder::le32(id));
            out.send(&close);
            out.broadcast(id, close);
        }
        // Sub 32: View another player's shop catalog.
        32 => {
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
        33 => {
            if payload.len() < 6 {
                return;
            }
            let seller_id = conn.session.open_shop_id;
            if seller_id == 0 || seller_id == id {
                out.send(red_message("Không thể mua từ cửa hàng này"));
                return;
            }
            let shop_index = payload[4] as usize;
            let count = payload[5];

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
                    if res.equip {
                        out.send(equip_item_frame(&res.item));
                    }
                    if let Some(hub) = hub {
                        // Notify the seller of the gold received.
                        let seller_gold = gold_frame(seller_session.gold);
                        hub.send_to(seller_id, &seller_gold).await;
                    }
                }
                Err(ShopBuyError::NotEnoughGold { total }) => {
                    out.send(red_message(&format!("Không đủ vàng để mua, cần {}", total)));
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

/// `1706` frame carrying a freshly purchased equipable item to the buyer
/// (`id2 count2 0000 long giatri khang texp4`).
fn equip_item_frame(item: &InventoryItem) -> String {
    let body = format!(
        "{}{:02X}0000{:02X}{:02X}{:02X}{}",
        encoder::le16(item.id),
        item.count,
        item.long_val,
        (100u16.wrapping_add(u16::from(item.giatri_long))) & 0xFF,
        item.khang,
        encoder::le32(item.texp)
    );
    crate::protocol::frame("1706", &body)
}
