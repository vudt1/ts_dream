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

    // Frame: F444 + len + 03 + body. The C# Logined1 emits a single opcode
    // byte `03` (no sub byte — Client.cs:8060), so len counts 1 + body_len,
    // i.e. `33 + equipped*2 + nameLen` (§2.4.1 step 2).
    crate::protocol::frame("03", &body)
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
        // C# Logined1 emits Tiengtam via `smethod_12` = LE32 (Client.cs:8061).
        encoder::le32(u32::from(tiengtam)),
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

/// Pet summary frames (C# `Data.SendStatusAllPet`, Logined1 step 5).
///
/// Emits **nothing** when the player owns no active pet (C# guards on
/// `text.Length > 0`). With pets (stt 1..4, id > 0) it builds the `0F08`
/// per-pet stat entries, the `0F14` slot summary, and the fixed stable-open
/// trailer — byte-for-byte the C# layout (Client.cs / Data.SendStatusAllPet).
pub fn pet_summary(s: &Session) -> Vec<String> {
    let active: Vec<&crate::server::session::PetState> = s
        .pets
        .iter()
        .filter(|p| (1..=4).contains(&p.stt) && p.id > 0)
        .collect();

    if active.is_empty() {
        return Vec::new();
    }

    let stats: String = active.iter().map(|p| pet_stat_entry(s, p)).collect();
    let slots: String = active.iter().map(|p| pet_slot_entry(p)).collect();

    vec![
        crate::protocol::frame("0F08", &stats),
        crate::protocol::frame("0F14", &slots),
        "F44402000F0A".to_string(),
        "F44405000F12010000".to_string(),
        "F44405000F12020000".to_string(),
        "F44405000F12030000".to_string(),
        "F44405000F12040000".to_string(),
        "F44404000F130100".to_string(),
    ]
}

/// Per-pet `0F08` stat body — the exact C# `Data.SendStatusPet` layout
/// (Data.cs:2270, mirrored by `Data.SendStatusAllPet`).
fn pet_stat_entry(s: &Session, p: &crate::server::session::PetState) -> String {
    let stt = p.stt;
    let lv_skill = [
        p.skills.first().map(|x| x.1).unwrap_or(0),
        p.skills.get(1).map(|x| x.1).unwrap_or(0),
        p.skills.get(2).map(|x| x.1).unwrap_or(0),
        p.skills.get(3).map(|x| x.1).unwrap_or(0),
    ];
    let mut stats = String::new();
    stats.push_str(&format!("{:02X}", stt));
    stats.push_str(&encoder::le32(u32::from(p.id)));
    stats.push_str(&encoder::le32(p.texp));
    stats.push_str(&format!("{:02X}", p.level));
    stats.push_str(&encoder::le16(p.hp));
    stats.push_str(&encoder::le16(p.sp));
    stats.push_str(&encoder::le16(p.int1));
    stats.push_str(&encoder::le16(p.atk));
    stats.push_str(&encoder::le16(p.def));
    stats.push_str(&encoder::le16(p.agi));
    stats.push_str(&encoder::le16(p.hpx));
    stats.push_str(&encoder::le16(p.spx));
    stats.push_str("00"); // reserved byte
    stats.push_str(&format!("{:02X}{:02X}", p.fai, p.quest));
    stats.push_str(&encoder::le16(p.skill_point));
    stats.push_str(&format!("{:02X}", p.name.len()));
    stats.push_str(&encoder::strhex(&p.name));
    stats.push_str(&format!("{:02X}{:02X}{:02X}", lv_skill[0], lv_skill[1], lv_skill[2]));
    for sub in 1..=6u16 {
        let slot = (stt as u16) * 10 + sub;
        let eq_id = s
            .trangbi
            .iter()
            .find(|i| u16::from(i.slot) == slot)
            .map(|i| i.id)
            .unwrap_or(0);
        stats.push_str(&encoder::le32(u32::from(eq_id)));
        stats.push_str("000000000000");
    }
    stats.push_str("00000000000000"); // 7 reserved bytes
    stats.push_str(&format!("{:02X}", lv_skill[3]));
    stats.push_str("00000000"); // 4 reserved bytes
    stats
}

/// Per-pet `0F14` slot entry.
fn pet_slot_entry(p: &crate::server::session::PetState) -> String {
    format!("{:02X}0000", p.stt)
}

/// Single-pet status + trailer (C# `Data.SendStatusPet`, Data.cs:2212-2278).
/// Returns an empty vector when the pet does not exist or has id 0.
pub fn pet_status_single(s: &Session, stt: u8) -> Vec<String> {
    let Some(p) = s.pets.iter().find(|p| p.stt == stt && p.id > 0) else {
        return Vec::new();
    };
    vec![
        crate::protocol::frame("0F08", &pet_stat_entry(s, p)),
        crate::protocol::frame("0F14", &pet_slot_entry(p)),
        "F44402000F0A".to_string(),
        "F44405000F12010000".to_string(),
        "F44405000F12020000".to_string(),
        "F44405000F12030000".to_string(),
        "F44405000F12040000".to_string(),
        "F44404000F130100".to_string(),
    ]
}

/// Build the full 22-step `Logined1` sequence frames for a logged-in session,
/// sourced from the session's actual state (props, stats, inventory, gold,
/// hotkeys, stores) so it matches a real C# `Logined1` byte-for-byte.
pub fn build_logined_sequence_session(s: &Session) -> Vec<String> {
    let mut frames = Vec::new();

    // 1. Step 1: end-talk + marker
    frames.extend(login_start());

    // 2. Step 2: player self-appear (op 0x03 sub 0x03)
    let color = if s.color.is_empty() {
        "0000000000000000"
    } else {
        &s.color
    };
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
        s.thuoctinh,
        s.hp,
        s.sp,
        s.int1,
        s.atk,
        s.def,
        s.agi,
        s.hpx,
        s.spx,
        s.level,
        s.texp,
        s.skill_point,
        s.point,
        s.tiengtam,
        s.hp_max,
        s.sp_max,
        s.atk2,
        s.def2,
        s.int2,
        s.agi2,
        s.hpx2,
        s.spx2,
        &skills_hex,
    ));

    // 4. Step 4: SendPlayerOnline — broadcast to the map, owned by the server loop.

    // 5. Step 5: Pet summary (0x0F08/0F14 + trailer) — only when pets exist
    //    (C# `SendStatusAllPet` sends nothing for a petless character).
    frames.extend(pet_summary(s));

    // 6. Step 6: Party frames (none for new login).

    // 7. Step 7: Pet summon (`F44406001301` + pet id) when one is active.
    if (1..=4).contains(&s.active_pet_stt) {
        if let Some(pet) = s.pets.iter().find(|p| p.stt == s.active_pet_stt) {
            frames.push(crate::protocol::frame(
                "1301",
                &encoder::le32(u32::from(pet.id)),
            ));
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
    fn pet_summary_empty_for_petless_session() {
        let s = Session::new();
        assert!(pet_summary(&s).is_empty(), "C# sends no pet frames with no pets");
    }

    #[test]
    fn pet_summary_builds_0f08_0f14_for_active_pet() {
        let mut s = Session::new();
        s.pets.push(crate::server::session::PetState {
            stt: 1,
            id: 15001,
            name: b"PET1".to_vec(),
            level: 2,
            hp: 100,
            sp: 50,
            int1: 10,
            atk: 20,
            def: 30,
            agi: 40,
            hpx: 50,
            spx: 60,
            fai: 70,
            texp: 1234,
            skill_point: 5,
            quest: 0,
            skills: [(10001, 1), (0, 0), (0, 0), (0, 0)],
            ..Default::default()
        });
        // Pet equipment lives in the Trangbi table at slot `stt*10+1` (11).
        s.trangbi.push(crate::server::session::InventoryItem {
            slot: 11,
            id: 20001,
            ..Default::default()
        });

        let frames = pet_summary(&s);
        assert_eq!(frames.len(), 8);
        // 0F14 slot summary: stt(01) + 0000 → 3 bytes → length 0x05.
        assert_eq!(frames[1], "F44405000F14010000");
        assert_eq!(frames[2], "F44402000F0A");
        assert_eq!(frames[3], "F44405000F12010000");
        assert_eq!(frames[4], "F44405000F12020000");
        assert_eq!(frames[5], "F44405000F12030000");
        assert_eq!(frames[6], "F44405000F12040000");
        assert_eq!(frames[7], "F44404000F130100");

        // 0F08: fixed prefix `00` + stt, id le32, texp le32.
        assert!(frames[0].starts_with("F444"));
        assert!(frames[0].contains("01993A0000D2040000"), "got {}", frames[0]);
        assert!(frames[0].contains("50455431"), "pet name PET1 embedded: {}", frames[0]);
        // Pet equipment slot 1 id (20001 = 0x4E21) + 6 zero bytes.
        assert!(frames[0].contains("214E0000"), "pet equip id: {}", frames[0]);
    }

    #[test]
    fn pet_summary_skips_stable_slots_and_id_zero() {
        let mut s = Session::new();
        // Stable slot (stt 7) and a zero-id pet must not appear.
        s.pets.push(crate::server::session::PetState {
            stt: 7,
            id: 15002,
            ..Default::default()
        });
        s.pets.push(crate::server::session::PetState {
            stt: 2,
            id: 0,
            ..Default::default()
        });
        assert!(pet_summary(&s).is_empty());
    }

    #[test]
    fn session_offline_frame_is_leave_hide_packet() {
        // Same literal as golden/13-battle-leave: F444 0800 0B00 + id + 0000.
        assert_eq!(session_offline_frame(300001), "F44408000B00E19304000000");
    }

    #[test]
    fn sys_msg_frame_never_emits_utf8() {
        // ASCII text is unchanged on the wire.
        assert_eq!(sys_msg_frame("TSVN"), "F4440A00020B000000005453564E");
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
