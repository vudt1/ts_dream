//! Chat & slash commands handler (Opcode 0x02).
//!
//! Channel sub-opcodes come from `crate::protocol::CHAT_SUB_*` (labels verified
//! against the aLogin client in `opcode_02.md`); slash commands are intercepted
//! on [`CHAT_SUB_NEAR`](crate::protocol::CHAT_SUB_NEAR). Cross-session routing
//! (whisper targets, party members, global broadcast) goes through
//! `ServerControl` when the live hub is present; golden replay (hub = None)
//! degrades to self-echo only.

use crate::db::persist;
use crate::protocol::encoder;
use crate::protocol::{
    CHAT_SUB_ANGEL, CHAT_SUB_GM, CHAT_SUB_GUILD, CHAT_SUB_LOUDSPEAKER, CHAT_SUB_NEAR,
    CHAT_SUB_WHISPER,
};
use crate::server::dispatcher::{HandleOutcome, MapBroadcast, OpcodeCtx};
use crate::server::gm;
use crate::server::handlers::stats;
use crate::server::session::{lock_online_sessions, Conn};
use crate::server::spawn;
use crate::web::server_control::ServerControl;

/// Maximum chat message length in characters (T4.1). Applies to sub 2
/// (map chat) and sub 3 (whisper), counted by `chars()` after
/// `viscii_decode`. Sub 5/6 have no length check.
const MAX_CHAT_CHARS: usize = 120;

/// Anti-spam cooldown between accepted chat messages: 5 seconds per message.
/// Applies to user chat subs (near/whisper/party/guild); slash commands and
/// GM broadcasts are exempt. Stamped on `Session::last_chat_ms`, runtime-only.
pub const CHAT_COOLDOWN_MS: u64 = 5_000;

fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

/// Enforce the 5s anti-spam cooldown. Returns false (after sending a `020B`
/// wait notice) when the sender must wait; on success stamps `last_chat_ms`
/// and returns true.
fn chat_cooldown_ok(conn: &mut Conn, out: &mut HandleOutcome) -> bool {
    let now = now_ms();
    let elapsed = now.saturating_sub(conn.session.last_chat_ms);
    if elapsed < CHAT_COOLDOWN_MS {
        let wait = CHAT_COOLDOWN_MS.saturating_sub(elapsed).div_ceil(1000);
        out.send(spawn::sys_msg_frame(&format!(
            "Ban chat qua nhanh, vui long doi {wait} giay."
        )));
        return false;
    }
    conn.session.last_chat_ms = now;
    true
}

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

/// Op 0x02 — Chat & slash commands.
pub async fn handle_chat(ctx: &mut OpcodeCtx<'_>) {
    let conn = &mut ctx.conn;
    let out = &mut ctx.out;
    let hub = ctx.env.hub;
    let pool = ctx.env.pool;
    let (sub, payload) = (ctx.sub, ctx.payload);
    match sub {
        // Sub 1: World / All chat — DISABLED (T2.5). The client gates sub 1
        // (class in [5..8] + flag +0x16D / magic 0xB3B6), so ordinary players
        // never see these messages. C->S sub 1 is dropped + logged until a
        // verified world channel exists. Code below kept for reference only.
        CHAT_SUB_ANGEL => {
            tracing::debug!(
                sender_id = conn.session.id,
                payload_len = payload.len(),
                "dropped C->S sub 1 world chat: client gate sub 1, waiting for verified world channel"
            );
            // let text = crate::encoding::viscii_decode(payload);
            // if text.chars().count() > 60 {
            //     return;
            // }
            // let frame = spawn::chat_frame(1, conn.session.id, payload);
            // out.send(&frame);
            // if let Some(hub) = hub {
            //     hub.broadcast_except(conn.session.id, &frame).await;
            // }
        }
        // Sub 2 (Gần): map chat (+ slash commands, exempt from cooldown).
        CHAT_SUB_NEAR => {
            let text = crate::encoding::viscii_decode(payload);
            if text.chars().count() > MAX_CHAT_CHARS {
                tracing::warn!(
                    sender_id = conn.session.id,
                    len_chars = text.chars().count(),
                    "dropped sub 2 chat: message too long"
                );
                out.send(spawn::sys_msg_frame("Tin nhan qua dai."));
                return;
            }
            if text.starts_with('/') {
                let msg = text.trim();
                if gm::handle(conn, out, pool, ctx.env.repos, hub, ctx.data, msg).await {
                    return;
                }
                handle_slash(conn, out, pool, hub, msg).await;
                return;
            }
            // Map-only chat (T1.4: promotion 2→1 via item 23100 removed;
            // sub 1 gate on the client would hide such messages anyway).
            // 5s anti-spam cooldown (slash commands above are exempt).
            if !chat_cooldown_ok(conn, out) {
                return;
            }
            let frame = spawn::chat_frame(CHAT_SUB_NEAR, conn.session.id, payload);
            out.send(&frame);
            if let Some(hub) = hub {
                // Map-scoped fan-out: every same-map peer except the sender.
                // Scope resolves through the `online_sessions()` snapshot
                // (P3 — other maps get nothing).
                hub.broadcast_map(
                    conn.session.id,
                    &[MapBroadcast {
                        subject: conn.session.id,
                        map_id: None,
                        frame,
                    }],
                )
                .await;
            }
        }
        // Sub 3 (Thì Thầm): both the sender and the recipient
        // receive op 0x02 sub 0x03 carrying the *sender* id (T1.1).
        CHAT_SUB_WHISPER => {
            if payload.len() < 4 {
                return;
            }
            let target_id = encoder::u32_le(payload[0], payload[1], payload[2], payload[3]);
            let chat_raw = &payload[4..];
            let text = crate::encoding::viscii_decode(chat_raw);
            if text.chars().count() > MAX_CHAT_CHARS {
                tracing::warn!(
                    sender_id = conn.session.id,
                    len_chars = text.chars().count(),
                    "dropped sub 3 whisper: message too long"
                );
                out.send(spawn::sys_msg_frame("Tin nhan qua dai."));
                return;
            }
            // T2.2: recipient offline -> 020B notice to sender, no chat frame.
            let online = lock_online_sessions().contains_key(&target_id);
            if !online {
                out.send(spawn::sys_msg_frame("Nguoi choi khong online."));
                return;
            }
            // 5s anti-spam cooldown.
            if !chat_cooldown_ok(conn, out) {
                return;
            }
            let frame = spawn::chat_frame(CHAT_SUB_WHISPER, conn.session.id, chat_raw);
            out.send(&frame); // sender's copy
            if let Some(hub) = hub {
                hub.send_to(target_id, &frame).await; // recipient's copy
            }
        }
        // Sub 4 (GM): GM-only broadcast (T1.2). Non-GM senders are dropped
        // silently (no client hint) to avoid revealing the privilege boundary.
        // GM traffic is exempt from the anti-spam cooldown.
        CHAT_SUB_GM => {
            if conn.session.gm_level <= 0 {
                tracing::warn!(
                    sender_id = conn.session.id,
                    gm_level = conn.session.gm_level,
                    "dropped sub 4 chat from non-GM sender"
                );
                return;
            }
            let frame = spawn::chat_frame(CHAT_SUB_GM, conn.session.id, payload);
            out.send(&frame);
            if let Some(hub) = hub {
                hub.broadcast_except(conn.session.id, &frame).await;
            }
        }
        // Sub 5 (Đài): leader + all members receive the frame.
        // T2.3 (partial): sender with no party -> 020B error instead of a
        // silent self-echo. Live-registry resolution is a future ticket.
        CHAT_SUB_LOUDSPEAKER => {
            let in_party =
                conn.session.id_leader != 0 || conn.session.id_mem.iter().any(|&m| m != 0);
            if !in_party {
                out.send(spawn::sys_msg_frame("Ban chua tham gia doi."));
                return;
            }
            // 5s anti-spam cooldown.
            if !chat_cooldown_ok(conn, out) {
                return;
            }
            let frame = spawn::chat_frame(CHAT_SUB_LOUDSPEAKER, conn.session.id, payload);
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
        // Sub 6 (Đoàn): echo-only until guild/army
        // membership exists (T1.3). No server-wide broadcast.
        CHAT_SUB_GUILD => {
            // 5s anti-spam cooldown (echo-only traffic still counts).
            if !chat_cooldown_ok(conn, out) {
                return;
            }
            let frame = spawn::chat_frame(CHAT_SUB_GUILD, conn.session.id, payload);
            out.send(&frame);
            tracing::debug!(
                sender_id = conn.session.id,
                "sub 6 army/guild chat unimplemented; echo-only"
            );
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
        "/endtalk" | "/offq" => {
            conn.session.idtalking = 0;
            conn.session.select_menu = 0;
            out.send("F44402001408".to_string());
        }
        "/help" => {
            out.send(spawn::sys_msg_frame(
                "/where /endtalk /offq /sleep /openhotel /openbank /openstore",
            ));
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
            // T3.3: Bear-parity confirmation announce after a successful
            // self-heal, before party-leader propagation. The battle_id>0
            // guard above and leader-only propagation below are unchanged.
            out.send(spawn::sys_msg_frame("Sleep command executed."));
            // Party-leader propagation: each online member gets the same
            // treatment through its own session snapshot in
            // `online_sessions()` and its client sender.
            if conn.session.id_leader == player_id {
                for &member_id in &conn.session.id_mem {
                    if member_id == 0 || member_id == player_id {
                        continue;
                    }
                    let member = {
                        let sessions = lock_online_sessions();
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
                        let mut sessions = lock_online_sessions();
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
        // Bot/auto group stubs (T3.2): reply 020B "not supported"
        // instead of a silent drop so players know the command is dead.
        "/bot" | "/autoboom" | "/autosell" | "/ai" | "/hpsp" | "/potion" | "/combo" => {
            out.send(spawn::sys_msg_frame("Lenh nay chua duoc ho tro."));
        }
        // Unknown `/cmd` — silently dropped.
        _ => {}
    }
}
