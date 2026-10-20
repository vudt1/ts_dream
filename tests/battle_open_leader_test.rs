//! Ticket 13 — `battle_open_leader` computes its wire length from the payload
//! instead of hardcoding `F4441C`, and the byte output stays identical.
//!
//! `0B FA` layout (`01_battle_start_scene.md §2.1`): after the `F444` magic +
//! LE16 length, the payload is `0BFA` + `le16(diahinh)` + `04` (leader `kind`
//! marker, ticket 15) + the 23-byte `_Packet` snapshot, then the `0B 0A 01`
//! trailer.

use ts_dream::battle::packets;
use ts_dream::protocol::encoder::le16;

/// A well-formed 23-byte (`_Packet`) snapshot in hex (46 chars).
const LEADER_PACKET: &str = "020102030405060708090A0B0C0D0E0F10111213141516";

#[test]
fn battle_open_leader_is_byte_identical_to_hardcoded_form() {
    let diahinh: u16 = 0x1234;
    let frame = packets::battle_open_leader(diahinh, LEADER_PACKET);

    // Old hardcoded form (length literal `1C`, i.e. 28 bytes); the self record
    // marker is `04` (ticket 15).
    let expected = format!(
        "F4441C000BFA{}04{}F44403000B0A01",
        le16(diahinh),
        LEADER_PACKET
    );
    assert_eq!(frame, expected);

    // Sanity: payload = 0BFA(2) + diahinh(2) + 04(1) + record(23) = 28 bytes.
    assert_eq!(&frame[4..8], "1C00");
}

#[test]
fn battle_open_leader_length_tracks_payload() {
    let diahinh: u16 = 1;
    let frame = packets::battle_open_leader(diahinh, LEADER_PACKET);

    let payload = format!("0BFA{}04{}", le16(diahinh), LEADER_PACKET);
    let declared = u16::from_le_bytes([
        u8::from_str_radix(&frame[4..6], 16).unwrap(),
        u8::from_str_radix(&frame[6..8], 16).unwrap(),
    ]);
    assert_eq!(declared as usize, payload.len() / 2);
}

#[test]
fn battle_open_pk_member_keeps_intentional_7000_magic() {
    let frame = packets::battle_open_pk_member("AA");
    // payload = 0BFA(2) + 7000(2) + AA(1) = 5 bytes.
    assert_eq!(frame, format!("F444{}0BFA7000AAF44403000B0A01", le16(5u16)));
}
