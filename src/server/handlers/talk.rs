//! Action / Talk handler (Opcode 0x14): H1 start talk, H6 menus, H4 end talk, H8 warp talk, H9 select menu.

use crate::data::loader::GameData;
use crate::protocol::encoder;
use crate::server::handler::{HandleOutcome, OpcodeCtx};
use crate::server::handlers::stats::build_stat_update;
use crate::server::session::{Conn, InventoryItem};

/// Helper: EndTalk packet + reset talk state.
pub fn end_talk(conn: &mut Conn, out: &mut HandleOutcome) {
    out.send("F44402001408");
    conn.session.idtalking = 0;
    conn.session.select_menu = 0;
}

/// Helper: Split dialog hex string on literal "F444" and send each fragment.
pub fn talk_messages(conn: &mut Conn, talk_string: &str, out: &mut HandleOutcome) {
    let parts: Vec<&str> = talk_string.split("F444").collect();
    for part in parts {
        if !part.is_empty() {
            let frame = format!("F444{part}");
            if frame == "F44402001408" {
                conn.session.select_menu = 40;
            }
            out.send(frame);
        }
    }
}

/// Dispatch Opcode 0x14 — Action / Talk.
pub fn handle_talk(ctx: &mut OpcodeCtx) {
    let conn = &mut ctx.conn;
    let out = &mut ctx.out;
    let data = ctx.data;
    let (sub, payload) = (ctx.sub, ctx.payload);
    match sub {
        // Sub 1: Start talk (H1)
        1 => handle_talk_start(conn, payload, data, out),
        // Sub 4: End talk (H4)
        4 => end_talk(conn, out),
        // Sub 6: Menu / Continue engine (H6)
        6 => handle_talk_continue(conn, payload, data, out),
        // Sub 8: Warp talk (H8)
        8 => handle_talk_warp(conn, payload, data, out),
        // Sub 9: Set SelectMenu (H9)
        9 => handle_talk_select_menu(conn, payload),
        _ => end_talk(conn, out),
    }
}

fn handle_talk_start(conn: &mut Conn, payload: &[u8], data: &GameData, out: &mut HandleOutcome) {
    if payload.len() < 2 {
        end_talk(conn, out);
        return;
    }
    let idtalking = encoder::u16_le(payload[0], payload[1]) as i32;
    conn.session.idtalking = idtalking;
    // Resolve the real NPC id from the on-map index (C# FTalk.H1:19) — the
    // NPC-shop sell path keys on it.
    conn.session.idnpctalking = data
        .npc_on_map
        .iter()
        .find(|n| n.map_id == i64::from(conn.session.map_id) && n.id == i64::from(idtalking))
        .map(|n| n.npc_id as i32)
        .unwrap_or(0);

    // Special NPC IDs
    match idtalking {
        16080 | 16004 | 16011 | 16015 => {
            out.send("F44402000602");
            out.send(format!(
                "F44411001401000000010603{:02X}0000000000000100",
                idtalking
            ));
        }
        15002 | 16001 | 16016 => {
            out.send("F44402000602");
            out.send(format!(
                "F44411001401000000010603{:02X}0000000000000200",
                idtalking
            ));
        }
        16012 => {
            // Silent
        }
        _ => {
            // Generic dialog lookup in GameData
            let key = format!("{}:NPC:{}:0", conn.session.map_id, idtalking);
            if let Some(talk) = data.talks.get(&key) {
                out.send("F44402000602");
                talk_messages(conn, &talk.dialogs, out);
            } else {
                out.send("F44402000602");
                out.send(format!(
                    "F44411001401000000010103{:02X}000000000000C830",
                    idtalking
                ));
            }
        }
    }
}

fn handle_talk_continue(
    conn: &mut Conn,
    _payload: &[u8],
    data: &GameData,
    out: &mut HandleOutcome,
) {
    let idtalking = conn.session.idtalking;
    let select_menu = conn.session.select_menu;

    // Pet-reborn NPC exceptions (55002/59102/59011)
    if crate::server::handlers::quest::handle_pet_reborn_npc(conn, idtalking, out) {
        return;
    }

    // Daily quest (map 12711)
    if conn.session.map_id == 12711 {
        crate::server::handlers::quest::generate_daily_quest(conn, out);
        return;
    }

    match idtalking {
        // Banker / Store NPCs
        16080 | 16004 | 16011 | 16023 => match select_menu {
            30 => {
                out.send("F44403001D0900");
                out.send(format!(
                    "F44406001D04{}",
                    encoder::le32(conn.session.bank_gold)
                ));
                out.send("F44402001D05");
                out.send("F44402001409");
            }
            31 => {
                out.send("F44402001D06");
                out.send("F44402001409");
            }
            40 => end_talk(conn, out),
            _ => end_talk(conn, out),
        },

        // Inn / Hotel NPCs
        15002 | 16001 | 16016 | 15118 => match select_menu {
            30 => {
                out.send("F44411001401000000010603010000000000000100");
            }
            31 => {
                conn.session.hp = conn.session.hp_max;
                conn.session.sp = conn.session.sp_max;
                out.send(build_stat_update(0x19, conn.session.hp as i32));
                out.send(build_stat_update(0x1A, conn.session.sp as i32));
                end_talk(conn, out);
            }
            32 => {
                out.send("F44411001401000000010603010000000000000100");
            }
            33 => {
                let item = InventoryItem {
                    slot: 0,
                    id: 46016,
                    count: 2,
                    doben: 100,
                    loai: 1,
                    ..Default::default()
                };
                let _ = conn.session.add_homdo_item(item);
                end_talk(conn, out);
            }
            40 => end_talk(conn, out),
            _ => end_talk(conn, out),
        },

        // NPC 16015
        16015 => match select_menu {
            30 => out.send("F44411001401000000010603010000000000000200"),
            31 => {
                conn.session.hp = conn.session.hp_max;
                conn.session.sp = conn.session.sp_max;
                out.send(build_stat_update(0x19, conn.session.hp as i32));
                out.send(build_stat_update(0x1A, conn.session.sp as i32));
                end_talk(conn, out);
            }
            32 => out.send("F44411001401000000010603010000000000000200"),
            33 => {
                let item = InventoryItem {
                    slot: 0,
                    id: 46016,
                    count: 2,
                    doben: 100,
                    loai: 1,
                    ..Default::default()
                };
                let _ = conn.session.add_homdo_item(item);
                end_talk(conn, out);
            }
            40 => end_talk(conn, out),
            _ => end_talk(conn, out),
        },

        // Silent NPC
        16012 => {}

        // Generic NPC continuation — try data-driven quest path
        _ => {
            if !crate::server::handlers::quest::try_quest_h6(conn, data, out) {
                end_talk(conn, out);
            }
        }
    }
}

fn handle_talk_warp(conn: &mut Conn, payload: &[u8], data: &GameData, out: &mut HandleOutcome) {
    // C# H8 reads the warp id directly from the packet (bytes 6-7).
    if payload.len() >= 2 {
        conn.session.idtalking = encoder::u16_le(payload[0], payload[1]) as i32;
    }
    crate::server::handlers::quest::handle_warp_confirm(conn, data, out);
}

fn handle_talk_select_menu(conn: &mut Conn, payload: &[u8]) {
    if !payload.is_empty() {
        conn.session.select_menu = payload[0] as i32;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::battle::service::BattleService;
    use crate::server::handler::test_ctx;
    use std::sync::Arc;

    #[test]
    fn test_talk_start_banker() {
        let mut conn = Conn::new();
        let data = GameData::default();
        let service = BattleService::new(Arc::new(GameData::default()));
        let mut out = HandleOutcome::default();

        // Start talk with banker 16080 (0x3ED0) -> payload: 0xD0, 0x3E
        let mut ctx = test_ctx(&mut conn, &data, &service, &mut out, 1, &[0xD0, 0x3E]);
        handle_talk(&mut ctx);

        assert_eq!(conn.session.idtalking, 16080);
        assert_eq!(out.outgoing.len(), 2);
        assert_eq!(out.outgoing[0], "F44402000602");
    }

    #[test]
    fn test_talk_banker_menu_30() {
        let mut conn = Conn::new();
        let data = GameData::default();
        let service = BattleService::new(Arc::new(GameData::default()));
        conn.session.idtalking = 16080;
        conn.session.bank_gold = 5000;
        conn.session.select_menu = 30;

        let mut out = HandleOutcome::default();
        let mut ctx = test_ctx(&mut conn, &data, &service, &mut out, 6, &[]);
        handle_talk(&mut ctx);

        assert_eq!(out.outgoing.len(), 4);
        assert_eq!(out.outgoing[0], "F44403001D0900");
    }

    #[test]
    fn test_talk_end() {
        let mut conn = Conn::new();
        let data = GameData::default();
        let service = BattleService::new(Arc::new(GameData::default()));
        conn.session.idtalking = 16080;
        conn.session.select_menu = 30;

        let mut out = HandleOutcome::default();
        let mut ctx = test_ctx(&mut conn, &data, &service, &mut out, 4, &[]);
        handle_talk(&mut ctx);

        assert_eq!(conn.session.idtalking, 0);
        assert_eq!(conn.session.select_menu, 0);
        assert_eq!(out.outgoing, vec!["F44402001408"]);
    }
}
