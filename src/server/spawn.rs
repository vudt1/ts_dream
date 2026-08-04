//! World-spawn (Logined1) and login packet builders (Chapter 2 §2.3/§2.4).
//!
//! Pure functions producing the exact byte stream; tested without a socket.
//! The 22-step `Logined1` sequence and the login gate responses live here.

use crate::protocol::encoder;
use crate::server::handler::HandleOutcome;

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