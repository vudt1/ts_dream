//! World-spawn (Logined1) and login packet builders (Chapter 2 §2.3/§2.4).
//!
//! Pure functions producing the exact byte stream; tested without a socket.
//! The 22-step `Logined1` sequence and the login gate responses live here.

use crate::protocol::encoder;
use crate::server::handler::HandleOutcome;
use crate::server::session::Session;
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
    crate::protocol::frame("0303", &body)
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

    crate::protocol::frame("0503", &body)
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
    let mut body = String::new();
    body.push_str(&encoder::le32(id));
    body.push_str(&format!("{:02X}", dir));
    body.push_str(&encoder::le16(x));
    body.push_str(&encoder::le16(y));
    crate::protocol::frame("0601", &body)
}

/// Build expression/action frame (op 0x20 sub 0x01 / 0x02).
pub fn expression_frame(id: u32, sub: u8, action: u8) -> String {
    let mut body = String::new();
    body.push_str(&format!("{:02X}", sub));
    body.push_str(&encoder::le32(id));
    body.push_str(&format!("{:02X}", action));
    crate::protocol::frame("20", &body)
}

/// Build chat packet frame (op 0x02 sub 0x01 / 0x02 / 0x03 / 0x05).
pub fn chat_frame(sub: u8, id: u32, chat_raw: &[u8]) -> String {
    let mut body = String::new();
    body.push_str(&encoder::le32(id));
    body.push_str(&encoder::hex(chat_raw));
    crate::protocol::frame(&format!("02{:02X}", sub), &body)
}

/// Shared banner builder for server-authored text (op 0x02 sub 0x0B/0x0C).
///
/// Routes the message through `smethod_17` (§4.4 item 3) so proper-Unicode
/// Vietnamese becomes single-byte VISCII on the wire (Đ→0xD0, not `'?'`), the
/// same path the C# server uses for banners and `/where` (Class5.smethod_17).
fn text_banner(op: &str, msg: &str) -> String {
    let visc = crate::encoding::viscii_encode(msg);
    let mut body = String::from("00000000");
    body.push_str(&encoder::strhex(&visc));
    crate::protocol::frame(op, &body)
}

/// Build system message banner packet (op 0x02 sub 0x0B).
pub fn sys_msg_frame(msg: &str) -> String {
    text_banner("020B", msg)
}

/// Build announcement packet (op 0x02 sub 0x0C).
pub fn announce_frame(msg: &str) -> String {
    text_banner("020C", msg)
}

/// Build server name packet (op 0x27 sub 0x09).
pub fn server_name_frame(id: u32, server_name: &str) -> String {
    let visc = crate::encoding::viscii_encode(server_name);
    let name_len = visc.len() as u8;
    let mut body = String::new();
    body.push_str(&encoder::le32(id));
    body.push_str("C4000000");
    body.push_str(&format!("{:02X}", name_len));
    body.push_str(&encoder::strhex(&visc));
    crate::protocol::frame("2709", &body)
}

/// World-hide broadcast frame emitted when a logged-in session disconnects
/// (Ch2 §2.1 / research 04 §7.4): `F44408000B00` + LE32(id) + `0000` removes
/// the entity from every peer's map on the wire.
pub fn session_offline_frame(id: u32) -> String {
    format!("F44408000B00{}0000", encoder::le32(id))
}

/// God / HP store / SP store frame (`method_0`): op 0x23 sub 0x04 + point + 12
/// zero bytes (C# `"F44412002304" + le32(point) + "000000000000000000000000"`).
pub fn store_frame(point: u32) -> String {
    let mut body = String::new();
    body.push_str(&encoder::le32(point));
    body.push_str(&"00".repeat(12));
    crate::protocol::frame("2304", &body)
}

/// Build the full 22-step `Logined1` sequence frames for a logged-in session,
/// sourced from the session's actual state (props, stats, inventory, gold,
/// hotkeys, stores) so it matches a real C# `Logined1` byte-for-byte.
pub fn build_logined_sequence_session(s: &Session) -> Vec<String> {
    let mut frames = Vec::new();

    // 1. Step 1: end-talk + marker
    frames.extend(login_start());

    // 2. Step 2: player self-appear (op 0x03 sub 0x03)
    let color = if s.color.is_empty() { "0000000000000000" } else { &s.color };
    frames.push(player_appear(
        s.id,
        s.sex,
        0,
        0,
        s.map_id,
        s.map_x,
        s.map_y,
        s.gocnhin,
        s.hair,
        color,
        &s.equipped_ids(),
        s.reborn,
        s.job,
        &s.name,
    ));

    // 3. Step 3: stats (op 0x05 sub 0x03)
    let skills_hex = skill_list(&s.skills);
    frames.push(stats(
        s.thuoctinh, s.hp, s.sp, s.int1, s.atk, s.def, s.agi, s.hpx, s.spx, s.level,
        s.texp, s.skill_point, s.point, s.tiengtam, s.hp_max, s.sp_max, s.atk2, s.def2,
        s.int2, s.agi2, s.hpx2, s.spx2, &skills_hex,
    ));

    // 4. Step 4: SendPlayerOnline — broadcast to the map, owned by the server loop.

    // 5. Step 5: Pet summary (0x0F08/0F14/0F0A).
    frames.push("F44402000F08".to_string());
    frames.push("F44402000F14".to_string());
    frames.push("F44402000F0A".to_string());

    // 6. Step 6: Party frames (none for new login).

    // 7. Step 7: Pet summon (`F44406001301` + pet id) when one is active.
    if (1..=4).contains(&s.active_pet_stt) {
        if let Some(pet) = s.pets.iter().find(|p| p.stt == s.active_pet_stt) {
            frames.push(crate::protocol::frame("1301", &encoder::le32(u32::from(pet.id))));
        }
    }

    // 8. Step 8: Pet stat recompute (no packet).

    // 9. Step 9: PK / war state
    frames.push(format!("F44404002102{:02X}{:02X}", s.pk, s.tham_chien));

    // 10. Step 10: Inventory dumps (Homdo, TienTrang, Tuideo, LuuLang)
    frames.push(s.dump_homdo());
    frames.push(s.dump_tientrang());
    frames.push(s.dump_tuideo());
    frames.push(s.dump_luulang());

    // 11. Step 11: Equipped
    frames.push(s.dump_trangbi());

    // 12. Step 12: Gold
    let mut gold_body = String::new();
    gold_body.push_str(&encoder::le32(s.gold));
    gold_body.push_str("00000000");
    frames.push(crate::protocol::frame("1A04", &gold_body));

    // 13. Step 13: Server name ("TSVN")
    frames.push(server_name_frame(s.id, "TSVN"));

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
    frames.push(s.dump_hotkeys());

    // 21. Step 21: God / HP store / SP store (3x)
    frames.push(store_frame(s.god));
    frames.push(store_frame(s.hp_store));
    frames.push(store_frame(s.sp_store));

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn session_offline_frame_is_leave_hide_packet() {
        // Same literal as golden/13-battle-leave: F444 0800 0B00 + id + 0000.
        assert_eq!(session_offline_frame(300001), "F44408000B00E19304000000");
    }

    #[test]
    fn sys_msg_frame_never_emits_utf8() {
        // ASCII text is unchanged on the wire.
        assert_eq!(
            sys_msg_frame("TSVN"),
            "F4440A00020B000000005453564E"
        );
        // Latin-1 accented é (U+00E9) travels as the single byte 0xE9 — never
        // as the two-byte UTF-8 pair C3 A9. The reverse map covers ≤0xFF.
        let f = sys_msg_frame("café");
        assert!(f.ends_with("636166E9"), "got {f}"); // 63 61 66 E9
        assert!(!f.contains("C3A9"), "got {f}");
        // Proper-Unicode Đ (U+0110) maps through smethod_17 to VISCII 0xD0;
        // ậ (U+1EAD) is 0xA7 in the positional table.
        let cfg = sys_msg_frame("Đậu2");
        assert!(cfg.ends_with("D0A77532"), "got {cfg}"); // Đ ậ u 2
        assert!(!cfg.contains("C490"), "got {cfg}");
    }

    #[test]
    fn server_name_frame_counts_viscii_bytes_not_utf8() {
        // "câu" = c(63) â(0xE2) u(75) → name_len = 3 VISCII bytes, hex 63 E2 75.
        let n = server_name_frame(1, "câu");
        assert!(n.ends_with("0363E275"), "got {n}");
        assert!(!n.contains("C3A2"), "got {n}");
    }
}