//! Chat & slash commands handler (Opcode 0x02).
//!
//! Sub 2 (global/map chat + slash commands), sub 3 (whisper), sub 4 (no-op),
//! sub 5 (party chat). Cross-session routing (whisper targets, party members,
//! global broadcast) goes through `ServerControl` when the live hub is present;
//! golden replay (hub = None) degrades to self-echo only.

use crate::db::persist;
use crate::protocol::encoder;
use crate::server::dispatcher::{HandleOutcome, MapBroadcast, OpcodeCtx};
use crate::server::gm;
use crate::server::handlers::stats;
use crate::server::session::{online_sessions, Conn};
use crate::server::spawn;
use crate::web::server_control::ServerControl;

/// Pet stat update frame: `F4440F00080204` + `le16(stt)`
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
/// (id 23100 → broadcast to every client, else map-only).
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
                return; // Dropped when longer than 60 chars
            }
            if text.starts_with('/') {
                let msg = text.trim();
                if gm::handle(conn, out, pool, ctx.env.repos, hub, ctx.data, msg).await {
                    return;
                }
                handle_slash(conn, out, pool, hub, msg).await;
                return;
            }
            // Channel selection: global item → op 0x02 sub
            // 0x01 to every client; otherwise map chat → sub 0x02.
            if wears_global_chat_item(conn) {
                let frame = spawn::chat_frame(1, conn.session.id, payload);
                out.send(&frame);
                if let Some(hub) = hub {
                    // Fan out to every client *except* the sender (the sender's
                    // own copy is echoed via `out` above).
                    hub.broadcast_except(conn.session.id, &frame).await;
                }
            } else {
                let frame = spawn::chat_frame(2, conn.session.id, payload);
                out.send(&frame);
                if let Some(hub) = hub {
                    // Map-scoped fan-out: every same-map peer except the sender.
                    // Scope resolves through the `online_sessions()` snapshot
                    // (P3 — other maps get nothing).
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
        // Sub 3: Whisper: both the sender and the recipient
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
        // Sub 4: No-op
        4 => {}
        // Sub 5: Party chat: leader + all members receive the frame.
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

/// Dispatch ordinary player slash commands. Privileged commands are handled
/// by `server::gm` before this function and never reach this fallback.
async fn handle_slash(
    conn: &mut Conn,
    out: &mut HandleOutcome,
    pool: Option<&crate::db::pool::DbPool>,
    hub: Option<&ServerControl>,
    msg: &str,
) {
    let (cmd, _args) = split_slash(msg);
    let cmd_lower = cmd.to_lowercase();
    let player_id = conn.session.id;

    // Legacy privileged commands were removed together with the admin role;
    // unrecognized or disabled `/cmd`s fall through to the silent-drop arm.

    // --- Player commands (all players equal) ---
    match cmd_lower.as_str() {
        "/where" => {
            // Reply `"MapID:<id> X:<x> Y:<y>"` as a VISCII-encoded system
            // message — `sys_msg_frame` handles the encoding.
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
            // Heal self, then pets stt 1..4, then — as party leader — every
            // online member.
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
            // Party-leader propagation: each online member gets the same
            // treatment through its own session snapshot in
            // `online_sessions()` and its client sender.
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
                    for pet in m
                        .pets
                        .iter()
                        .filter(|p| p.id > 0 && (1..=4).contains(&p.stt))
                    {
                        persist::upsert_pet(pool, member_id, pet).await;
                    }
                }
            }
        }
        "/openhotel" => {
            // One `1F06` frame per stable slot stt 5..10 (empty slots keep id 0),
            // concatenated in a single write, then `F44402001F07`; EndTalk only
            // while a dialog is active.
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
            // Bank screen showing the real stored money via `bank_gold`,
            // consistent with the bank handler (`trade_storage` / `talk.rs`).
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
        // Unknown `/cmd` — silently dropped.
        _ => {}
    }
}
