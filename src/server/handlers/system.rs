//! System & Role handlers: PK/War (0x21), Game points (0x22), Rank (0x41), GM Shop (0x42), Teleport confirm (0x0C), Account Mgmt (0x23).

use crate::protocol::encoder;
use crate::server::handler::OpcodeCtx;
use crate::server::session::InventoryItem;
use crate::server::spawn::sys_msg_frame;

/// Handle Opcode 0x21 — PK / War Mode.
pub fn handle_pk_war(ctx: &mut OpcodeCtx) {
    let conn = &mut ctx.conn;
    let out = &mut ctx.out;
    let (sub, payload) = (ctx.sub, ctx.payload);
    if payload.is_empty() {
        return;
    }
    let flag = payload[0];

    match sub {
        1 => {
            conn.session.pk = if flag != 0 { 1 } else { 0 };
            out.send(format!(
                "F44404002102{:02X}{:02X}",
                conn.session.pk, conn.session.tham_chien
            ));
        }
        2 => {
            conn.session.tham_chien = if flag != 0 { 1 } else { 0 };
            out.send(format!(
                "F44404002102{:02X}{:02X}",
                conn.session.pk, conn.session.tham_chien
            ));
        }
        _ => {}
    }
}

/// Handle Opcode 0x22 — Game points / God panel.
pub fn handle_game_points(ctx: &mut OpcodeCtx) {
    let conn = &mut ctx.conn;
    let out = &mut ctx.out;
    out.send(format!(
        "F44412002304{}{}",
        encoder::le16(conn.session.gold as u16),
        "00".repeat(24)
    ));
}

/// Handle Opcode 0x41 — Rank system.
pub fn handle_rank(ctx: &mut OpcodeCtx) {
    let out = &mut ctx.out;
    let sub = ctx.sub;
    match sub {
        1 => out.send("F44402004101"),
        2 => out.send("F44402004102"),
        _ => {}
    }
}

/// Handle Opcode 0x42 — GM / Mall shop.
pub fn handle_gm_shop(ctx: &mut OpcodeCtx) {
    let conn = &mut ctx.conn;
    let out = &mut ctx.out;
    let (sub, payload) = (ctx.sub, ctx.payload);
    match sub {
        // Sub 1: Buy item from GM shop
        1 => {
            if payload.len() >= 6 {
                let item_id = encoder::u16_le(payload[2], payload[3]);
                let price = encoder::u16_le(payload[4], payload[5]) as u32;

                if conn.session.shop_point >= price {
                    conn.session.shop_point -= price;
                    let item = InventoryItem {
                        slot: 0,
                        id: item_id,
                        count: 1,
                        doben: 100,
                        loai: 1,
                        ..Default::default()
                    };
                    if conn.session.add_homdo_item(item).is_some() {
                        out.send(format!(
                            "F44406004202{}0100",
                            encoder::le16(conn.session.shop_point as u16)
                        ));
                        out.send(conn.session.dump_homdo());
                    } else {
                        conn.session.shop_point += price;
                        out.send("F44403001B0102"); // Inventory full
                    }
                }
            }
        }
        // Sub 3: Query GM shop points
        3 => {
            out.send(format!(
                "F44406004202{}0100",
                encoder::le16(conn.session.shop_point as u16)
            ));
        }
        _ => {}
    }
}

/// Handle Opcode 0x0C — Teleport confirm.
pub fn handle_teleport_confirm(ctx: &mut OpcodeCtx) {
    let conn = &mut ctx.conn;
    let out = &mut ctx.out;
    conn.session.idtalking = 0;
    conn.session.select_menu = 0;
    out.send("F44402000504F44402001408");
}

/// Handle Opcode 0x23 — Account Management (change pass, delete char, gift code).
pub fn handle_account_mgmt(ctx: &mut OpcodeCtx) {
    let conn = &mut ctx.conn;
    let out = &mut ctx.out;
    let (sub, payload) = (ctx.sub, ctx.payload);
    match sub {
        // Sub 1: Change password
        1 => {
            // Check old password matches
            if !conn.session.pending_pass.is_empty() && payload.starts_with(&conn.session.pending_pass) {
                out.send("F4440300230101");
            } else {
                out.send("F4440300230102");
            }
        }
        // Sub 2: Delete character
        2 => {
            conn.session.logined = false;
            out.send("F4440300230201");
            out.shutdown = true;
        }
        // Sub 3: Redeem item_code / gift code
        3 => {
            if conn.session.gift_code_redeemed {
                out.send(sys_msg_frame("Ma qua tang da duoc su dung!"));
            } else {
                conn.session.gift_code_redeemed = true;
                let gift = InventoryItem {
                    slot: 0,
                    id: 20002, // Stat point book
                    count: 1,
                    doben: 100,
                    loai: 1,
                    ..Default::default()
                };
                let _ = conn.session.add_homdo_item(gift);
                out.send(sys_msg_frame("Nhan ma qua tang thanh cong!"));
                out.send(conn.session.dump_homdo());
            }
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::battle::service::BattleService;
    use crate::data::loader::GameData;
    use crate::server::handler::{test_ctx, HandleOutcome};
    use crate::server::session::Conn;
    use std::sync::Arc;

    #[test]
    fn test_pk_and_war_toggle() {
        let mut conn = Conn::new();
        let data = GameData::default();
        let service = BattleService::new(Arc::new(GameData::default()));

        let mut out = HandleOutcome::default();
        let mut ctx = test_ctx(&mut conn, &data, &service, &mut out, 1, &[1]);
        handle_pk_war(&mut ctx);
        assert_eq!(conn.session.pk, 1);
        assert_eq!(out.outgoing[0], "F444040021020100");

        let mut out2 = HandleOutcome::default();
        let mut ctx = test_ctx(&mut conn, &data, &service, &mut out2, 2, &[1]);
        handle_pk_war(&mut ctx);
        assert_eq!(conn.session.tham_chien, 1);
        assert_eq!(out2.outgoing[0], "F444040021020101");
    }

    #[test]
    fn test_gm_shop_buy() {
        let mut conn = Conn::new();
        conn.session.shop_point = 500;

        let data = GameData::default();
        let service = BattleService::new(Arc::new(GameData::default()));
        let mut out = HandleOutcome::default();
        // Buy item 10001 (0x2711) for 200 pts (0x00C8) -> payload: 0, 0, 0x11, 0x27, 0xC8, 0x00
        let payload = vec![0, 0, 0x11, 0x27, 0xC8, 0x00];
        let mut ctx = test_ctx(&mut conn, &data, &service, &mut out, 1, &payload);
        handle_gm_shop(&mut ctx);

        assert_eq!(conn.session.shop_point, 300);
        assert_eq!(conn.session.homdo.len(), 1);
        assert!(out.outgoing.iter().any(|f| f.contains("4202")));
    }
}
