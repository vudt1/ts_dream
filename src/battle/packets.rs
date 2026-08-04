//! Battle packet builders (Chapter 6 §6.8).
//!
//! All packets are hex strings matching the C# output byte-for-byte.
//! Key frames use the `F444` magic + LE16 length encoding.

use crate::protocol::encoder;

/// Battle board open: `F4441C000BFA` + LE16(diahinh) + `03` + `_Packet` + trailer.
pub fn battle_open_leader(diahinh: u16, leader_packet: &str) -> String {
    format!(
        "F4441C000BFA{}03{}F44403000B0A01",
        encoder::le16(diahinh),
        leader_packet
    )
}

/// Battle open frame for member/NPC battles with arbitrary text.
pub fn battle_open_text(diahinh: u16, text: &str) -> String {
    let payload = format!("0BFA{}{}", encoder::le16(diahinh), text);
    let payload_len = payload.len() / 2;
    format!(
        "F444{}{}F44403000B0A01",
        encoder::le16(payload_len as u16),
        payload
    )
}

/// PK member frame: uses fixed 7000 instead of DiaHinh.
pub fn battle_open_pk_member(text: &str) -> String {
    let payload = format!("0BFA7000{}", text);
    let payload_len = payload.len() / 2;
    format!(
        "F444{}{}F44403000B0A01",
        encoder::le16(payload_len as u16),
        payload
    )
}

/// NPC entity in grid: `F4441A000B0503` + `_Packet`.
pub fn entity_npc(packet: &str) -> String {
    format!("F4441A000B0503{}", packet)
}

/// Player/pet entity in grid: `F4441A000B0505` + `_Packet`.
pub fn entity_player(packet: &str) -> String {
    format!("F4441A000B0505{}", packet)
}

/// Show entity on map (in battle): `F4440A000B0402` + LE32(id) + tail.
pub fn show_on_map(id: u32, tail: &str) -> String {
    format!("F4440A000B0402{}{}", encoder::le32(id), tail)
}

/// Show on map — standard player tail `000003`.
pub fn show_player_on_map(id: u32) -> String {
    show_on_map(id, "000003")
}

/// Show on map — member tail `000005`.
pub fn show_member_on_map(id: u32) -> String {
    show_on_map(id, "000005")
}

/// Show on map — PK leader tail `000002`.
pub fn show_pk_leader_on_map(id: u32) -> String {
    show_on_map(id, "000002")
}

/// Hide entity from map: `F44408000B00` + LE32(id) + `0000`.
pub fn hide_from_map(id: u32) -> String {
    format!("F44408000B00{}0000", encoder::le32(id))
}

/// Reposition on map: `F44405000B01` + row + col + `00`.
pub fn reposition(row: u8, col: u8) -> String {
    format!("F44405000B01{:02X}{:02X}00", row, col)
}

/// Clear pet cell: `F44404000B01` + (row^1) + col.
pub fn clear_pet_cell(row: u8, col: u8) -> String {
    format!("F44404000B01{:02X}{:02X}", row ^ 1, col)
}

/// Your turn prompt: `F44402003401`.
pub fn your_turn() -> String {
    "F44402003401".to_string()
}

/// Acting indicator: `F44404003505` + row + col.
pub fn acting(row: u8, col: u8) -> String {
    format!("F44404003505{:02X}{:02X}", row, col)
}

/// Turn action frame: `F444` + LE16(len) + `3201` + text9.
pub fn turn_action(text9: &str) -> String {
    let payload = format!("3201{}", text9);
    let payload_len = payload.len() / 2;
    format!("F444{}{}", encoder::le16(payload_len as u16), payload)
}

/// Combo packet: `F444130032010F00` + row + col + `264E0101` + row + col + `010301E0000000`.
pub fn combo(row: u8, col: u8) -> String {
    format!(
        "F444130032010F00{:02X}{:02X}264E0101{:02X}{:02X}010301E0000000",
        row, col, row, col
    )
}

/// Buff end: `F44407003501` + row + col + troi_end + `0000`.
pub fn troi_end(row: u8, col: u8, troi_end_byte: u8) -> String {
    format!(
        "F44407003501{:02X}{:02X}{:02X}0000",
        row, col, troi_end_byte
    )
}

/// Buff start on caster: `F44407003501` + row + col + `01` + LE16(skillId).
pub fn troi_start(row: u8, col: u8, skill_id: u16) -> String {
    format!(
        "F44407003501{:02X}{:02X}01{}",
        row,
        col,
        encoder::le16(skill_id)
    )
}

/// Drop reward: `F44408003504` + LE16(itemId) + npcRow + npcCol + row + col.
pub fn drop_item(item_id: u16, npc_row: u8, npc_col: u8, row: u8, col: u8) -> String {
    format!(
        "F44408003504{}{:02X}{:02X}{:02X}{:02X}",
        encoder::le16(item_id),
        npc_row,
        npc_col,
        row,
        col
    )
}

/// Status update: `F4440C000801` + status_byte + sign + LE32(abs_value) + `00000000`.
pub fn status_update(status_byte: u8, value: i32) -> String {
    let sign: u8 = if value >= 0 { 0x01 } else { 0x02 };
    format!(
        "F4440C000801{:02X}{:02X}{}00000000",
        status_byte,
        sign,
        encoder::le32(value.unsigned_abs())
    )
}

/// Battle start trailer: `F44403000B0A01`.
pub fn battle_trailer() -> String {
    "F44403000B0A01".to_string()
}

/// Battle exit UI: `F44402000504`.
pub fn battle_exit_move() -> String {
    "F44402000504".to_string()
}

/// Battle exit talk: `F44402001408`.
pub fn battle_exit_talk() -> String {
    "F44402001408".to_string()
}

/// Skilling effect (10 bytes):
/// row + col + miss_attack + atk_def_lantranh + count_hieuung + troi_hp_sp + LE16(damage) + buff_or_attack.
pub fn skilling_int(
    row: u8,
    col: u8,
    miss_attack: u8,
    atk_def_lantranh: u8,
    count: u8,
    troi_hp_sp: u8,
    damage: u16,
    buff_or_attack: u8,
) -> String {
    format!(
        "{:02X}{:02X}{:02X}{:02X}{:02X}{:02X}{}{:02X}",
        row,
        col,
        miss_attack,
        atk_def_lantranh,
        count,
        troi_hp_sp,
        encoder::le16(damage),
        buff_or_attack
    )
}

/// TroiBuffHpSp byte constants (DataStructure.cs:1481-1509).
pub mod troi_byte {
    pub const MISS: u8 = 0x00;
    pub const TYPE3: u8 = 0xDD; // 221
    pub const TYPE4: u8 = 0xDE; // 222
    pub const TYPE15: u8 = 0xDF; // 223
    pub const TYPE19: u8 = 0xE1; // 225
    pub const HP: u8 = 0x19; // 25
    pub const SP: u8 = 0x1A; // 26
    pub const HOCHU: u8 = 0x0E; // 14
}

/// TroiBuffEnd byte constants.
pub mod troi_end_byte {
    pub const TYPE3: u8 = 1;
    pub const TYPE4: u8 = 2;
    pub const TYPE15: u8 = 3;
    pub const TYPE19: u8 = 5;
}

/// Attack/Def/Lantranh byte constants.
pub mod attack_status {
    pub const ATTACK: u8 = 0;
    pub const DEF: u8 = 1;
    pub const LANTRANH: u8 = 2;
}

/// Miss/Attack byte constants.
pub mod miss_status {
    pub const ATTACK: u8 = 1;
    pub const MISS: u8 = 0;
}

/// Status bytes for stat push packets.
pub mod stat_byte {
    pub const HP: u8 = 0x19;
    pub const SP: u8 = 0x1A;
    pub const INT: u8 = 0x1B;
    pub const ATK: u8 = 0x1C;
    pub const DEF: u8 = 0x1D;
    pub const AGI: u8 = 0x1E;
    pub const HPX: u8 = 0x1F;
    pub const SPX: u8 = 0x20;
    pub const LV: u8 = 0x23;
    pub const TEXP: u8 = 0x24;
    pub const SKILL_POINT: u8 = 0x25;
    pub const POINT: u8 = 0x26;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn battle_open_leader_format() {
        let pkt = battle_open_leader(112, "02AABBCCDD");
        assert!(pkt.starts_with("F4441C000BFA"));
        assert!(pkt.ends_with("F44403000B0A01"));
    }

    #[test]
    fn entity_packet_format() {
        let npc = entity_npc("ABCDEF");
        assert_eq!(npc, "F4441A000B0503ABCDEF");

        let player = entity_player("123456");
        assert_eq!(player, "F4441A000B0505123456");
    }

    #[test]
    fn hide_and_show() {
        let hide = hide_from_map(300001);
        assert!(hide.starts_with("F44408000B00"));
        assert!(hide.ends_with("0000"));

        let show = show_player_on_map(300001);
        assert!(show.starts_with("F4440A000B0402"));
        assert!(show.ends_with("000003"));
    }

    #[test]
    fn your_turn_exact() {
        assert_eq!(your_turn(), "F44402003401");
    }

    #[test]
    fn acting_format() {
        assert_eq!(acting(3, 2), "F444040035050302");
    }

    #[test]
    fn status_update_positive() {
        let s = status_update(stat_byte::HP, 100);
        assert!(s.starts_with("F4440C00080119"));
        assert!(s.contains("01")); // positive sign
    }

    #[test]
    fn skilling_int_format() {
        let eff = skilling_int(0, 2, 1, 0, 1, troi_byte::HP, 50, 1);
        assert_eq!(eff.len(), 18); // 9 bytes = 18 hex chars (row col miss adl count troi LE16 buff)
    }
}
