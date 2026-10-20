//! Ticket 15 — the **self** record marker in every `0B FA` frame is `04`.
//!
//! The client writes the record's `kind` byte straight into `player+0x376`
//! (`006438ac:262`). Only `0x376 == 4` unlocks the "leave battle" button
//! (`00590ba8:159`) and the `0x376 = 0` reset inside `0B 00`
//! (`00641fa4:69-76`). Sending `03` (leader) / `05` (member) traps the client
//! in the battle. Non-self records keep their entity-type markers (`03` for
//! the leader + leader pet, `64` for other members) because the client never
//! reads them into `+0x376`.
//!
//! Frame layout (`HANDOFF-client-reverse.md §2.1`): after `F444` + LE16 length,
//! the payload is `0BFA` + `le16(diahinh)` + 24-byte records, each
//! `[kind][23-byte _Packet]`, followed by the `F44403000B0A01` trailer.

use ts_dream::battle::construction::{Battle, StartPacket};
use ts_dream::battle::packets;
use ts_dream::server::session::Session;

/// One `0B FA` record is 24 bytes: 1 kind byte + a 23-byte `_Packet` snapshot.
const RECORD_BYTES: usize = 24;
/// Hex chars per record.
const RECORD_HEX: usize = RECORD_BYTES * 2;

/// A well-formed 23-byte (`_Packet`) snapshot in hex (46 chars).
const LEADER_PACKET: &str = "020102030405060708090A0B0C0D0E0F10111213141516";

/// Parse a `0B FA` frame into `(declared_payload_len, self_kind, record_kinds)`.
///
/// `record_kinds[0]` is the self record.
fn parse_open_frame(frame: &str) -> (usize, u8, Vec<u8>) {
    assert!(frame.starts_with("F444"), "missing magic: {frame}");
    let lo = u8::from_str_radix(&frame[4..6], 16).unwrap() as usize;
    let hi = u8::from_str_radix(&frame[6..8], 16).unwrap() as usize;
    let declared = lo | (hi << 8);

    let payload = &frame[8..8 + declared * 2];
    assert!(payload.starts_with("0BFA"), "missing 0B FA: {payload}");
    // Skip `0BFA` + `diahinh` (2 bytes each) to reach the first record.
    let records = &payload[8..];
    assert_eq!(
        records.len() % RECORD_HEX,
        0,
        "payload must be a whole number of 24-byte records"
    );

    let kinds: Vec<u8> = records
        .as_bytes()
        .chunks(RECORD_HEX)
        .map(|r| u8::from_str_radix(std::str::from_utf8(&r[..2]).unwrap(), 16).unwrap())
        .collect();

    let self_kind = *kinds.first().expect("at least the self record");
    (declared, self_kind, kinds)
}

fn session_with_id(id: u32) -> Session {
    let mut s = Session::new();
    s.id = id;
    s
}

/// Leader view: the leader's own record is `04`; length stays `0x1C` (28 bytes).
#[test]
fn leader_frame_self_record_is_04_and_length_unchanged() {
    let frame = packets::battle_open_leader(0x1234, LEADER_PACKET);
    let (declared, self_kind, kinds) = parse_open_frame(&frame);

    assert_eq!(self_kind, 0x04, "leader self record must be kind 04");
    assert_eq!(kinds, vec![0x04]);
    // 0BFA(2) + diahinh(2) + 04(1) + record(23) = 28 bytes = 0x1C.
    assert_eq!(declared, 0x1C);
    assert_eq!(&frame[4..8], "1C00", "declared frame length must not change");
}

/// Member view: the member's own record is `04`, the remaining records keep
/// their non-self entity markers (`03` leader / leader pet, `64` other member
/// + pet), and the declared length matches the real payload byte count.
#[test]
fn member_frame_self_record_is_04_and_others_keep_their_markers() {
    let mut battle = Battle::new(1, 0x1234);
    battle.add_player(&session_with_id(1000), 0, 3, 2); // leader (3,2)
    battle.add_player(&session_with_id(2000), 0, 3, 1); // self member (3,1)
    battle.add_player(&session_with_id(3000), 0, 2, 2); // leader pet cell (2,2)
    battle.add_player(&session_with_id(4000), 0, 3, 3); // other member (3,3)
    battle.add_player(&session_with_id(5000), 0, 2, 3); // other member pet (2,3)

    let frames = battle.member_battle_frame(0x1234, 2000);
    let frame = frames
        .iter()
        .find_map(|p| match p {
            StartPacket::To { frame, .. } if frame.contains("0BFA") => Some(frame.clone()),
            _ => None,
        })
        .expect("member must receive a 0B FA frame");

    let (declared, self_kind, kinds) = parse_open_frame(&frame);

    // 5 records: self(04), leader(03), leader pet(03), other member(64),
    // other member pet(64).
    assert_eq!(kinds, vec![0x04, 0x03, 0x03, 0x64, 0x64]);
    assert_eq!(self_kind, 0x04, "member self record must be kind 04");

    // Declared length = 4 (0B FA + diahinh) + records, in bytes.
    assert_eq!(declared, 4 + 5 * RECORD_BYTES);
    assert!(frame.ends_with("F44403000B0A01"));
}

/// Both views use the same self marker, so the marker flip alone cannot change
/// a frame's length (the record count and record size are untouched).
#[test]
fn self_kind_is_04_on_both_frames_and_length_only_tracks_records() {
    let leader = packets::battle_open_leader(1, LEADER_PACKET);
    let (leader_len, leader_kind, _) = parse_open_frame(&leader);
    assert_eq!(leader_kind, 0x04);

    let mut battle = Battle::new(1, 1);
    battle.add_player(&session_with_id(1), 0, 3, 1);
    battle.add_player(&session_with_id(2), 0, 3, 2);
    let member = battle
        .member_battle_frame(1, 1)
        .into_iter()
        .find_map(|p| match p {
            StartPacket::To { frame, .. } if frame.contains("0BFA") => Some(frame),
            _ => None,
        })
        .expect("member 0B FA frame");
    let (member_len, member_kind, _) = parse_open_frame(&member);
    assert_eq!(member_kind, 0x04);

    // Same byte count regardless of the marker value `04`: the length field is
    // driven only by the record count / size.
    assert_eq!(leader_len, 4 + RECORD_BYTES);
    // Member text = self(04) + leader(03) = 2 records.
    assert_eq!(member_len, 4 + 2 * RECORD_BYTES);
}
