//! NPC shop buy/sell (Opcode 0x1B) & Player shop (Opcode 0x17 subs 30–33) handlers.

use crate::db::persist;
use crate::protocol::encoder;
use crate::server::handler::{HandleOutcome, OpcodeCtx};
use crate::server::session::{Conn, InventoryItem};

/// `F4440A001A04` + gold + `00000000` — the gold-update frame C# sends after
/// every successful shop transaction (Client.cs buy/sell branches).
fn gold_frame(gold: u32) -> String {
    format!("F4440A001A04{}00000000", encoder::le32(gold))
}

/// VISCII-encoded red message frame (`F444 + len + 020B + 00000000 + msg`),
/// mirroring C# `SendRedMessage` (Client.cs:9854). Proper-Unicode Vietnamese
/// text is mapped through `smethod_17` (§4.4 item 3) so ư/ờ/đ survive as
/// single-byte VISCII instead of collapsing to `'?'`.
fn red_message(msg: &str) -> String {
    let visc = crate::encoding::viscii_encode(msg);
    let body = format!("00000000{}", encoder::strhex(&visc));
    crate::protocol::frame("020B", &body)
}

/// Sell the first item in `item_range` present in inventory: remove up to
/// `count` of it and credit that many gold (C# `Update_H1B` megi bán branches,
/// Client.cs:7095–7122). Returns true when something was sold.
fn try_sell_range(
    conn: &mut Conn,
    out: &mut HandleOutcome,
    item_range: std::ops::RangeInclusive<u16>,
    count: u8,
) -> bool {
    if count == 0 {
        return false;
    }
    for item_id in item_range {
        if conn
            .session
            .homdo
            .iter()
            .any(|i| i.id == item_id && i.count > 0)
        {
            let removed = conn.session.remove_homdo_item(item_id, u32::from(count));
            conn.session.gold = conn.session.gold.saturating_add(removed);
            out.send(gold_frame(conn.session.gold));
            out.send(red_message("Khách quan bán hàng thành công"));
            return true;
        }
    }
    false
}

/// Lookup NPC shop entry by (idtalking, map_id, menu) → (item_id, price).
/// Source: C# `Update_H1B` (Client.cs:6472–7130), transcribed verbatim.
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
/// Mirrors C# `Update_H1B` (Client.cs:6428–7130): the branch is selected by
/// `idtalking` (sell NPCs scan an item-id range; buy NPCs use the transcribed
/// `(map, menu)` price table; `(7, 9999)` grants the free starter bundle).
pub async fn handle_npc_shop(ctx: &mut OpcodeCtx<'_>) {
    let conn = &mut ctx.conn;
    let out = &mut ctx.out;
    let pool = ctx.env.pool;
    // C# `num2 = packet[6]` (menu), `num3 = packet[7]` (sell count).
    let menu = ctx.payload.first().copied().unwrap_or(0);
    let count = ctx.payload.get(1).copied().unwrap_or(0);
    let idtalking = conn.session.idtalking;
    let idnpctalking = conn.session.idnpctalking;
    let map_id = conn.session.map_id;

    let sold = if map_id > 10000 && (idnpctalking == 16005 || idnpctalking == 99999) {
        try_sell_range(conn, out, 26001..=26455, count)
    } else if map_id > 10000 && (idnpctalking == 16002 || idnpctalking == 99999) {
        try_sell_range(conn, out, 27001..=27165, count)
    } else {
        false
    };
    if sold {
        persist::update_player(pool, conn.session.id, "Gold", conn.session.gold as i64).await;
        return;
    }

    // Free starter bundle (C# `num4 == 7 && my_MapId == 9999`).
    if idtalking == 7 && map_id == 9999 {
        out.send(red_message("Khách quan mua hàng thành công"));
        for (item_id, n) in [(18001u16, 1u8), (27156, 50), (52015, 50)] {
            let item = InventoryItem {
                slot: 0,
                id: item_id,
                count: n,
                ..Default::default()
            };
            let _ = conn.session.add_homdo_item(item);
        }
        return;
    }

    // Buy from the transcribed (map, menu) shelf.
    if let Some((item_id, price)) = get_npc_shop_price(idtalking, map_id, menu) {
        if conn.session.gold >= price {
            conn.session.gold -= price;
            let item = InventoryItem {
                slot: 0,
                id: item_id,
                count: 1,
                ..Default::default()
            };
            let _ = conn.session.add_homdo_item(item);
            persist::update_player(pool, conn.session.id, "Gold", conn.session.gold as i64).await;
            out.send(red_message("Khách quan mua hàng thành công"));
            out.send(gold_frame(conn.session.gold));
        }
    }
}

/// Player shop catalog frame `1721` (C# `OpenPlayerShop`, Client.cs:10147):
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

/// Player-shop purchase errors (C# case 33 failure paths).
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

/// Perform a player-shop purchase between two in-memory sessions (C# case 33,
/// Client.cs:5466–5545). Validates before mutating so a failed buy leaves both
/// sides untouched (fixing the C# order bug where items moved before the gold
/// check). Seller gold is capped at 9,999,999 like C#.
pub fn complete_shop_buy(
    buyer: &mut crate::server::session::Session,
    seller: &mut crate::server::session::Session,
    shop_index: usize,
    count: u8,
) -> Result<ShopBuyResult, ShopBuyError> {
    if seller.shop.items.is_empty() {
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
    if seller_item.count < count {
        return Err(ShopBuyError::NotEnoughStock);
    }

    let total = u64::from(listing.price)
        .saturating_mul(u64::from(count))
        .min(u32::MAX as u64) as u32;
    if buyer.gold < total {
        return Err(ShopBuyError::NotEnoughGold { total });
    }

    let equip = (1..=6).contains(&seller_item.loai);
    if equip && crate::server::inventory::free_slot(&buyer.homdo).is_none() {
        return Err(ShopBuyError::NoFreeSlot);
    }

    // Apply the swap.
    buyer.gold -= total;
    seller.gold = (u64::from(seller.gold).saturating_add(u64::from(total))).min(9_999_999) as u32;

    let removed =
        crate::server::inventory::remove_item(&mut seller.homdo, seller_item.id, u32::from(count));
    let moved = removed.min(u32::from(count)) as u8;

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

    // Drop the listing when the seller's slot is sold out (C# RemoveAt(index)).
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

/// Handle Opcode 0x17 subs 30..33 — Player shop (C# `Update_H17` cases 30–33,
/// Client.cs:5399–5545). Sub 30/31 also broadcast the open/close frames to the
/// map via the hub; sub 32 records `_Open_Shop_Id` and shows the seller's 1721
/// catalog; sub 33 completes the purchase through the online-session registry.
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
            let name_len = payload.first().copied().unwrap_or(0) as usize;
            let name_bytes = payload.get(1..1 + name_len).unwrap_or(&[]).to_vec();
            let name = String::from_utf8_lossy(&name_bytes).to_string();

            conn.session.shop.active = true;
            conn.session.shop.name = name.clone();
            conn.session.shop.items.clear();

            let mut items_hex = String::new();
            let mut cursor = 2 + name_len; // payload[name_len + 2]
            while cursor + 5 <= payload.len() {
                let slot = payload[cursor];
                let price = encoder::u32_le(
                    payload[cursor + 1],
                    payload[cursor + 2],
                    payload[cursor + 3],
                    payload[cursor + 4],
                );
                conn.session
                    .shop
                    .items
                    .push(crate::server::session::ShopItem {
                        slot,
                        price,
                        ..Default::default()
                    });
                items_hex.push_str(&format!("{:02X}", slot));
                items_hex.push_str(&encoder::le32(price));
                cursor += 5;
            }

            // Self catalog (171E) + broadcast open (171F) to other map clients.
            let body = format!(
                "{:02X}{}{}",
                name_len,
                encoder::strhex(&name_bytes),
                items_hex
            );
            out.send(crate::protocol::frame("171E", &body));
            if let Some(hub) = hub {
                let bcast = crate::protocol::frame(
                    "171F",
                    &format!(
                        "{}{:02X}{}",
                        encoder::le32(id),
                        name_len,
                        encoder::strhex(&name_bytes)
                    ),
                );
                hub.broadcast_except(id, &bcast).await;
            }
        }
        // Sub 31: Close player shop — self + broadcast 1720.
        31 => {
            conn.session.shop.active = false;
            conn.session.shop.name.clear();
            conn.session.shop.items.clear();
            let close = format!("F44406001720{}", encoder::le32(id));
            out.send(&close);
            if let Some(hub) = hub {
                hub.broadcast_except(id, &close).await;
            }
        }
        // Sub 32: View another player's shop catalog.
        32 => {
            if payload.len() >= 4 {
                let target = encoder::u32_le(payload[0], payload[1], payload[2], payload[3]);
                conn.session.open_shop_id = target;
                let seller = crate::server::session::online_sessions()
                    .lock()
                    .unwrap()
                    .get(&target)
                    .cloned();
                match seller {
                    Some(s) => out.send(player_shop_catalog_frame(&s)),
                    None => out.send(red_message("Người bán đang offline")),
                }
            }
        }
        // Sub 33: Buy from the shop opened by sub 32.
        33 => {
            if payload.len() < 6 {
                return;
            }
            let seller_id = conn.session.open_shop_id;
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

            match complete_shop_buy(&mut conn.session, &mut seller_session, shop_index, count) {
                Ok(res) => {
                    crate::server::session::online_sessions()
                        .lock()
                        .unwrap()
                        .insert(seller_id, seller_session.clone());
                    persist::update_player(pool, conn.session.id, "Gold", conn.session.gold as i64)
                        .await;
                    persist::update_player(pool, seller_id, "Gold", seller_session.gold as i64)
                        .await;

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
/// (C# case 33: `id2 count2 0000 long giatri khang texp4`).
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::battle::service::BattleService;
    use crate::data::loader::GameData;
    use crate::server::handler::{test_ctx, HandleOutcome};
    use crate::server::session::Conn;
    use std::sync::Arc;

    /// Player shop catalog frame `1721` (C# `OpenPlayerShop`, Client.cs:10147):
    /// 17 zero bytes + per listed item (id2 count2 price4 long giatri khang texp4 idx).
    #[test]
    fn player_shop_catalog_frame_matches_csharp() {
        use crate::server::session::Session;
        let mut seller = Session::new();
        seller.id = 300002;
        seller.homdo.push(InventoryItem {
            slot: 1,
            id: 20023,
            count: 5,
            doben: 100,
            long_val: 100,
            giatri_long: 50,
            khang: 30,
            texp: 1234,
            loai: 1,
            ..Default::default()
        });
        seller.shop.items.push(crate::server::session::ShopItem {
            slot: 1,
            price: 58800,
            ..Default::default()
        });

        let frame = player_shop_catalog_frame(&seller);
        assert_eq!(
            frame,
            "F44423001721".to_string()
                + "0000000000000000000000000000000000"
                + "374E0500B0E5000064961ED204000001"
        );
    }

    /// Complete a player-shop purchase: item + gold swap both sides (C# case 33).
    #[test]
    fn complete_shop_buy_transfers_item_and_gold() {
        use crate::server::session::Session;
        let mut buyer = Session::new();
        buyer.id = 300001;
        buyer.gold = 100000;
        let mut seller = Session::new();
        seller.id = 300002;
        seller.gold = 1000;
        seller.homdo.push(InventoryItem {
            slot: 3,
            id: 20023,
            count: 2,
            ..Default::default()
        });
        seller.shop.items.push(crate::server::session::ShopItem {
            slot: 3,
            item_id: 20023,
            price: 58800,
            ..Default::default()
        });

        let res = complete_shop_buy(&mut buyer, &mut seller, 0, 1).unwrap();
        assert_eq!(res.total, 58800);
        assert_eq!(buyer.gold, 41200);
        assert!(buyer.homdo.iter().any(|i| i.id == 20023 && i.count == 1));
        assert_eq!(seller.gold, 59800);
        assert!(seller.homdo.iter().any(|i| i.id == 20023 && i.count == 1));
        // Listing survives while the seller still holds stock.
        assert_eq!(seller.shop.items.len(), 1);
    }

    /// Buying without enough gold fails before any mutation.
    #[test]
    fn complete_shop_buy_rejects_not_enough_gold() {
        use crate::server::session::Session;
        let mut buyer = Session::new();
        buyer.gold = 100;
        let mut seller = Session::new();
        seller.gold = 0;
        seller.homdo.push(InventoryItem {
            slot: 3,
            id: 20023,
            count: 2,
            ..Default::default()
        });
        seller.shop.items.push(crate::server::session::ShopItem {
            slot: 3,
            item_id: 20023,
            price: 58800,
            ..Default::default()
        });

        let err = complete_shop_buy(&mut buyer, &mut seller, 0, 1).unwrap_err();
        assert!(matches!(err, ShopBuyError::NotEnoughGold { total: 58800 }));
        assert_eq!(buyer.gold, 100);
        assert_eq!(seller.gold, 0);
        assert!(buyer.homdo.is_empty());
    }

    /// Buying more than the seller has on that slot fails.
    #[test]
    fn complete_shop_buy_rejects_not_enough_stock() {
        use crate::server::session::Session;
        let mut buyer = Session::new();
        buyer.gold = 100000;
        let mut seller = Session::new();
        seller.homdo.push(InventoryItem {
            slot: 3,
            id: 20023,
            count: 1,
            ..Default::default()
        });
        seller.shop.items.push(crate::server::session::ShopItem {
            slot: 3,
            item_id: 20023,
            price: 58800,
            ..Default::default()
        });

        let err = complete_shop_buy(&mut buyer, &mut seller, 0, 5).unwrap_err();
        assert!(matches!(err, ShopBuyError::NotEnoughStock));
        assert_eq!(buyer.gold, 100000);
        assert_eq!(seller.homdo[0].count, 1);
    }

    /// Sub 30: parse name + listed items, store shop state, emit self 171E.
    #[tokio::test]
    async fn player_shop_sub30_open_parses_and_emits_171e() {
        let mut conn = Conn::new();
        conn.session.id = 300001;
        // payload: name_len(4) "TEST" pad slot price(1000)
        let payload = [
            4u8, b'T', b'E', b'S', b'T', 0x00, 0x01, 0xE8, 0x03, 0x00, 0x00,
        ];

        let data = GameData::default();
        let service = BattleService::new(Arc::new(GameData::default()));
        let mut out = HandleOutcome::default();
        let mut ctx = test_ctx(&mut conn, &data, &service, &mut out, 30, &payload);
        handle_player_shop(&mut ctx).await;

        assert!(conn.session.shop.active);
        assert_eq!(conn.session.shop.name, "TEST");
        assert_eq!(conn.session.shop.items.len(), 1);
        assert_eq!(conn.session.shop.items[0].slot, 1);
        assert_eq!(conn.session.shop.items[0].price, 1000);
        assert!(out
            .outgoing
            .iter()
            .any(|f| f.starts_with("F4440C00171E") && f.contains("045445535401E8030000")));
    }

    /// Sub 31: close clears the shop and emits 1720 + player id.
    #[tokio::test]
    async fn player_shop_sub31_close_emits_1720() {
        let mut conn = Conn::new();
        conn.session.id = 300001;
        conn.session.shop.active = true;
        conn.session.shop.name = "TEST".to_string();

        let data = GameData::default();
        let service = BattleService::new(Arc::new(GameData::default()));
        let mut out = HandleOutcome::default();
        let mut ctx = test_ctx(&mut conn, &data, &service, &mut out, 31, &[]);
        handle_player_shop(&mut ctx).await;

        assert!(!conn.session.shop.active);
        assert!(conn.session.shop.items.is_empty());
        let close = format!("F44406001720{}", encoder::le32(300001));
        assert!(out.outgoing.iter().any(|f| f == &close));
    }

    /// Sub 32 + 33: open a seller's shop then buy from it (C# `_Open_Shop_Id`).
    #[tokio::test]
    async fn player_shop_sub33_buys_from_registry_seller() {
        use crate::server::session::online_sessions;
        online_sessions().lock().unwrap().clear();

        let mut conn = Conn::new();
        conn.session.id = 300001;
        conn.session.gold = 100000;
        conn.session.map_id = 12001;

        let mut seller = crate::server::session::Session::new();
        seller.id = 300002;
        seller.gold = 1000;
        seller.homdo.push(InventoryItem {
            slot: 3,
            id: 20023,
            count: 2,
            ..Default::default()
        });
        seller.shop.items.push(crate::server::session::ShopItem {
            slot: 3,
            item_id: 20023,
            price: 58800,
            ..Default::default()
        });
        online_sessions().lock().unwrap().insert(300002, seller);

        let data = GameData::default();
        let service = BattleService::new(Arc::new(GameData::default()));

        // Sub 32: open the seller's shop → records _Open_Shop_Id + sends 1721.
        let mut out32 = HandleOutcome::default();
        let open_payload = [0xE2u8, 0x93, 0x04, 0x00];
        let mut ctx = test_ctx(&mut conn, &data, &service, &mut out32, 32, &open_payload);
        handle_player_shop(&mut ctx).await;
        assert_eq!(conn.session.open_shop_id, 300002);
        assert!(out32.outgoing.iter().any(|f| f.contains("1721")));

        // Sub 33: buy index 0, count 1.
        let mut out = HandleOutcome::default();
        let buy_payload = [0u8, 0, 0, 0, 0, 1];
        let mut ctx = test_ctx(&mut conn, &data, &service, &mut out, 33, &buy_payload);
        handle_player_shop(&mut ctx).await;

        assert_eq!(conn.session.gold, 41200);
        assert!(conn.session.homdo.iter().any(|i| i.id == 20023));
        let seller_now = online_sessions()
            .lock()
            .unwrap()
            .get(&300002)
            .unwrap()
            .clone();
        assert_eq!(seller_now.gold, 59800);
        assert!(seller_now
            .homdo
            .iter()
            .any(|i| i.id == 20023 && i.count == 1));
        let gold_frame = format!("F4440A001A04{}00000000", encoder::le32(41200));
        assert!(out.outgoing.iter().any(|f| f == &gold_frame));
        online_sessions().lock().unwrap().clear();
    }

    /// Rows transcribed verbatim from C# `Update_H1B` (Client.cs:6472–7130).
    #[tokio::test]
    async fn npc_price_table_transcribed_rows() {
        assert_eq!(get_npc_shop_price(1, 12201, 0), Some((10001, 10)));
        assert_eq!(get_npc_shop_price(1, 12201, 2), Some((10013, 20)));
        assert_eq!(get_npc_shop_price(1, 12223, 0), Some((26041, 5)));
        assert_eq!(get_npc_shop_price(1, 12223, 3), Some((27032, 15)));
        assert_eq!(get_npc_shop_price(1, 12204, 2), Some((15001, 10)));
        assert_eq!(get_npc_shop_price(2, 12204, 6), Some((22001, 10)));
        assert_eq!(get_npc_shop_price(2, 20001, 3), Some((46103, 100)));
        assert_eq!(get_npc_shop_price(3, 12244, 2), Some((26005, 15)));
        assert_eq!(get_npc_shop_price(4, 12002, 0), Some((26016, 5)));
        assert_eq!(get_npc_shop_price(7, 12001, 1), Some((27156, 115)));
        assert_eq!(get_npc_shop_price(7, 12001, 2), Some((52015, 1)));
        assert_eq!(get_npc_shop_price(8, 12990, 4), Some((20401, 10)));
        assert_eq!(get_npc_shop_price(15, 12002, 12), Some((19756, 19900)));
        assert_eq!(get_npc_shop_price(16, 12002, 0), Some((20023, 58800)));
        assert_eq!(get_npc_shop_price(16, 12002, 14), Some((19423, 58800)));
        assert_eq!(get_npc_shop_price(26, 11011, 7), Some((22412, 20)));
        assert_eq!(get_npc_shop_price(1, 19241, 4), Some((26028, 10)));
        // Unknown (map, menu) → not on the shelf.
        assert_eq!(get_npc_shop_price(5, 60000, 0), None);
    }

    /// Op 0x1B buy: `idtalking` 16 + map 12002 + menu 0 → item 20023 (58800).
    #[tokio::test]
    async fn npc_buy_deducts_gold_adds_item_emits_1a04() {
        let mut conn = Conn::new();
        conn.session.idtalking = 16;
        conn.session.map_id = 12002;
        conn.session.gold = 100000;

        let data = GameData::default();
        let service = BattleService::new(Arc::new(GameData::default()));
        let mut out = HandleOutcome::default();
        // payload[0] = menu, payload[1] = count (unused for buy).
        let mut ctx = test_ctx(&mut conn, &data, &service, &mut out, 1, &[0, 0]);
        handle_npc_shop(&mut ctx).await;

        assert_eq!(conn.session.gold, 41200);
        assert!(conn
            .session
            .homdo
            .iter()
            .any(|i| i.id == 20023 && i.count == 1));
        let gold_frame = format!("F4440A001A04{}00000000", encoder::le32(41200));
        assert!(out.outgoing.iter().any(|f| f == &gold_frame));
        // Red message ("Khách quan mua hàng thành công") via op 0x02 sub 0x0B.
        assert!(out.outgoing.iter().any(|f| f.contains("020B")));
    }

    /// Op 0x1B buy: not enough gold → nothing changes, no gold frame.
    #[tokio::test]
    async fn npc_buy_rejects_when_gold_short() {
        let mut conn = Conn::new();
        conn.session.idtalking = 16;
        conn.session.map_id = 12002;
        conn.session.gold = 1000;

        let data = GameData::default();
        let service = BattleService::new(Arc::new(GameData::default()));
        let mut out = HandleOutcome::default();
        let mut ctx = test_ctx(&mut conn, &data, &service, &mut out, 1, &[0, 0]);
        handle_npc_shop(&mut ctx).await;

        assert_eq!(conn.session.gold, 1000);
        assert!(conn.session.homdo.is_empty());
        assert!(!out.outgoing.iter().any(|f| f.contains("1A04")));
    }

    /// Op 0x1B buy: unknown (map, menu) is a silent no-op.
    #[tokio::test]
    async fn npc_buy_unknown_shelf_is_noop() {
        let mut conn = Conn::new();
        conn.session.idtalking = 5;
        conn.session.map_id = 60000;
        conn.session.gold = 1000;

        let data = GameData::default();
        let service = BattleService::new(Arc::new(GameData::default()));
        let mut out = HandleOutcome::default();
        let mut ctx = test_ctx(&mut conn, &data, &service, &mut out, 1, &[0, 0]);
        handle_npc_shop(&mut ctx).await;

        assert_eq!(conn.session.gold, 1000);
        assert!(conn.session.homdo.is_empty());
    }

    /// Op 0x1B sell: idnpctalking 16005 scans 26001..26455, pays `count` gold.
    #[tokio::test]
    async fn npc_sell_16005_scans_range_adds_count_gold() {
        let mut conn = Conn::new();
        conn.session.idnpctalking = 16005;
        conn.session.map_id = 12001;
        conn.session.gold = 10;
        conn.session.homdo.push(InventoryItem {
            slot: 1,
            id: 26041,
            count: 3,
            ..Default::default()
        });

        let data = GameData::default();
        let service = BattleService::new(Arc::new(GameData::default()));
        let mut out = HandleOutcome::default();
        // payload[1] = count to sell.
        let mut ctx = test_ctx(&mut conn, &data, &service, &mut out, 2, &[0, 2]);
        handle_npc_shop(&mut ctx).await;

        assert_eq!(conn.session.gold, 12);
        let item = conn.session.homdo.iter().find(|i| i.id == 26041).unwrap();
        assert_eq!(item.count, 1);
        let gold_frame = format!("F4440A001A04{}00000000", encoder::le32(12));
        assert!(out.outgoing.iter().any(|f| f == &gold_frame));
    }

    /// Op 0x1B sell: idnpctalking 16002 scans 27001..27165.
    #[tokio::test]
    async fn npc_sell_16002_scans_27000_range() {
        let mut conn = Conn::new();
        conn.session.idnpctalking = 16002;
        conn.session.map_id = 12001;
        conn.session.gold = 0;
        conn.session.homdo.push(InventoryItem {
            slot: 1,
            id: 27017,
            count: 1,
            ..Default::default()
        });

        let data = GameData::default();
        let service = BattleService::new(Arc::new(GameData::default()));
        let mut out = HandleOutcome::default();
        let mut ctx = test_ctx(&mut conn, &data, &service, &mut out, 2, &[0, 1]);
        handle_npc_shop(&mut ctx).await;

        assert_eq!(conn.session.gold, 1);
        assert!(!conn.session.homdo.iter().any(|i| i.id == 27017));
    }

    /// Op 0x1B: idtalking 7 + map 9999 grants the free starter bundle.
    #[tokio::test]
    async fn npc_buy_7_9999_free_starter_bundle() {
        let mut conn = Conn::new();
        conn.session.idtalking = 7;
        conn.session.map_id = 9999;
        conn.session.gold = 0;

        let data = GameData::default();
        let service = BattleService::new(Arc::new(GameData::default()));
        let mut out = HandleOutcome::default();
        let mut ctx = test_ctx(&mut conn, &data, &service, &mut out, 1, &[0, 0]);
        handle_npc_shop(&mut ctx).await;

        assert!(conn
            .session
            .homdo
            .iter()
            .any(|i| i.id == 18001 && i.count >= 1));
        assert!(conn
            .session
            .homdo
            .iter()
            .any(|i| i.id == 27156 && i.count >= 50));
        assert!(conn
            .session
            .homdo
            .iter()
            .any(|i| i.id == 52015 && i.count >= 50));
        assert_eq!(conn.session.gold, 0);
    }
}
