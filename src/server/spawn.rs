//! World-spawn (Logined1) and login packet builders (Chapter 2 §2.3/§2.4).
//!
//! Pure functions producing the exact byte stream; tested without a socket.
//! The 22-step `Logined1` sequence and the login gate responses live here.

use crate::protocol::encoder;
use crate::server::dispatcher::HandleOutcome;
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
/// No character on this account → create-char screen (Bear `Authentication`
/// case 3: `[01][03][00]`).
pub const LOGIN_CREATE_CHAR: &str = "F4440300010300";
/// Alias of [`LOGIN_CREATE_CHAR`] for the op-0x03 enter-game path (same wire
/// bytes; kept as a separate name so call sites read correctly).
pub const ENTER_GAME_CREATE: &str = LOGIN_CREATE_CHAR;
/// Double login: account already online elsewhere (Bear `Authentication`
/// case 2: `[00][19]`). Identical to `SystemAlertReason::DuplicateLoginOtherLocation`
/// (`F44402000013`); sent before disconnecting so the client shows the dialog
/// instead of hanging.
pub const DOUBLE_LOGIN: &str = "F44402000013";
/// Server Greeting Packet gửi ngay khi Client kết nối TCP thành công.
/// Op: 0x01 (OP_AUTH), Sub: 0x09 (Scene Switch), SceneMode: 0x5A (90 - Login Scene).
/// Payload 3 bytes: 01 09 5A -> Frame: F444030001095A.
pub const LOGIN_SCENE_GREETING: &str = "F444030001095A";

/// Character creation responses (op 0x09).
/// Op 0x09 Sub 0x03 ss 0x00 — Tên nhân vật hợp lệ và khả dụng (kích hoạt state-machine client).
pub const CHAR_NAME_AVAILABLE: &str = "F4440300090300";
/// Op 0x09 Sub 0x03 ss 0x01 — Tên nhân vật đã bị trùng lặp (toast: "Tên bị trùng lập, hãy lập lại tên mới").
pub const CHAR_NAME_DUPLICATE: &str = "F4440300090301";
/// Op 0x09 Sub 0x03 ss 0x02 — Tên nhân vật không hợp lệ (toast: "Tên không hợp lệ, hãy lấy tên khác").
pub const CHAR_NAME_INVALID: &str = "F4440300090302";
/// Op 0x09 Sub 0x01 — Tạo nhân vật thành công (client nhận được sẽ gửi Op 0x03 Sub 0x01 để vào game).
pub const CHAR_CREATE_SUCCESS: &str = "F44402000901";

/// Step 1 of Logined: end-talk + the `F4440300142100` marker.
pub fn login_start() -> Vec<String> {
    vec!["F44402001408".to_string(), "F4440300142100".to_string()]
}

/// Player self-appear frame — op 0x03 sub 0x03 (Logined  step 2).
// Positional mirror of the fixed frame layout: each argument maps to one
// field of the wire body, so the long parameter list is intentional.
#[allow(clippy::too_many_arguments)]
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

    // Frame: F444 + len + 03 + body. A single opcode byte `03` is emitted
    // (no sub byte), so len counts 1 + body_len,
    // i.e. `33 + equipped*2 + nameLen` (§2.4.1 step 2).
    crate::protocol::frame("03", &body)
}

/// Stats frame — op 0x05 sub 0x03 (Logined  step 3). `skills_hex` = the
/// player's skill list hex (`««SKILL_ID Lv` pairs are handled by the caller
/// via [`skill_list`]). Length counts `skills/2 + 113` bytes.
// Positional mirror of the fixed frame layout: each argument maps to one
// field of the wire body, so the long parameter list is intentional.
#[allow(clippy::too_many_arguments)]
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
        // Tiengtam travels as LE32.
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
/// Encodes proper-Unicode Vietnamese as single-byte VISCII on the wire
/// (Đ→0xD0, not `'?'`) — the same treatment banners and `/where` replies get.
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

/// Build server name packet (op 0x27/OP_RANK_ANNOUNCE sub 0x09).
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

/// God / HP store / SP store frame: op 0x23/OP_ACCOUNT sub 0x04 + point + 12 zero bytes
/// (`F44412002304 + le32(point) + "00"×12`).
pub fn store_frame(point: u32) -> String {
    let mut body = String::new();
    body.push_str(&encoder::le32(point));
    body.push_str(&"00".repeat(12));
    crate::protocol::frame("2304", &body)
}

/// Client-verified S→C notice frames (aLogin bus semantics, see
/// `.scratch/client-pseudo-op-code/opcode_17.md` §3–§4 and `opcode_0b.md`
/// §2–§3). These carry only static client-side text/effects, so they are
/// safe for aLogin; Bear-dialect clients simply ignore unknown subs.

/// Item pickup failure toast (`[17][19]` — client shows
/// "Vật phẩm này tạm thời không thể nhặt lên", 1200ms).
pub const ITEM_PICKUP_FAIL: &str = "F44402001719";
/// Trade-cancelled toast (`[17][3B]` — client shows "Hủy bỏ giao dịch",
/// 2000ms).
pub const TRADE_CANCELLED: &str = "F4440200173B";

/// Battle flag write (`[0B][09][A][B]`): client stores `A` at
/// `PlayerRec+0x1308`; when `B == 1` it also toasts the number 2000ms.
/// Minimum 4B payload — never send short.
pub fn battle_flag_frame(a: u8, b: u8) -> String {
    format!("F44404000B09{a:02X}{b:02X}")
}

/// Battle toast (`[0B][03][code]`, code 1..4 → 1000ms toast; other codes
/// no-op on the client).
pub fn battle_toast_frame(code: u8) -> String {
    format!("F44403000B03{code:02X}")
}

/// Battle mode flag + saturating counter (`[0B][07][B1][B2]`): client sets
/// `self+0x1305 = B1`, adds B2 to `self+0x1306` (saturates at 0xFF), and
/// toasts when B1 is 1/2. Minimum 4B payload — never send short.
pub fn battle_counter_frame(b1: u8, b2: u8) -> String {
    format!("F44405000B07{b1:02X}{b2:02X}")
}

/// Pet summary frames (Logined1 step 5).
///
/// Emits **nothing** when the player owns no active pet (guards on an empty
/// active list). With pets (stt 1..4, id > 0) it builds the `0F08`
/// per-pet stat entries, the `0F14` slot summary, and the fixed stable-open
/// trailer — byte-for-byte the fixed legacy layout.
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

/// Per-pet `0F08` stat body — the fixed legacy layout shared by
/// [`pet_summary`] and [`pet_status_single`].
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
    stats.push_str(&format!(
        "{:02X}{:02X}{:02X}",
        lv_skill[0], lv_skill[1], lv_skill[2]
    ));
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

/// Single-pet status + trailer. Returns an empty vector when the pet does not
/// exist or has id 0.
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
/// hotkeys, stores) so it reproduces the canonical Logined1 byte stream.
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
    //    (a petless character receives nothing).
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
