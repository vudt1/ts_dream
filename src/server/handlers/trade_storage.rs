//! Trade (Opcode 0x19), Storage Transfer (Opcode 0x1E), & Bank Gold (Opcode 0x1D) handlers.

use crate::protocol::encoder;
use crate::server::handler::OpcodeCtx;
use crate::server::session::TradeState;

/// Handle Opcode 0x19 — Trade.
pub fn handle_trade(ctx: &mut OpcodeCtx) {
    let conn = &mut ctx.conn;
    let out = &mut ctx.out;
    let (sub, payload) = (ctx.sub, ctx.payload);
    match sub {
        // Sub 1: Request / open trade
        1 => {
            if payload.len() >= 4 {
                let partner_id = encoder::u32_le(payload[0], payload[1], payload[2], payload[3]);
                conn.session.trade = TradeState {
                    active: true,
                    partner_id,
                    ..Default::default()
                };
                out.send(format!("F44406001901{}", encoder::le32(partner_id)));
            }
        }
        // Sub 2: Offer gold & items
        2 => {
            if payload.len() >= 4 {
                let gold = encoder::u32_le(payload[0], payload[1], payload[2], payload[3]);
                conn.session.trade.gold = gold;
                out.send(format!("F44406001903{}", encoder::le32(gold)));
            }
        }
        // Sub 3: Confirm / cancel trade
        3 => {
            if payload.is_empty() {
                return;
            }
            if payload[0] == 1 {
                conn.session.trade.accepted = true;
                out.send("F4440300190204");
            } else {
                conn.session.trade = TradeState::default();
                out.send("F4440300190209");
            }
        }
        // Sub 10: Open pet trade
        10 => {
            if payload.len() >= 4 {
                let partner_id = encoder::u32_le(payload[0], payload[1], payload[2], payload[3]);
                out.send(format!("F4440600190A{}", encoder::le32(partner_id)));
            }
        }
        // Sub 11: Offer pet
        11 => {
            out.send("F4440300190B0A");
        }
        // Sub 12: Confirm pet trade
        12 => {
            out.send("F4440300190B04");
        }
        // Sub 20: Transfer item 1-way to recipient
        20 => {
            if payload.len() >= 8 {
                let recipient_id = encoder::u32_le(payload[4], payload[5], payload[6], payload[7]);
                let body = format!("{}01", encoder::le32(recipient_id));
                out.send(crate::protocol::frame("1706", &body));
                out.send(conn.session.dump_homdo());
            }
        }
        _ => {}
    }
}

/// Handle Opcode 0x1E — Storage Transfer (TienTrang).
pub fn handle_storage_transfer(ctx: &mut OpcodeCtx) {
    let conn = &mut ctx.conn;
    let out = &mut ctx.out;
    let (sub, payload) = (ctx.sub, ctx.payload);
    match sub {
        // Sub 1: TienTrang -> Homdo
        1 => {
            if payload.is_empty() {
                return;
            }
            let tt_slot = payload[0];
            if let Some(pos) = conn
                .session
                .tientrang
                .iter()
                .position(|i| i.slot == tt_slot)
            {
                let item = conn.session.tientrang.remove(pos);
                let _ = conn.session.add_homdo_item(item);
                out.send(conn.session.dump_homdo());
                out.send("F44402001732");
            }
        }
        // Sub 2: Homdo -> TienTrang
        2 => {
            if payload.is_empty() {
                return;
            }
            let homdo_slot = payload[0];
            if let Some(pos) = conn.session.homdo.iter().position(|i| i.slot == homdo_slot) {
                let mut item = conn.session.homdo.remove(pos);
                let next_slot = (conn.session.tientrang.len() + 1) as u8;
                item.slot = next_slot;
                conn.session.tientrang.push(item);

                out.send(format!("F44404001709{:02X}32", homdo_slot));

                let entries = format!("{:02X}0000000000000000000000", next_slot);
                out.send(crate::protocol::frame("1E04", &entries));
            }
        }
        // Sub 8: SelectMenu = 40
        8 => {
            conn.session.select_menu = 40;
        }
        _ => {}
    }
}

/// Handle Opcode 0x1D — Bank Gold.
pub fn handle_bank_gold(ctx: &mut OpcodeCtx) {
    let conn = &mut ctx.conn;
    let out = &mut ctx.out;
    let (sub, payload) = (ctx.sub, ctx.payload);
    if payload.len() < 2 {
        return;
    }
    let amount = encoder::u16_le(payload[0], payload[1]) as u32;

    match sub {
        // Sub 1: Withdraw bank gold
        1 => {
            if conn.session.bank_gold >= amount && conn.session.gold + amount <= 9_999_999 {
                conn.session.bank_gold -= amount;
                conn.session.gold += amount;

                out.send(format!("F44406001D02{}", encoder::le16(amount as u16)));
                out.send(format!("F44406001A01{}", encoder::le16(amount as u16)));
                out.send(format!(
                    "F4440A001A04{}00000000",
                    encoder::le32(conn.session.gold)
                ));
            }
        }
        // Sub 2: Deposit gold to bank
        2 => {
            if conn.session.gold >= amount {
                conn.session.gold -= amount;
                conn.session.bank_gold += amount;

                out.send(format!("F44406001D01{}", encoder::le16(amount as u16)));
                out.send(format!("F44406001A02{}", encoder::le16(amount as u16)));
                out.send(format!(
                    "F4440A001A04{}00000000",
                    encoder::le32(conn.session.gold)
                ));
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
    fn test_trade_open_and_confirm() {
        let mut conn = Conn::new();
        conn.session.id = 300001;

        let data = GameData::default();
        let service = BattleService::new(Arc::new(GameData::default()));

        let mut out1 = HandleOutcome::default();
        let payload_open = vec![0x02, 0x93, 0x04, 0x00]; // partner 300034
        let mut ctx = test_ctx(&mut conn, &data, &service, &mut out1, 1, &payload_open);
        handle_trade(&mut ctx);

        assert!(conn.session.trade.active);
        assert_eq!(conn.session.trade.partner_id, 299778);
        assert!(out1.outgoing[0].contains("1901"));

        let mut out2 = HandleOutcome::default();
        let mut ctx = test_ctx(&mut conn, &data, &service, &mut out2, 3, &[1]);
        handle_trade(&mut ctx);
        assert!(conn.session.trade.accepted);
        assert_eq!(out2.outgoing[0], "F4440300190204");
    }

    #[test]
    fn test_bank_deposit_withdraw() {
        let mut conn = Conn::new();
        conn.session.gold = 5000;
        conn.session.bank_gold = 1000;

        let data = GameData::default();
        let service = BattleService::new(Arc::new(GameData::default()));

        let mut out1 = HandleOutcome::default();
        // Deposit 1000 gold -> hex 0x03E8
        let mut ctx = test_ctx(&mut conn, &data, &service, &mut out1, 2, &[0xE8, 0x03]);
        handle_bank_gold(&mut ctx);

        assert_eq!(conn.session.gold, 4000);
        assert_eq!(conn.session.bank_gold, 2000);
        assert!(out1.outgoing.iter().any(|f| f.contains("1D01")));

        let mut out2 = HandleOutcome::default();
        // Withdraw 500 gold -> hex 0x01F4
        let mut ctx = test_ctx(&mut conn, &data, &service, &mut out2, 1, &[0xF4, 0x01]);
        handle_bank_gold(&mut ctx);

        assert_eq!(conn.session.gold, 4500);
        assert_eq!(conn.session.bank_gold, 1500);
        assert!(out2.outgoing.iter().any(|f| f.contains("1D02")));
    }
}
