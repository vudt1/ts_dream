//! NPC shop buy/sell (Opcode 0x1B) & Player shop (Opcode 0x17 subs 30–33) handlers.

use crate::protocol::encoder;
use crate::server::handler::HandleOutcome;
use crate::server::session::{Conn, InventoryItem};


/// Lookup NPC shop item price by (map_id, menu_id, item_id).
pub fn get_npc_shop_price(_map_id: u16, item_id: u16) -> u32 {
    match item_id {
        10001 => 100,
        10002 => 150,
        12001 => 500,
        _ => 200,
    }
}

/// Handle Opcode 0x1B — NPC shop buy/sell.
pub fn handle_npc_shop(
    conn: &mut Conn,
    sub: u8,
    payload: &[u8],
    out: &mut HandleOutcome,
) {
    if payload.is_empty() {
        return;
    }

    match sub {
        // Sub 1: Buy from NPC shop
        1 => {
            if payload.len() < 3 {
                return;
            }
            let item_id = encoder::u16_le(payload[0], payload[1]);
            let count = payload[2].max(1);
            let unit_price = get_npc_shop_price(conn.session.map_id, item_id);
            let total_price = unit_price * count as u32;

            if conn.session.gold >= total_price {
                conn.session.gold -= total_price;
                let item = InventoryItem {
                    slot: 0,
                    id: item_id,
                    count,
                    doben: 100,
                    loai: 1,
                    ..Default::default()
                };
                if conn.session.add_homdo_item(item).is_some() {
                    out.send(format!(
                        "F4440A001A04{}00000000",
                        encoder::le32(conn.session.gold)
                    ));
                    out.send(conn.session.dump_homdo());
                } else {
                    // Refund if inventory full
                    conn.session.gold += total_price;
                    out.send("F44403001B0102"); // Inventory full
                }
            }
        }
        // Sub 2: Sell to NPC shop
        2 => {
            if payload.len() < 2 {
                return;
            }
            let slot = payload[0];
            let count = payload[1].max(1);

            if let Some(pos) = conn.session.homdo.iter().position(|i| i.slot == slot) {
                let item = conn.session.homdo[pos].clone();
                let sell_price = get_npc_shop_price(conn.session.map_id, item.id) / 2;
                let earned = sell_price * count.min(item.count) as u32;

                conn.session.gold += earned;

                if item.count > count {
                    conn.session.homdo[pos].count -= count;
                } else {
                    conn.session.homdo.remove(pos);
                }

                out.send(format!(
                    "F4440A001A04{}00000000",
                    encoder::le32(conn.session.gold)
                ));
                out.send(conn.session.dump_homdo());
            }
        }
        _ => {}
    }
}

/// Handle Opcode 0x17 subs 30..33 — Player shop.
pub fn handle_player_shop(
    conn: &mut Conn,
    sub: u8,
    payload: &[u8],
    out: &mut HandleOutcome,
) {
    match sub {
        // Sub 30: Open player shop
        30 => {
            conn.session.shop.active = true;
            conn.session.shop.name = "My Shop".to_string();

            let name_hex = encoder::strhex(conn.session.shop.name.as_bytes());
            let body = format!("{}{}", encoder::le32(conn.session.id), name_hex);
            let total_len = 2 + body.len() / 2;
            out.send(format!("F444{}171E{}", encoder::le16(total_len as u16), body));
        }
        // Sub 31: Close player shop
        31 => {
            conn.session.shop.active = false;
            out.send(format!("F4440600171F{}", encoder::le32(conn.session.id)));
        }
        // Sub 32: View someone's player shop catalog
        32 => {
            if payload.len() >= 4 {
                let target_id = encoder::u32_le(payload[0], payload[1], payload[2], payload[3]);
                let body = format!("{}00", encoder::le32(target_id));
                let total_len = 2 + body.len() / 2;
                out.send(format!("F444{}1720{}", encoder::le16(total_len as u16), body));
            }
        }
        // Sub 33: Buy from player shop
        33 => {
            if payload.len() >= 6 {
                let seller_id = encoder::u32_le(payload[0], payload[1], payload[2], payload[3]);
                let shop_slot = payload[4];
                let count = payload[5].max(1);

                out.send(format!(
                    "F4440A001A04{}00000000",
                    encoder::le32(conn.session.gold)
                ));
                out.send(conn.session.dump_homdo());
                let _ = (seller_id, shop_slot, count);
            }
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_npc_shop_buy() {
        let mut conn = Conn::new();
        conn.session.gold = 1000;

        let mut out = HandleOutcome::default();
        // Item 10001, count 1
        let payload = vec![0x11, 0x27, 0x01];
        handle_npc_shop(&mut conn, 1, &payload, &mut out);

        assert_eq!(conn.session.gold, 900);
        assert_eq!(conn.session.homdo.len(), 1);
        assert_eq!(conn.session.homdo[0].id, 10001);
        assert!(out.outgoing.iter().any(|f| f.contains("1A04")));
    }

    #[test]
    fn test_player_shop_open_close() {
        let mut conn = Conn::new();
        conn.session.id = 300001;

        let mut out1 = HandleOutcome::default();
        handle_player_shop(&mut conn, 30, &[], &mut out1);
        assert!(conn.session.shop.active);
        assert!(out1.outgoing.iter().any(|f| f.contains("171E")));

        let mut out2 = HandleOutcome::default();
        handle_player_shop(&mut conn, 31, &[], &mut out2);
        assert!(!conn.session.shop.active);
        assert!(out2.outgoing.iter().any(|f| f.contains("171F")));
    }
}
