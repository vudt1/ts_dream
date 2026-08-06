//! Chat & slash commands handler (Opcode 0x02) — C# `FChat.cs` / `Update_H2`.
//!
//! Sub 2 (global/map chat + slash commands), sub 3 (whisper), sub 4 (no-op),
//! sub 5 (party chat). Cross-session routing (whisper targets, party members,
//! global broadcast) goes through `ServerControl` when the live hub is present;
//! golden replay (hub = None) degrades to self-echo only.

use crate::db::persist;
use crate::protocol::encoder;
use crate::server::handler::{HandleOutcome, MapBroadcast, OpcodeCtx};
// use crate::server::handlers::quest::BattleTrigger; // (disabled: admin `/battle` reference)
use crate::server::handlers::stats;
use crate::server::session::{online_sessions, Conn};
// (Disabled admin reference used `InventoryItem`; the tests use it + `PetState`.)
// use crate::server::session::{InventoryItem, PetState};
use crate::server::spawn;
use crate::web::server_control::ServerControl;

// (Disabled — the admin role was removed; every account is a player. C#
// `Client.isAdmin()` (Client.cs:10163-10170) treated ids below 300012 as
// server/admin. Kept only as reference; do not re-enable without a role system.)
// fn is_admin(id: u32) -> bool {
//     id < crate::protocol::ADMIN_ID_THRESHOLD
// }

/// Pet stat update frame (C# `Data.PetUpdateData`): `F4440F00080204` + `le16(stt)`
/// + type (`19` Hp / `1A` Sp, Type_Status) + sign `01` + `le32(value)` + `00000000`.
fn pet_stat_frame(stt: u8, ty: u8, value: u16) -> String {
    format!(
        "F4440F00080204{}{:02X}01{}00000000",
        encoder::le16(stt as u16),
        ty,
        encoder::le32(value as u32)
    )
}

/// True when the player wears the global-chat item in Trangbi slot 6
/// (C# `FChat.H2`: id 23100 → broadcast to every client, else map-only).
fn wears_global_chat_item(conn: &Conn) -> bool {
    conn.session
        .trangbi
        .iter()
        .any(|i| i.slot == 6 && i.id == 23100)
}

/// Op 0x02 — Chat & slash commands.
pub async fn handle_chat(ctx: &mut OpcodeCtx<'_>) {
    let conn = &mut ctx.conn;
    let out = &mut ctx.out;
    let hub = ctx.env.hub;
    let pool = ctx.env.pool;
    let (sub, payload) = (ctx.sub, ctx.payload);
    match sub {
        // Sub 2: Global / Map chat (+ slash commands)
        2 => {
            let text = String::from_utf8_lossy(payload);
            if text.chars().count() > 60 {
                return; // Dropped if message > 60 chars (C# `FChat.H2`)
            }
            if text.starts_with('/') {
                let msg = text.trim();
                handle_slash(conn, out, pool, hub, msg).await;
                return;
            }
            // Channel selection (C# `Toan`/`Gan`): global item → op 0x02 sub
            // 0x01 to every client; otherwise map chat → sub 0x02.
            if wears_global_chat_item(conn) {
                let frame = spawn::chat_frame(1, conn.session.id, payload);
                out.send(&frame);
                if let Some(hub) = hub {
                    // C# broadcasts to every client *except* the sender (the C#
                    // client shows its own message locally; the Rust port also
                    // echoes to self via `out` above).
                    hub.broadcast_except(conn.session.id, &frame).await;
                }
            } else {
                let frame = spawn::chat_frame(2, conn.session.id, payload);
                out.send(&frame);
                if let Some(hub) = hub {
                    // Map-scoped fan-out (C# `SendToAllClientMapid`, Server.cs:596):
                    // every same-map peer except the sender. Scope resolves through
                    // the `online_sessions()` snapshot (P3 — other maps get nothing).
                    hub.broadcast_map(
                        conn.session.id,
                        &[MapBroadcast {
                            subject: conn.session.id,
                            frame,
                        }],
                    )
                    .await;
                }
            }
        }
        // Sub 3: Whisper (C# `ThiTham`): both the sender and the recipient
        // receive op 0x02 sub 0x03 carrying the *recipient* id.
        3 => {
            if payload.len() < 4 {
                return;
            }
            let target_id = encoder::u32_le(payload[0], payload[1], payload[2], payload[3]);
            let chat_raw = &payload[4..];
            if chat_raw.len() > 60 {
                return;
            }
            let frame = spawn::chat_frame(3, target_id, chat_raw);
            out.send(&frame); // sender's copy
            if let Some(hub) = hub {
                hub.send_to(target_id, &frame).await; // recipient's copy
            }
        }
        // Sub 4: No-op (C# `case 4: break;`)
        4 => {}
        // Sub 5: Party chat (C# `Doi`): leader + all members receive the frame.
        5 => {
            let frame = spawn::chat_frame(5, conn.session.id, payload);
            out.send(&frame);
            if let Some(hub) = hub {
                let id = conn.session.id;
                let leader = conn.session.id_leader;
                if leader > 0 && leader != id {
                    hub.send_to(leader, &frame).await;
                }
                for member in conn.session.id_mem {
                    if member > 0 && member != id {
                        hub.send_to(member, &frame).await;
                    }
                }
            }
        }
        _ => {}
    }
}

/// Parse `/cmd arg1[,count]` into `(cmd, args)`.
fn split_slash(msg: &str) -> (String, Vec<String>) {
    let mut it = msg.split_whitespace();
    let cmd = it.next().unwrap_or("").to_string();
    let args: Vec<String> = it.map(|s| s.to_string()).collect();
    (cmd, args)
}

/// Dispatch a slash command.
///
/// All players are equal — there is no admin role. The legacy C# admin slash
/// commands (`FChat.H2` + `Client.isAdmin()`) are disabled and kept below only
/// as a commented reference; typing one now falls into the silent-drop arm.
async fn handle_slash(
    conn: &mut Conn,
    out: &mut HandleOutcome,
    pool: Option<&sqlx::MySqlPool>,
    hub: Option<&ServerControl>,
    msg: &str,
) {
    let (cmd, _args) = split_slash(msg);
    let cmd_lower = cmd.to_lowercase();
    let player_id = conn.session.id;

    // ------------------------------------------------------------------
    // Disabled admin slash commands — reference only.
    // ------------------------------------------------------------------
    // Legacy C# `FChat.H2` admin block, gated by `Client.isAdmin()`:
    // `_My_Id < 300012`. The admin role was removed — every account is a
    // player. The commands are kept here, commented out, for future
    // reference; do not re-enable without reintroducing a role system.
    // (Note: in the live code `args` is bound as `_args` because no player
    // command uses it; restore the `args` name when re-enabling.)
    //
    // if admin {
    //     match cmd.as_str() {
    //         "/additem" => {
    //             if args.is_empty() {
    //                 return;
    //             }
    //             let (id_str, count) = match args[0].split_once(',') {
    //                 Some((id, c)) => (id.to_string(), c.parse::<u32>().unwrap_or(1)),
    //                 None => (args[0].clone(), 1),
    //             };
    //             let Ok(item_id) = id_str.parse::<u16>() else {
    //                 return;
    //             };
    //             if count > 50 {
    //                 return; // C# gate: `num <= 50`
    //             }
    //             if let Some(slot) = conn.session.add_homdo_item(InventoryItem {
    //                 id: item_id,
    //                 count: count.min(255) as u8,
    //                 doben: 100,
    //                 loai: 1,
    //                 ..Default::default()
    //             }) {
    //                 if let Some(item) = conn.session.homdo.iter().find(|i| i.slot == slot) {
    //                     persist::upsert_item(pool, player_id, "homdo", item).await;
    //                 }
    //                 out.send(conn.session.dump_homdo());
    //             }
    //             return;
    //         }
    //         "/addpet" => {
    //             if let Some(id_str) = args.first() {
    //                 if let Ok(pet_id) = id_str.parse::<u16>() {
    //                     if !conn.session.pets.iter().any(|p| p.id == pet_id) {
    //                         let stt = (1..=4)
    //                             .find(|s| !conn.session.pets.iter().any(|p| p.stt == *s))
    //                             .unwrap_or(1);
    //                         conn.session.pets.push(PetState {
    //                             stt,
    //                             id: pet_id,
    //                             level: 1,
    //                             hp: 100,
    //                             hp_max: 100,
    //                             sp: 100,
    //                             sp_max: 100,
    //                             ..Default::default()
    //                         });
    //                     }
    //                 }
    //             }
    //             return;
    //         }
    //         "/addskpoint" => {
    //             if let Some(n) = args.first().and_then(|s| s.parse::<u16>().ok()) {
    //                 conn.session.skill_point += n;
    //                 let val = conn.session.skill_point;
    //                 persist::update_player(pool, player_id, "SkillPoint", i64::from(val)).await;
    //                 let body = format!("2501{}00000000", encoder::le32(val as u32));
    //                 out.send(crate::protocol::frame("0801", &body));
    //             }
    //             return;
    //         }
    //         "/test" => {
    //             // C# `FChat.H2` `/test N`: build the 1721 sample-item dump.
    //             let mut text3 = String::from("0000000000000000000000000000000000");
    //             for (id, count, texp) in [
    //                 (62503u16, 1u16, 123u32),
    //                 (62502, 1, 999_999_999),
    //                 (27163, 11, 13),
    //                 (62501, 1, 213),
    //             ] {
    //                 text3.push_str(&encoder::le16(id));
    //                 text3.push_str(&encoder::le16(count));
    //                 text3.push_str(&encoder::le32(texp));
    //                 text3.push_str("0000000000000000");
    //             }
    //             out.send(crate::protocol::frame("1721", &text3));
    //             return;
    //         }
    //         "/reloadtalks" => {
    //             out.send(spawn::sys_msg_frame("Reload Talks Complete"));
    //             return;
    //         }
    //         "/loadnpcs" | "/loaditems" | "/loadscenes" => {
    //             let name = &cmd[1..];
    //             out.send(spawn::sys_msg_frame(&format!("Reload {name} Complete")));
    //             return;
    //         }
    //         "/battle" => {
    //             if let Some(n) = args.first().and_then(|s| s.parse::<u64>().ok()) {
    //                 // C# spawns a TEAMDEF battle with 10 random defender ids and
    //                 // a random terrain. Deterministic xorshift seed keeps the
    //                 // replay stable.
    //                 let mut state = player_id as u64 ^ n.wrapping_mul(0x9E3779B97F4A7C15);
    //                 let mut ids = Vec::with_capacity(10);
    //                 for _ in 0..10 {
    //                     state ^= state << 13;
    //                     state ^= state >> 7;
    //                     state ^= state << 17;
    //                     ids.push(10004 + (state % 215) as i64);
    //                 }
    //                 out.battle_trigger = Some(BattleTrigger {
    //                     teamdef: ids,
    //                     diahinh: 101 + (n % 117) as i32,
    //                 });
    //             }
    //             return;
    //         }
    //         "/packet" => {
    //             let ptype = args
    //                 .first()
    //                 .and_then(|s| s.parse::<u16>().ok())
    //                 .unwrap_or(0);
    //             let start = args
    //                 .get(1)
    //                 .and_then(|s| s.split(',').next())
    //                 .and_then(|s| s.parse::<u16>().ok())
    //                 .unwrap_or(0);
    //             let count = args
    //                 .get(1)
    //                 .and_then(|s| s.split(',').nth(1))
    //                 .and_then(|s| s.parse::<u16>().ok())
    //                 .unwrap_or(1);
    //             for i in start..start.saturating_add(count) {
    //                 out.send(format!("F4440200{:02X}{:02X}", ptype & 0xFF, i & 0xFF));
    //             }
    //             return;
    //         }
    //         "/sendpacket" => {
    //             if let Some(hex) = args.first() {
    //                 let hex = hex.trim();
    //                 if hex.len() % 2 == 0 && encoder::bytes(hex).is_some() {
    //                     out.send(hex.to_uppercase());
    //                 }
    //             }
    //             return;
    //         }
    //         "/warp" => {
    //             // The ticket marks `/warp` as an admin command; disabled here
    //             // with the rest of the privileged set.
    //             if let Some(map_str) = args.first() {
    //                 if let Ok(map_id) = map_str.parse::<u16>() {
    //                     conn.session.map_id = map_id;
    //                     let warp_frame = format!(
    //                         "F4440D000C{}{}{}{}00",
    //                         encoder::le32(player_id),
    //                         encoder::le16(map_id),
    //                         encoder::le16(conn.session.map_x),
    //                         encoder::le16(conn.session.map_y)
    //                     );
    //                     out.send(warp_frame);
    //                 }
    //             }
    //             return;
    //         }
    //         _ => {}
    //     }
    // }
    // ------------------------------------------------------------------

    // --- Player commands (all players equal) ---
    match cmd_lower.as_str() {
        "/where" => {
            // C# `FChat.H2` `/where`: `"MapID:" + mapId + " X:" + x + " Y:" + y`,
            // routed through `smethod_17` (VISCII) — `sys_msg_frame` does that.
            let info = format!(
                "MapID:{} X:{} Y:{}",
                conn.session.map_id, conn.session.map_x, conn.session.map_y
            );
            out.send(spawn::sys_msg_frame(&info));
        }
        "/endtalk" => {
            conn.session.idtalking = 0;
            conn.session.select_menu = 0;
            out.send("F44402001408".to_string());
        }
        "/sleep" => {
            if conn.session.battle_id > 0 {
                return;
            }
            // C# `Client.Sleep()` (Client.cs:646-846): heal self, then pets
            // stt 1..4, then — as party leader — every online member.
            out.send("F44402001F0A".to_string());
            if conn.session.hp < conn.session.hp_max {
                conn.session.hp = conn.session.hp_max;
                out.send(stats::build_stat_update(0x19, conn.session.hp as i32));
                persist::update_player(pool, player_id, "Hp", i64::from(conn.session.hp)).await;
            }
            if conn.session.sp < conn.session.sp_max {
                conn.session.sp = conn.session.sp_max;
                out.send(stats::build_stat_update(0x1A, conn.session.sp as i32));
                persist::update_player(pool, player_id, "Sp", i64::from(conn.session.sp)).await;
            }
            for pet in conn.session.pets.iter_mut() {
                if pet.id == 0 || !(1..=4).contains(&pet.stt) {
                    continue;
                }
                if pet.hp < pet.hp_max {
                    pet.hp = pet.hp_max;
                    out.send(pet_stat_frame(pet.stt, 0x19, pet.hp));
                }
                if pet.sp < pet.sp_max {
                    pet.sp = pet.sp_max;
                    out.send(pet_stat_frame(pet.stt, 0x1A, pet.sp));
                }
                persist::upsert_pet(pool, player_id, pet).await;
            }
            out.send("F44403001F0100".to_string());
            // Party-leader propagation (C# `Sleep`, Client.cs:686-846): each
            // online member gets the same treatment through its own session
            // snapshot in `online_sessions()` and its client sender.
            if conn.session.id_leader == player_id {
                for &member_id in &conn.session.id_mem {
                    if member_id == 0 || member_id == player_id {
                        continue;
                    }
                    let member = {
                        let sessions = online_sessions().lock().unwrap();
                        sessions.get(&member_id).cloned()
                    };
                    let Some(mut m) = member else {
                        continue;
                    };
                    let mut frames = vec!["F44402001F0A".to_string()];
                    if m.hp < m.hp_max {
                        m.hp = m.hp_max;
                        frames.push(stats::build_stat_update(0x19, m.hp as i32));
                    }
                    if m.sp < m.sp_max {
                        m.sp = m.sp_max;
                        frames.push(stats::build_stat_update(0x1A, m.sp as i32));
                    }
                    for pet in m.pets.iter_mut() {
                        if pet.id == 0 || !(1..=4).contains(&pet.stt) {
                            continue;
                        }
                        if pet.hp < pet.hp_max {
                            pet.hp = pet.hp_max;
                            frames.push(pet_stat_frame(pet.stt, 0x19, pet.hp));
                        }
                        if pet.sp < pet.sp_max {
                            pet.sp = pet.sp_max;
                            frames.push(pet_stat_frame(pet.stt, 0x1A, pet.sp));
                        }
                    }
                    frames.push("F44403001F0100".to_string());
                    {
                        let mut sessions = online_sessions().lock().unwrap();
                        sessions.insert(member_id, m.clone());
                    }
                    if let Some(hub) = hub {
                        for f in &frames {
                            hub.send_to(member_id, f).await;
                        }
                    }
                    persist::update_player(pool, member_id, "Hp", i64::from(m.hp)).await;
                    persist::update_player(pool, member_id, "Sp", i64::from(m.sp)).await;
                    for pet in m.pets.iter().filter(|p| p.id > 0 && (1..=4).contains(&p.stt)) {
                        persist::upsert_pet(pool, member_id, pet).await;
                    }
                }
            }
        }
        "/openhotel" => {
            // C# `OpenHotel` (Client.cs:10002): one `1F06` frame per stable slot
            // stt 5..10 (empty slots keep id 0), concatenated in a single write,
            // then `F44402001F07`; EndTalk only while a dialog is active.
            let mut text2 = String::new();
            for i in 5u8..=10 {
                let pet = conn.session.pets.iter().find(|p| p.stt == i);
                let (id, lv, hp, name): (u16, u8, u16, &[u8]) = match pet {
                    Some(p) => (p.id, p.level, p.hp, &p.name[..]),
                    None => (0, 0, 0, &[]),
                };
                let text = format!(
                    "{:02X}{}{:02X}{}{:02X}",
                    i - 4,
                    encoder::le16(id),
                    lv,
                    encoder::le16(hp),
                    name.len(),
                ) + &encoder::strhex(name);
                text2.push_str(&crate::protocol::frame("1F06", &text));
            }
            out.send(text2);
            if conn.session.idtalking > 0 {
                conn.session.idtalking = 0;
                conn.session.select_menu = 0;
                out.send("F44402001408".to_string());
            }
            conn.session.select_menu = 40;
            out.send("F44402001F07".to_string());
        }
        "/openbank" => {
            // C# `OpenBank` (Client.cs:10052): bank screen with the real storage
            // money (`TienTrangGetDataMoney`); Rust uses `bank_gold`, consistent
            // with the bank handler (`trade_storage` / `talk.rs`).
            out.send("F44403001D0900".to_string());
            out.send(format!(
                "F44406001D04{}",
                encoder::le32(conn.session.bank_gold)
            ));
            out.send("F44402001D05".to_string());
            out.send("F44402001409".to_string());
        }
        "/openstore" => {
            out.send("F44402001D06".to_string());
            out.send("F44402001409".to_string());
        }
        // Unknown `/cmd` — silently dropped (the C# server would broadcast the
        // text as chat; the Rust port deliberately ignores it).
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::battle::service::BattleService;
    use crate::data::loader::GameData;
    use crate::server::session::{Conn, InventoryItem, PetState};
    use std::sync::Arc;

    #[tokio::test]
    async fn map_chat_echoes_self() {
        let mut conn = Conn::new();
        conn.session.id = 300001;
        let mut out = HandleOutcome::default();
        let payload = b"HELLO";
        let data = GameData::default();
        let service = BattleService::new(Arc::new(GameData::default()));
        let mut ctx =
            crate::server::handler::test_ctx(&mut conn, &data, &service, &mut out, 2, payload);
        handle_chat(&mut ctx).await;
        assert_eq!(out.outgoing, vec!["F4440B000202E193040048454C4C4F"]);
    }

    #[tokio::test]
    async fn long_chat_dropped() {
        let mut conn = Conn::new();
        conn.session.id = 300015;
        let mut out = HandleOutcome::default();
        let payload = vec![b'X'; 61];
        let data = GameData::default();
        let service = BattleService::new(Arc::new(GameData::default()));
        let mut ctx =
            crate::server::handler::test_ctx(&mut conn, &data, &service, &mut out, 2, &payload);
        handle_chat(&mut ctx).await;
        assert!(out.outgoing.is_empty());
    }

    #[tokio::test]
    async fn global_chat_item_selects_0201() {
        let mut conn = Conn::new();
        conn.session.id = 300015;
        conn.session.trangbi.push(InventoryItem {
            slot: 6,
            id: 23100,
            ..Default::default()
        });
        let mut out = HandleOutcome::default();
        let data = GameData::default();
        let service = BattleService::new(Arc::new(GameData::default()));
        let mut ctx =
            crate::server::handler::test_ctx(&mut conn, &data, &service, &mut out, 2, b"HI");
        handle_chat(&mut ctx).await;
        assert_eq!(out.outgoing.len(), 1);
        assert!(out.outgoing[0].starts_with("F44408000201"));
    }

    #[tokio::test]
    async fn slash_where_returns_sysmsg() {
        let mut conn = Conn::new();
        conn.session.id = 300015;
        conn.session.map_id = 12001;
        conn.session.map_x = 400;
        conn.session.map_y = 500;
        let mut out = HandleOutcome::default();
        let data = GameData::default();
        let service = BattleService::new(Arc::new(GameData::default()));
        let mut ctx =
            crate::server::handler::test_ctx(&mut conn, &data, &service, &mut out, 2, b"/where");
        handle_chat(&mut ctx).await;
        assert_eq!(out.outgoing.len(), 1);
        assert!(out.outgoing[0].contains("020B"));
        assert!(out.outgoing[0].contains("3132303031")); // hex of "12001"
    }

    // Disabled admin slash tests (reference only — the commands they cover are
    // commented out in `handle_slash`).
    // #[tokio::test]
    // async fn slash_additem_requires_admin() {
    //     let mut conn = Conn::new();
    //     conn.session.id = 300015; // not admin
    //     let mut out = HandleOutcome::default();
    //     let data = GameData::default();
    //     let service = BattleService::new(Arc::new(GameData::default()));
    //     let mut ctx = crate::server::handler::test_ctx(
    //         &mut conn,
    //         &data,
    //         &service,
    //         &mut out,
    //         2,
    //         b"/additem 10001,2",
    //     );
    //     handle_chat(&mut ctx).await;
    //     assert!(
    //         conn.session.homdo.is_empty(),
    //         "non-admin /additem must be ignored"
    //     );
    // }

    // #[tokio::test]
    // async fn slash_additem_admin_adds_to_homdo() {
    //     let mut conn = Conn::new();
    //     conn.session.id = 1; // admin (< 300012)
    //     let mut out = HandleOutcome::default();
    //     let data = GameData::default();
    //     let service = BattleService::new(Arc::new(GameData::default()));
    //     let mut ctx = crate::server::handler::test_ctx(
    //         &mut conn,
    //         &data,
    //         &service,
    //         &mut out,
    //         2,
    //         b"/additem 10001,2",
    //     );
    //     handle_chat(&mut ctx).await;
    //     assert_eq!(conn.session.homdo.len(), 1);
    //     assert_eq!(conn.session.homdo[0].id, 10001);
    //     assert_eq!(conn.session.homdo[0].count, 2);
    //     assert!(out.outgoing.iter().any(|f| f.contains("1705")));
    // }

    // #[tokio::test]
    // async fn slash_battle_sets_trigger() {
    //     let mut conn = Conn::new();
    //     conn.session.id = 1;
    //     let mut out = HandleOutcome::default();
    //     let data = GameData::default();
    //     let service = BattleService::new(Arc::new(GameData::default()));
    //     let mut ctx =
    //         crate::server::handler::test_ctx(&mut conn, &data, &service, &mut out, 2, b"/battle 5");
    //     handle_chat(&mut ctx).await;
    //     let trigger = out.battle_trigger.as_ref().expect("battle trigger set");
    //     assert_eq!(trigger.teamdef.len(), 10);
    //     assert!(trigger.diahinh >= 101);
    // }

    #[tokio::test]
    async fn unknown_slash_silently_dropped() {
        // A disabled admin command (or any unrecognized `/cmd`) produces no
        // reply and no broadcast — the C# server would broadcast it as chat.
        let mut conn = Conn::new();
        conn.session.id = 300015;
        let mut out = HandleOutcome::default();
        let data = GameData::default();
        let service = BattleService::new(Arc::new(GameData::default()));
        let mut ctx = crate::server::handler::test_ctx(
            &mut conn,
            &data,
            &service,
            &mut out,
            2,
            b"/additem 10001,2",
        );
        handle_chat(&mut ctx).await;
        assert!(
            out.outgoing.is_empty(),
            "unknown /cmd must be silently dropped"
        );
        assert!(conn.session.homdo.is_empty());
    }

    #[tokio::test]
    async fn sleep_heals_self_and_pets() {
        let mut conn = Conn::new();
        conn.session.id = 300015;
        conn.session.hp = 10;
        conn.session.hp_max = 100;
        conn.session.sp = 5;
        conn.session.sp_max = 50;
        conn.session.pets.push(PetState {
            stt: 1,
            id: 18001,
            level: 1,
            hp: 10,
            hp_max: 90,
            sp: 10,
            sp_max: 40,
            ..Default::default()
        });
        let mut out = HandleOutcome::default();
        let data = GameData::default();
        let service = BattleService::new(Arc::new(GameData::default()));
        let mut ctx =
            crate::server::handler::test_ctx(&mut conn, &data, &service, &mut out, 2, b"/sleep");
        handle_chat(&mut ctx).await;
        assert_eq!(conn.session.hp, 100);
        assert_eq!(conn.session.sp, 50);
        assert_eq!(conn.session.pets[0].hp, 90);
        assert_eq!(conn.session.pets[0].sp, 40);
        let joined = out.outgoing.join("");
        assert!(joined.contains("1F0A"));
        assert!(joined.contains("080204")); // pet Hp/Sp stat frames
        assert!(joined.contains("1F0100")); // sleep done
    }

    #[tokio::test]
    async fn sleep_skips_when_in_battle() {
        let mut conn = Conn::new();
        conn.session.id = 300015;
        conn.session.battle_id = 7;
        conn.session.hp = 10;
        conn.session.hp_max = 100;
        let mut out = HandleOutcome::default();
        let data = GameData::default();
        let service = BattleService::new(Arc::new(GameData::default()));
        let mut ctx =
            crate::server::handler::test_ctx(&mut conn, &data, &service, &mut out, 2, b"/sleep");
        handle_chat(&mut ctx).await;
        assert!(out.outgoing.is_empty());
        assert_eq!(conn.session.hp, 10);
    }

    #[tokio::test]
    async fn openhotel_builds_stable_frames() {
        let mut conn = Conn::new();
        conn.session.id = 300015;
        conn.session.pets.push(PetState {
            stt: 5,
            id: 18001,
            level: 3,
            hp: 80,
            name: b"PET".to_vec(),
            ..Default::default()
        });
        let mut out = HandleOutcome::default();
        let data = GameData::default();
        let service = BattleService::new(Arc::new(GameData::default()));
        let mut ctx = crate::server::handler::test_ctx(
            &mut conn,
            &data,
            &service,
            &mut out,
            2,
            b"/openhotel",
        );
        handle_chat(&mut ctx).await;
        let joined = out.outgoing.join("");
        assert!(joined.contains("1F06"));
        assert!(joined.contains("504554")); // hex of "PET"
        assert!(joined.ends_with("1F07"));
        assert_eq!(conn.session.select_menu, 40);
    }

    #[tokio::test]
    async fn openbank_uses_bank_gold() {
        let mut conn = Conn::new();
        conn.session.id = 300015;
        conn.session.bank_gold = 4321;
        let mut out = HandleOutcome::default();
        let data = GameData::default();
        let service = BattleService::new(Arc::new(GameData::default()));
        let mut ctx = crate::server::handler::test_ctx(
            &mut conn,
            &data,
            &service,
            &mut out,
            2,
            b"/openbank",
        );
        handle_chat(&mut ctx).await;
        // 4321 = 0x10E1 → LE bytes E1 10 00 00
        assert!(out.outgoing.iter().any(|f| f.contains("1D04") && f.contains("E1100000")));
    }

    #[tokio::test]
    async fn whisper_builds_frame_with_recipient_id() {
        let mut conn = Conn::new();
        conn.session.id = 300015;
        let mut out = HandleOutcome::default();
        // target id 300016 (0x1043F9... LE: 0x10 0x39 0x04 0x00 -> 300016)
        let payload: Vec<u8> = [0x10, 0x39, 0x04, 0x00].to_vec();
        let mut payload = payload;
        payload.extend_from_slice(b"hey");
        let data = GameData::default();
        let service = BattleService::new(Arc::new(GameData::default()));
        let mut ctx =
            crate::server::handler::test_ctx(&mut conn, &data, &service, &mut out, 3, &payload);
        handle_chat(&mut ctx).await;
        assert_eq!(out.outgoing.len(), 1);
        assert!(out.outgoing[0].starts_with("F44409000203"));
        assert!(out.outgoing[0].contains("10390400"));
        assert!(out.outgoing[0].ends_with("686579"));
    }

    #[tokio::test]
    async fn party_chat_frame_uses_0205() {
        let mut conn = Conn::new();
        conn.session.id = 300015;
        let mut out = HandleOutcome::default();
        let data = GameData::default();
        let service = BattleService::new(Arc::new(GameData::default()));
        let mut ctx = crate::server::handler::test_ctx(
            &mut conn,
            &data,
            &service,
            &mut out,
            5,
            b"hello party",
        );
        handle_chat(&mut ctx).await;
        assert_eq!(out.outgoing.len(), 1);
        assert!(out.outgoing[0].starts_with("F44411000205"));
    }
}
