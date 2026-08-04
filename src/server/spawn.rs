//! World-spawn (Logined1) and login packet builders (Chapter 2 §2.3/§2.4).
//!
//! Pure functions producing the exact byte stream; tested without a socket.
//! The 22-step `Logined1` sequence and the login gate responses live here.

use crate::protocol::encoder;
use crate::server::handler::HandleOutcome;
use std::sync::atomic::{AtomicI64, Ordering};

/// Wall-clock override (unix seconds) for the deterministic golden replay of
/// the `Thoi gian` banner (Ch9 §9.2 — timing-dependent frames are not locked).
/// `0` = real clock; a fixed value makes Logined1 byte-deterministic.
static FIXED_NOW: AtomicI64 = AtomicI64::new(0);

/// Pin the time banner to a fixed unix timestamp (deterministic golden replay).
pub fn override_now(unix_secs: i64) {
    FIXED_NOW.store(unix_secs, Ordering::SeqCst);
}

/// Restore the real wall clock for the time banner.
pub fn reset_now() {
    FIXED_NOW.store(0, Ordering::SeqCst);
}

fn now_banner() -> String {
    match FIXED_NOW.load(Ordering::SeqCst) {
        0 => chrono::Local::now()
            .format("Thoi gian: %Y-%m-%d %H:%M:%S")
            .to_string(),
        // Deterministic golden replay (Ch9 §9.2): fixed instant, UTC, so the
        // banner never depends on the machine's wall clock or timezone.
        t => chrono::DateTime::from_timestamp(t, 0)
            .expect("valid fixed timestamp")
            .with_timezone(&chrono::Utc)
            .format("Thoi gian: %Y-%m-%d %H:%M:%S")
            .to_string(),
    }
}

/// Login failure responses (op 0x01).
pub const LOGIN_WRONG_PASS: &str = "F44402000106";
pub const LOGIN_CREATE_CHAR: &str = "F4440300010300";
pub const HELLO_REPLY: &str = "F4440300010901";
/// Op 0x03 enter-game when not authed → create char screen.
pub const ENTER_GAME_CREATE: &str = "F4440300010300";

/// Step 1 of Logined: end-talk + the `F4440300142100` marker.
pub fn login_start() -> Vec<String> {
    vec!["F44402001408".to_string(), "F4440300142100".to_string()]
}

/// Player self-appear frame — op 0x03 sub 0x03 (Logined  step 2).
pub fn player_appear(
    id: u32,
    sex: u8,
    ghost: u8,
    god: u8,
    map: u16,
    x: u16,
    y: u16,
    dir: u8,
    hair: u16,
    color: &str,
    equipped_ids: &[u16],
    reborn: u8,
    job: u8,
    name: &[u8],
) -> String {
    // Shared body (header: opcode/sub are included in length computation).
    let mut body = String::new();
    body.push_str(&encoder::le32(id));
    body.push_str(&format!("{:02X}{:02X}{:02X}", sex, ghost, god));
    body.push_str(&encoder::le16(map));
    body.push_str(&encoder::le16(x));
    body.push_str(&encoder::le16(y));
    body.push_str(&format!("{:02X}", dir));
    body.push_str(&encoder::le16(hair));
    body.push_str(color);
    body.push_str(&format!("{:02X}", equipped_ids.len()));
    for e in equipped_ids {
        body.push_str(&encoder::le16(*e));
    }
    // Tail: fixed `0000000005` + reborn + job + name.
    body.push_str("0000000005");
    body.push_str(&format!("{:02X}{:02X}", reborn, job));
    body.push_str(&encoder::strhex(name));

    // Frame: F444 + len + 03 03 + body. len counts bytes after the header,
    // i.e. 2 (opcode/sub) + body_len.
    let total_len = 2 + body.len() / 2;
    let mut frame = String::from("F444");
    frame.push_str(&encoder::le16(total_len as u16));
    frame.push_str("0303");
    frame.push_str(&body);
    frame
}

/// Stats frame — op 0x05 sub 0x03 (Logined  step 3). `skills_hex` = the
/// player's skill list hex (`««SKILL_ID Lv` pairs are handled by the caller
/// via [`skill_list`]). Length counts `skills/2 + 113` bytes.
pub fn stats(
    thuoctinh: u8,
    hp: u16,
    sp: u16,
    int1: u16,
    atk: u16,
    def: u16,
    agi: u16,
    hpx: u16,
    spx: u16,
    lv: u8,
    texp: u32,
    skill_point: u16,
    point: u16,
    tiengtam: u16,
    hp_max: u16,
    sp_max: u16,
    atk2: u32,
    def2: u32,
    int2: u32,
    agi2: u32,
    hpx2: u32,
    spx2: u32,
    skills_hex: &str,
) -> String {
    let mut body = String::new();
    body.push_str(&format!("{:02X}", thuoctinh));
    for v in [
        encoder::le16(hp),
        encoder::le16(sp),
        encoder::le16(int1),
        encoder::le16(atk),
        encoder::le16(def),
        encoder::le16(agi),
        encoder::le16(hpx),
        encoder::le16(spx),
        format!("{:02X}", lv),
        encoder::le32(texp),
        encoder::le16(skill_point),
        encoder::le16(point),
        encoder::le16(tiengtam),
        encoder::le16(hp_max),
        encoder::le16(sp_max),
        encoder::le32(atk2),
        encoder::le32(def2),
        encoder::le32(int2),
        encoder::le32(agi2),
        encoder::le32(hpx2),
        encoder::le32(spx2),
    ] {
        body.push_str(&v);
    }
    // Literal `F401`×5 + 90 zero bytes + skill list.
    body.push_str("F401F401F401F401F401");
    body.push_str(&"00".repeat(90));
    body.push_str(skills_hex);

    let total_len = 2 + body.len() / 2;
    let mut frame = String::from("F444");
    frame.push_str(&encoder::le16(total_len as u16));
    frame.push_str("0503");
    frame.push_str(&body);
    frame
}

/// Build the skill-list hex from (skillId, level) pairs used in the stats
/// frame: each entry is `le16(skillId)` + `lv`.
pub fn skill_list(skills: &[(u16, u8)]) -> String {
    let mut s = String::new();
    for (id, lv) in skills {
        s.push_str(&encoder::le16(*id));
        s.push_str(&format!("{:02X}", lv));
    }
    s
}

/// Build the move broadcast frame (op 0x06 sub 0x01).
pub fn move_broadcast(id: u32, dir: u8, x: u16, y: u16) -> String {
    format!(
        "F4440B000601{}{}{:02X}{}{}",
        encoder::le32(id),
        "",
        dir,
        encoder::le16(x),
        encoder::le16(y)
    )
}

/// Build expression/action frame (op 0x20 sub 0x01 / 0x02).
pub fn expression_frame(id: u32, sub: u8, action: u8) -> String {
    format!(
        "F444070020{:02X}{}{:02X}",
        sub,
        encoder::le32(id),
        action
    )
}

/// Build chat packet frame (op 0x02 sub 0x01 / 0x02 / 0x03 / 0x05).
pub fn chat_frame(sub: u8, id: u32, chat_raw: &[u8]) -> String {
    let mut body = String::new();
    body.push_str(&encoder::le32(id));
    body.push_str(&encoder::hex(chat_raw));
    let total_len = 2 + body.len() / 2;
    format!("F444{}02{:02X}{}", encoder::le16(total_len as u16), sub, body)
}

/// Build system message banner packet (op 0x02 sub 0x0B).
pub fn sys_msg_frame(msg: &str) -> String {
    let mut body = String::from("00000000");
    body.push_str(&encoder::strhex(msg.as_bytes()));
    let total_len = 2 + body.len() / 2;
    format!("F444{}020B{}", encoder::le16(total_len as u16), body)
}

/// Build announcement packet (op 0x02 sub 0x0C).
pub fn announce_frame(msg: &str) -> String {
    let mut body = String::from("00000000");
    body.push_str(&encoder::strhex(msg.as_bytes()));
    let total_len = 2 + body.len() / 2;
    format!("F444{}020C{}", encoder::le16(total_len as u16), body)
}

/// Build server name packet (op 0x27 sub 0x09).
pub fn server_name_frame(id: u32, server_name: &str) -> String {
    let name_hex = encoder::strhex(server_name.as_bytes());
    let name_len = server_name.len() as u8;
    let mut body = String::new();
    body.push_str(&encoder::le32(id));
    body.push_str("C4000000");
    body.push_str(&format!("{:02X}", name_len));
    body.push_str(&name_hex);
    let total_len = 2 + body.len() / 2;
    format!("F444{}2709{}", encoder::le16(total_len as u16), body)
}

/// Build full 22-step Logined1 sequence frames for a logged-in character.
pub fn build_logined_sequence(
    id: u32,
    name: &[u8],
    sex: u8,
    hair: u16,
    color: &str,
    thuoctinh: u8,
    map_id: u16,
    map_x: u16,
    map_y: u16,
    gocnhin: u8,
    pk: u8,
    tham_chien: u8,
) -> Vec<String> {
    let mut frames = Vec::new();

    // 1. Step 1: end-talk + marker
    frames.extend(login_start());

    // 2. Step 2: player self-appear (op 0x03 sub 0x03)
    frames.push(player_appear(
        id, sex, 0, 0, map_id, map_x, map_y, gocnhin, hair, color, &[], 0, 0, name,
    ));

    // 3. Step 3: stats (op 0x05 sub 0x03)
    let hp_max = crate::battle::engine::get_hp_max(0, 0, 1, 0) as u16;
    let sp_max = crate::battle::engine::get_sp_max(0, 0, 1, 0) as u16;
    frames.push(stats(
        thuoctinh, hp_max, sp_max, 0, 0, 0, 0, 0, 0, 1, 6, 0, 0, 1, hp_max, sp_max, 0, 0, 0, 1, 0, 0, "",
    ));

    // 4. Step 4: SendPlayerOnline (self-appear for online pool, handled by broadcast)

    // 5. Step 5: Pet summary
    frames.push("F44402000F08".to_string());
    frames.push("F44402000F14".to_string());
    frames.push("F44402000F0A".to_string());

    // 6. Step 6: Party frames (none for new login)

    // 7. Step 7: Pet summon (none active)

    // 8. Step 8: Pet stat recompute (no packet)

    // 9. Step 9: PK / war state
    frames.push(format!("F44404002102{:02X}{:02X}", pk, tham_chien));

    // 10. Step 10: Inventory dumps (Homdo, TienTrang, Tuideo, LuuLang)
    frames.push("F44402001705".to_string());
    frames.push("F44402001E01".to_string());
    frames.push("F4440200172F".to_string());
    frames.push("F44402001766".to_string());

    // 11. Step 11: Equipped
    frames.push("F4440200170B".to_string());

    // 12. Step 12: Gold
    frames.push("F4440A001A04000000000000".to_string());

    // 13. Step 13: Server name ("TSVN")
    frames.push(server_name_frame(id, "TSVN"));

    // 14. Step 14: Terminator
    frames.push("F44402000504F44402000F0A".to_string());

    // 15. Step 15: Literal
    frames.push("F4440A000B0B0000000000002040".to_string());

    // 16. Step 16: Stable close
    frames.push("F44402001F0F".to_string());

    // 17. Step 17: Empty send (omitted)

    // 18. Step 18: Time banner
    frames.push(sys_msg_frame(&now_banner()));

    // 19. Step 19: Welcome banner
    frames.push(sys_msg_frame(
        "TS offline RebuildVN Thanks: Duong Van Truong && Somchai choosawai",
    ));

    // 20. Step 20: Hotbar
    frames.push("F4440300280102".to_string());

    // 21. Step 21: God / HP store / SP store (3x F44412002304...)
    let store_frame = format!("F444120023041027{}", "00".repeat(24));
    frames.push(store_frame.clone());
    frames.push(store_frame.clone());
    frames.push(store_frame);

    frames
}

/// Assemble a Logined1 sequence wrapper. `ok_all` carries the ordered frames
/// already built (self-appear, stats, pet summary, dumps, gold, name, banners,
/// hotbar, stores…); `ok` is returned as the outcome.
pub fn logined_sequence(ok: Vec<String>, _id: u32) -> HandleOutcome {
    let mut out = HandleOutcome::default();
    for f in ok {
        out.send(f);
    }
    out
}