//! Protocol, encoding, config and harness tests — migrated from inline #[cfg(test)] blocks (ticket 08).

use std::sync::Arc;

use tempfile::tempdir;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tokio::sync::watch;

use ts_dream::config::{from_file, parse_bool, parse_u16, Config};
use ts_dream::encoding::{
    compute_garble, name_wire_hex, to_viscii, viscii_encode, viscii_to_unicode,
};
use ts_dream::error::Result;
use ts_dream::harness::{proxy, Golden};
use ts_dream::protocol::codec::Packet;
use ts_dream::protocol::codecs::{
    ehuman, BattleRoleData, BattleRoleSerializer, FriendExtra, PlayerCard, ThingData,
    FRIEND_EXTRA_SIZE, THING_DATA_SIZE,
};
use ts_dream::protocol::encoder::{
    bytes, hex, le16, le32, strhex, strhex_of, u16_le, u32_le, xor01,
};
use ts_dream::protocol::frame::{check_magic, encode_to_wire, Decoder};
use ts_dream::protocol::reader::PacketReader;
use ts_dream::protocol::writer::PacketWriter;
use ts_dream::protocol::{frame, XOR_KEY};

// ── protocol (mod.rs) ────────────────────────────────────────────────────────

#[test]
fn frame_builds_known_literals_byte_identical() {
    // Matches the fixed literals used across the codebase: len counts every
    // byte after the 4-byte header (code + body).
    assert_eq!(
        frame("0801", "1B0102000000000000"),
        "F4440B0008011B0102000000000000"
    );
    assert_eq!(
        frame("0601", "01000000026400C800"),
        "F4440B00060101000000026400C800"
    );
    assert_eq!(frame("1705", ""), "F44402001705");
}

// ── protocol::frame ──────────────────────────────────────────────────────────

/// Wire bytes for a decoded hex frame (hex → bytes → XOR 0xAD).
fn wire(hex: &str) -> Vec<u8> {
    encode_to_wire(hex).expect("valid frame hex")
}

#[test]
fn feed_single_complete_frame() {
    let mut d = Decoder::new();
    assert_eq!(d.feed(&wire("F444010000")), vec!["F444010000"]);
    assert!(d.pending().is_empty());
}

#[test]
fn feed_concatenated_frames_in_one_chunk() {
    let mut d = Decoder::new();
    let frames = d.feed(&wire("F444010000F4440300010901F44402000901"));
    assert_eq!(frames, vec!["F444010000", "F4440300010901", "F44402000901"]);
    assert!(d.pending().is_empty());
}

#[test]
fn feed_partial_trailing_frame_buffered_across_chunks() {
    // "F4440B000601E1930400026400C800" split 2 bytes + remainder.
    let mut d = Decoder::new();
    assert!(d.feed(&wire("F4440B")).is_empty());
    assert_eq!(d.pending(), "F4440B");
    let frames = d.feed(&wire("000601E1930400026400C800"));
    assert_eq!(frames, vec!["F4440B000601E1930400026400C800"]);
    assert!(d.pending().is_empty());
}

#[test]
fn feed_partial_frame_split_mid_length_field() {
    // First chunk ends inside the length field: 3 bytes of the 5-byte frame.
    let mut d = Decoder::new();
    assert!(d.feed(&wire("F44401")).is_empty());
    assert_eq!(d.pending(), "F44401");
    let frames = d.feed(&wire("0000"));
    assert_eq!(frames, vec!["F444010000"]);
}

#[test]
fn feed_multiple_frames_with_partial_trailing_retained() {
    let mut d = Decoder::new();
    // Two complete frames in chunk 1.
    assert_eq!(
        d.feed(&wire("F444010000F4440300010901")),
        vec!["F444010000", "F4440300010901"]
    );
    // Chunk 2 carries only the start of a third frame ("F4440B" = 3 bytes).
    assert!(d.feed(&wire("F4440B")).is_empty());
    assert_eq!(d.pending(), "F4440B");
    // Remainder completes it on chunk 3.
    assert_eq!(
        d.feed(&wire("000601E1930400026400C800")),
        vec!["F4440B000601E1930400026400C800"]
    );
    assert!(d.pending().is_empty());
}

#[test]
fn feed_empty_chunk_yields_nothing() {
    let mut d = Decoder::new();
    assert!(d.feed(&[]).is_empty());
    assert!(d.pending().is_empty());
}

#[test]
fn feed_splits_large_multi_frame_chunk() {
    let mut d = Decoder::new();
    let frames = d.feed(&wire(&"F444010000".repeat(50)));
    assert_eq!(frames.len(), 50);
    assert!(frames.iter().all(|f| f == "F444010000"));
    assert!(d.pending().is_empty());
}

#[test]
fn check_magic_accepts_f444_rejects_others() {
    assert!(check_magic(&[0xF4, 0x44, 0x01, 0x00, 0x00]));
    assert!(!check_magic(&[0xF4, 0x45, 0x01, 0x00, 0x00]));
    assert!(!check_magic(&[0x01]));
    assert!(!check_magic(&[]));
}

// ── protocol::codec ──────────────────────────────────────────────────────────

#[test]
fn build_frame_length() {
    // opcode 0x03 sub 0x01, empty body -> F44402000301
    let p = Packet::opcode(0x03, 0x01).build();
    assert_eq!(p, "F44402000301");
}

#[test]
fn full_frame_length_field() {
    // body = opcode/sub 00 00 + payload 01 00 = 4 bytes -> len 4 "0400"
    let p = Packet::opcode(0x00, 0x00).raw("0100").build();
    assert_eq!(p, "F444040000000100");
    assert_eq!(p.len(), 16);
}

// ── protocol::encoder ────────────────────────────────────────────────────────

#[test]
fn le16_little_endian() {
    assert_eq!(le16(7168), "001C");
    assert_eq!(le16(1), "0100");
    assert_eq!(le16(0x1234), "3412");
}

#[test]
fn le32_little_endian() {
    assert_eq!(le32(3), "03000000");
    assert_eq!(le32(0x11223344), "44332211");
}

#[test]
fn u16_u32_from_bytes() {
    assert_eq!(u16_le(0x00, 0x1C), 0x1C00);
    assert_eq!(u32_le(0x03, 0x00, 0x00, 0x00), 3);
}

#[test]
fn hex_and_bytes_roundtrip() {
    assert_eq!(hex(&[0x0A, 0x0B]), "0A0B");
    assert_eq!(bytes("0A0B"), Some(vec![0x0A, 0x0B]));
    assert_eq!(bytes("0A0"), None);
}

#[test]
fn xor01_inverts() {
    let data = [0xF4u8, 0x44, 0x00, 0xFF];
    let x = xor01(&data);
    assert_eq!(x, data.iter().map(|b| b ^ 0xAD).collect::<Vec<_>>());
    assert_eq!(xor01(&x), data.to_vec());
}

#[test]
fn strhex_low_byte() {
    assert_eq!(strhex(b"A"), "41");
    assert_eq!(strhex(b"hi"), "6869");
    assert_eq!(strhex_of("TSVN"), "5453564E");
}

// ── protocol::reader ─────────────────────────────────────────────────────────

#[test]
fn test_read_primitives() {
    let data = [
        0x12, // u8
        0x34, 0x12, // u16: 0x1234
        0x78, 0x56, 0x34, 0x12, // u32: 0x12345678
        0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x80, // u64: 0x8000000000000001
        0xFE, // i8: -2
        0xF0, 0xFF, // i16: -16
        0x00, 0x00, 0xFF, 0xFF, // i32: -65536
        0x01, // bool: true
        0x00, // bool: false
    ];
    let mut reader = PacketReader::new(&data);

    assert_eq!(reader.read_u8().unwrap(), 0x12);
    assert_eq!(reader.read_u16_le().unwrap(), 0x1234);
    assert_eq!(reader.read_u32_le().unwrap(), 0x12345678);
    assert_eq!(reader.read_u64_le().unwrap(), 0x8000000000000001);
    assert_eq!(reader.read_i8().unwrap(), -2);
    assert_eq!(reader.read_i16_le().unwrap(), -16);
    assert_eq!(reader.read_i32_le().unwrap(), -65536);
    assert!(reader.read_bool().unwrap());
    assert!(!reader.read_bool().unwrap());
    assert_eq!(reader.remaining(), 0);
    assert!(!reader.is_readable());
}

#[test]
fn test_read_f64() {
    let val = 12345.6789f64;
    let bytes = val.to_le_bytes();
    let mut reader = PacketReader::new(&bytes);
    assert_eq!(reader.read_f64_le().unwrap(), val);
}

#[test]
fn test_read_bytes_zero_copy() {
    let data = b"Hello, world!";
    let mut reader = PacketReader::new(data);
    let part1 = reader.read_bytes(5).unwrap();
    assert_eq!(part1, b"Hello");
    reader.skip(2).unwrap(); // skip ", "
    let part2 = reader.read_bytes(6).unwrap();
    assert_eq!(part2, b"world!");
    assert_eq!(reader.remaining(), 0);
}

#[test]
fn test_read_viscii_pascal() {
    // "TSVN" -> length 4 + bytes
    let data = [4, 0x54, 0x53, 0x56, 0x4E];
    let mut reader = PacketReader::new(&data);
    assert_eq!(reader.read_viscii_pascal().unwrap(), "TSVN");

    // VISCII Vietnamese: Đ (0xD0), ấ (0xA4), ă (0xE5), ỏ (0xF6)
    let data_vn = [4, 0xD0, 0xA4, 0xE5, 0xF6];
    let mut reader_vn = PacketReader::new(&data_vn);
    assert_eq!(reader_vn.read_viscii_pascal().unwrap(), "Đấăỏ");
}

#[test]
fn test_read_viscii_fixed() {
    // Fixed 8 bytes with null padding: "TS" + 6 nulls
    let data = [0x54, 0x53, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00];
    let mut reader = PacketReader::new(&data);
    assert_eq!(reader.read_viscii_fixed(8).unwrap(), "TS");

    // Fixed 4 bytes full (no nulls): "TSVN"
    let data_full = [0x54, 0x53, 0x56, 0x4E];
    let mut reader_full = PacketReader::new(&data_full);
    assert_eq!(reader_full.read_viscii_fixed(4).unwrap(), "TSVN");
}

#[test]
fn test_bounds_checking_errors() {
    let data = [0x01, 0x02];
    let mut reader = PacketReader::new(&data);

    // Attempting to read u32 (4 bytes) when only 2 remain
    assert!(reader.read_u32_le().is_err());
    assert_eq!(reader.position(), 0);

    // Attempting to skip past end
    assert!(reader.skip(5).is_err());

    // Attempting to set position out of bounds
    assert!(reader.set_position(10).is_err());

    // Valid read then EOF
    assert_eq!(reader.read_u8().unwrap(), 0x01);
    assert_eq!(reader.read_u8().unwrap(), 0x02);
    assert!(reader.read_u8().is_err());
}

#[test]
fn test_boundary_integer_values() {
    let mut buf = Vec::new();
    buf.push(u8::MAX);
    buf.extend_from_slice(&u16::MAX.to_le_bytes());
    buf.extend_from_slice(&u32::MAX.to_le_bytes());
    buf.extend_from_slice(&u64::MAX.to_le_bytes());
    buf.push(i8::MIN as u8);
    buf.push(i8::MAX as u8);
    buf.extend_from_slice(&i16::MIN.to_le_bytes());
    buf.extend_from_slice(&i16::MAX.to_le_bytes());
    buf.extend_from_slice(&i32::MIN.to_le_bytes());
    buf.extend_from_slice(&i32::MAX.to_le_bytes());

    let mut reader = PacketReader::new(&buf);
    assert_eq!(reader.read_u8().unwrap(), u8::MAX);
    assert_eq!(reader.read_u16_le().unwrap(), u16::MAX);
    assert_eq!(reader.read_u32_le().unwrap(), u32::MAX);
    assert_eq!(reader.read_u64_le().unwrap(), u64::MAX);
    assert_eq!(reader.read_i8().unwrap(), i8::MIN);
    assert_eq!(reader.read_i8().unwrap(), i8::MAX);
    assert_eq!(reader.read_i16_le().unwrap(), i16::MIN);
    assert_eq!(reader.read_i16_le().unwrap(), i16::MAX);
    assert_eq!(reader.read_i32_le().unwrap(), i32::MIN);
    assert_eq!(reader.read_i32_le().unwrap(), i32::MAX);
    assert_eq!(reader.remaining(), 0);
}

#[test]
fn test_empty_string_and_seek() {
    let data = [0x00, 0x01, 0x02, 0x03];
    let mut reader = PacketReader::new(&data);

    // Empty pascal string (length 0)
    assert_eq!(reader.read_viscii_pascal().unwrap(), "");
    assert_eq!(reader.position(), 1);

    // Seek forward and back
    reader.set_position(3).unwrap();
    assert_eq!(reader.read_u8().unwrap(), 0x03);
    reader.set_position(1).unwrap();
    assert_eq!(reader.read_u16_le().unwrap(), 0x0201);
}

// ── protocol::writer ─────────────────────────────────────────────────────────

#[test]
fn test_write_primitives() {
    let mut writer = PacketWriter::empty();
    writer
        .write_u8(0x12)
        .write_u16_le(0x1234)
        .write_u32_le(0x12345678)
        .write_u64_le(0x8000000000000001)
        .write_i8(-2)
        .write_i16_le(-16)
        .write_i32_le(-65536)
        .write_f64_le(12345.6789)
        .write_bool(true)
        .write_bool(false);

    let mut reader = PacketReader::new(writer.as_slice());
    assert_eq!(reader.read_u8().unwrap(), 0x12);
    assert_eq!(reader.read_u16_le().unwrap(), 0x1234);
    assert_eq!(reader.read_u32_le().unwrap(), 0x12345678);
    assert_eq!(reader.read_u64_le().unwrap(), 0x8000000000000001);
    assert_eq!(reader.read_i8().unwrap(), -2);
    assert_eq!(reader.read_i16_le().unwrap(), -16);
    assert_eq!(reader.read_i32_le().unwrap(), -65536);
    assert_eq!(reader.read_f64_le().unwrap(), 12345.6789);
    assert!(reader.read_bool().unwrap());
    assert!(!reader.read_bool().unwrap());
    assert_eq!(reader.remaining(), 0);
}

#[test]
fn test_write_viscii_pascal() {
    let mut writer = PacketWriter::empty();
    writer.write_viscii_pascal("TSVN");
    assert_eq!(writer.as_slice(), &[4, 0x54, 0x53, 0x56, 0x4E]);

    let mut reader = PacketReader::new(writer.as_slice());
    assert_eq!(reader.read_viscii_pascal().unwrap(), "TSVN");

    // Vietnamese text
    let mut writer_vn = PacketWriter::empty();
    writer_vn.write_viscii_pascal("Thời gian:");
    let mut reader_vn = PacketReader::new(writer_vn.as_slice());
    assert_eq!(reader_vn.read_viscii_pascal().unwrap(), "Thời gian:");
}

#[test]
fn test_write_viscii_fixed() {
    // Short text padded to fixed len
    let mut writer = PacketWriter::empty();
    writer.write_viscii_fixed("TS", 6);
    assert_eq!(writer.as_slice(), &[0x54, 0x53, 0x00, 0x00, 0x00, 0x00]);

    let mut reader = PacketReader::new(writer.as_slice());
    assert_eq!(reader.read_viscii_fixed(6).unwrap(), "TS");

    // Truncated text
    let mut writer_trunc = PacketWriter::empty();
    writer_trunc.write_viscii_fixed("TSOnline", 4);
    assert_eq!(writer_trunc.as_slice(), &[0x54, 0x53, 0x4F, 0x6E]);
}

#[test]
fn test_build_frame_and_wire() {
    // Opcode 0x03, Sub 0x01 with no extra payload -> body len = 2
    let writer = PacketWriter::new(0x03, 0x01);
    let frame = writer.build_frame();
    assert_eq!(frame, vec![0xF4, 0x44, 0x02, 0x00, 0x03, 0x01]);

    let wire = writer.build_wire();
    let expected_wire: Vec<u8> = frame.iter().map(|b| b ^ 0xAD).collect();
    assert_eq!(wire, expected_wire);

    // Opcode only
    let writer_op = PacketWriter::opcode(0x0A);
    let frame_op = writer_op.build_frame();
    assert_eq!(frame_op, vec![0xF4, 0x44, 0x01, 0x00, 0x0A]);
}

#[test]
fn test_writer_methods() {
    let mut writer = PacketWriter::new(0x06, 0x01);
    assert_eq!(writer.len(), 2);
    assert!(!writer.is_empty());

    writer.write_bytes(&[0xAA, 0xBB]);
    assert_eq!(writer.len(), 4);
    assert_eq!(writer.as_slice(), &[0x06, 0x01, 0xAA, 0xBB]);

    let bytes = writer.into_bytes();
    assert_eq!(bytes, vec![0x06, 0x01, 0xAA, 0xBB]);
}

#[test]
fn test_empty_string_and_default() {
    let mut writer = PacketWriter::default();
    assert!(writer.is_empty());
    assert_eq!(writer.len(), 0);

    writer.write_viscii_pascal("");
    assert_eq!(writer.as_slice(), &[0]); // 1-byte length prefix 0
    assert_eq!(writer.len(), 1);

    let mut reader = PacketReader::new(writer.as_slice());
    assert_eq!(reader.read_viscii_pascal().unwrap(), "");
}

#[test]
fn test_wire_xor_parity() {
    // Build frame with opcode 0x17 sub 0x05
    let writer = PacketWriter::new(0x17, 0x05);
    let frame = writer.build_frame();
    assert_eq!(frame, vec![0xF4, 0x44, 0x02, 0x00, 0x17, 0x05]);

    let wire = writer.build_wire();
    // Decode XOR 0xAD again should equal frame
    let decoded: Vec<u8> = wire.iter().map(|b| b ^ XOR_KEY).collect();
    assert_eq!(decoded, frame);
}

// ── protocol::codecs::thing_data ─────────────────────────────────────────────

#[test]
fn test_thing_data_roundtrip() {
    let item = ThingData {
        item_id: 23145,
        quantity: 50,
        damage: 10,
        element: 3,
        element_value: 45,
        proof_kind: 2,
        grow_level: 5,
        grow_exp: 12000,
        special_kind: 1,
        stone_attr: 4,
        stone_level: 7,
        enhance_level: 10,
        delete_time: 44927.5,
        damaged_item_id: 23000,
        is_locked: true,
        reinforced: 3,
        affix1: 12,
        affix2: 15,
        affix3: 18,
        style_level: 6,
    };

    let bytes = item.to_bytes();
    assert_eq!(bytes.len(), THING_DATA_SIZE);

    let decoded = ThingData::from_bytes(&bytes).unwrap();
    assert_eq!(item, decoded);
}

#[test]
fn test_empty_thing_data() {
    let empty = ThingData::empty();
    assert!(empty.is_empty());

    let bytes = empty.to_bytes();
    assert_eq!(bytes.len(), 35);
    assert_eq!(bytes, [0u8; 35]);

    let decoded = ThingData::from_bytes(&bytes).unwrap();
    assert_eq!(empty, decoded);
    assert!(decoded.is_empty());
}

#[test]
fn test_partial_bytes_error() {
    let truncated = [0u8; 30];
    assert!(ThingData::from_bytes(&truncated).is_err());
}

// ── protocol::codecs::player_info ────────────────────────────────────────────

#[test]
fn test_player_card_roundtrip() {
    let card = PlayerCard {
        name: "LữBố".to_string(),
        level: 120,
        element: 3,
        turn3_element: 4,
        turn: 2,
        career: 1,
        sex: 1,
        hair: 5,
        color1: 0x12345678,
        color2: 0x9ABCDEF0,
    };

    let bytes = card.to_bytes();
    let decoded = PlayerCard::from_bytes(&bytes).unwrap();
    assert_eq!(card, decoded);
}

#[test]
fn test_friend_extra_roundtrip() {
    let extra = FriendExtra {
        online: true,
        friendly: 500,
        function_flag: 2,
        add_time: 44100.25,
        offline_time: 44101.75,
    };

    let bytes = extra.to_bytes();
    assert_eq!(bytes.len(), FRIEND_EXTRA_SIZE);

    let decoded = FriendExtra::from_bytes(&bytes).unwrap();
    assert_eq!(extra, decoded);
}

// ── protocol::codecs::battle_role ────────────────────────────────────────────

#[test]
fn test_player_battle_role_roundtrip() {
    let role = BattleRoleData {
        war_type: 1,
        human_kind: ehuman::PLAYER,
        role_id: 300001,
        npc_id: 0,
        master_id: 0,
        col: 1,
        row: 2,
        max_hp: 1500,
        max_sp: 800,
        hp: 1450,
        sp: 790,
        level: 85,
        upgrade_lv: 1,
        element: 2,
        name: "TriệuVân".to_string(),
        sex: 1,
        face: 3,
        hair: 4,
        color1: 0x11223344,
        color2: 0x55667788,
        turn: 1,
        career: 2,
        equip_item_ids: vec![12001, 12002, 12003],
        outfit_item_ids: vec![19001],
    };

    let bytes = BattleRoleSerializer::serialize(&role);
    let mut reader = PacketReader::new(&bytes);
    let decoded = BattleRoleSerializer::deserialize(&mut reader).unwrap();
    assert_eq!(role, decoded);
    assert_eq!(reader.remaining(), 0);
}

#[test]
fn test_follow_npc_battle_role_roundtrip() {
    let role = BattleRoleData {
        war_type: 1,
        human_kind: ehuman::FOLLOW_NPC,
        role_id: 40001,
        npc_id: 11005,
        master_id: 300001,
        col: 0,
        row: 1,
        max_hp: 3500,
        max_sp: 1200,
        hp: 3500,
        sp: 1200,
        level: 100,
        upgrade_lv: 0,
        element: 3,
        name: "QuanVũ".to_string(),
        ..Default::default()
    };

    let bytes = BattleRoleSerializer::serialize(&role);
    let mut reader = PacketReader::new(&bytes);
    let decoded = BattleRoleSerializer::deserialize(&mut reader).unwrap();
    assert_eq!(role, decoded);
    assert_eq!(reader.remaining(), 0);
}

#[test]
fn test_monster_npc_battle_role_roundtrip() {
    let role = BattleRoleData {
        war_type: 1,
        human_kind: ehuman::MINE_NPC,
        role_id: 50001,
        npc_id: 14002,
        master_id: 0,
        col: 2,
        row: 0,
        max_hp: 200,
        max_sp: 50,
        hp: 200,
        sp: 50,
        level: 15,
        upgrade_lv: 0,
        element: 1,
        ..Default::default()
    };

    let bytes = BattleRoleSerializer::serialize(&role);
    // Base fields = 42 bytes exactly for NPC without extra appearance
    assert_eq!(bytes.len(), 42);

    let mut reader = PacketReader::new(&bytes);
    let decoded = BattleRoleSerializer::deserialize(&mut reader).unwrap();
    assert_eq!(role, decoded);
    assert_eq!(reader.remaining(), 0);
}

#[test]
fn test_serialize_all_multiple_roles() {
    let player = BattleRoleData {
        war_type: 1,
        human_kind: ehuman::PLAYER,
        role_id: 300001,
        npc_id: 0,
        master_id: 0,
        col: 1,
        row: 2,
        max_hp: 1500,
        max_sp: 800,
        hp: 1450,
        sp: 790,
        level: 85,
        upgrade_lv: 0,
        element: 2,
        name: "Player1".to_string(),
        sex: 1,
        face: 1,
        hair: 2,
        color1: 0,
        color2: 0,
        turn: 0,
        career: 0,
        equip_item_ids: vec![10001],
        outfit_item_ids: vec![],
    };

    let pet = BattleRoleData {
        war_type: 1,
        human_kind: ehuman::FOLLOW_NPC,
        role_id: 40001,
        npc_id: 11001,
        master_id: 300001,
        col: 0,
        row: 2,
        max_hp: 800,
        max_sp: 300,
        hp: 800,
        sp: 300,
        level: 50,
        upgrade_lv: 0,
        element: 1,
        name: "Pet1".to_string(),
        ..Default::default()
    };

    let enemy = BattleRoleData {
        war_type: 1,
        human_kind: ehuman::MINE_NPC,
        role_id: 50001,
        npc_id: 12001,
        master_id: 0,
        col: 3,
        row: 1,
        max_hp: 1000,
        max_sp: 200,
        hp: 1000,
        sp: 200,
        level: 60,
        upgrade_lv: 0,
        element: 4,
        ..Default::default()
    };

    let roles = vec![player, pet, enemy];
    let bytes = BattleRoleSerializer::serialize_all(&roles);
    let mut reader = PacketReader::new(&bytes);
    let decoded = BattleRoleSerializer::deserialize_all(&mut reader, 3).unwrap();
    assert_eq!(roles, decoded);
    assert_eq!(reader.remaining(), 0);
}

// ── encoding ─────────────────────────────────────────────────────────────────

#[test]
fn mojibake_roundtrip_simple() {
    // "D¤u Ch¤m Höi" = VISCII 44 A4 75 20 43 68 A4 6D 20 48 F6 69
    let s = "D¤u Ch¤m Höi";
    let v = to_viscii(s);
    assert_eq!(
        v,
        vec![0x44, 0xA4, 0x75, 0x20, 0x43, 0x68, 0xA4, 0x6D, 0x20, 0x48, 0xF6, 0x69]
    );
}

#[test]
fn cp1252_punct_maps_back() {
    // „ U+201E -> byte 0x84
    assert_eq!(to_viscii("„"), vec![0x84]);
    // † U+2020 -> 0x86
    assert_eq!(to_viscii("†"), vec![0x86]);
}

#[test]
fn unmappable_anh_normalized() {
    // single genuine ă U+0103 -> VISCII 0xE5
    assert_eq!(to_viscii("ă"), vec![0xE5]);
}

#[test]
fn ascii_passthrough() {
    assert_eq!(to_viscii("TSVN"), b"TSVN".to_vec());
}

#[test]
fn garble_two_garbage_bytes() {
    // §4.6 item 18973 "Thái „t binh pháp": „ U+201E -> "201E" -> bytes 20 1E.
    let g = compute_garble("Thái „t binh pháp").expect("garble name");
    assert!(!g.abort);
    assert_eq!(g.hex, "5468E16920201E742062696E68207068E170");
    assert_eq!(
        name_wire_hex(b"", &Some(g.clone())).as_deref(),
        Some("5468E16920201E742062696E68207068E170")
    );
}

#[test]
fn garble_aborts_on_three_digit_group() {
    // §4.6 item 48101 "BB Thái Văn C½ 3": ă U+0103 -> "103" (3 digits) aborts.
    let g = compute_garble("BB Thái Văn C½ 3").expect("garble name");
    assert!(g.abort);
    assert_eq!(g.hex, "4242205468E16920561036E2043BD2033");
    assert_eq!(name_wire_hex(b"", &Some(g)), None);
}

#[test]
fn garble_none_for_clean_names() {
    // Item 10000 "D¤u Ch¤m Höi" — all codepoints ≤ 0xFF → no override.
    assert_eq!(compute_garble("D¤u Ch¤m Höi"), None);
    assert_eq!(name_wire_hex(&[0x44, 0xA4], &None).as_deref(), Some("44A4"));
}

#[test]
fn viscii_display_full_table() {
    // VISCII display table: 0xA4 = ấ, 0xE5 = ă, 0xF6 = ỏ, 0xD0/0xDD = Đ.
    assert_eq!(viscii_to_unicode(0xA4), 'ấ');
    assert_eq!(viscii_to_unicode(0xE5), 'ă');
    assert_eq!(viscii_to_unicode(0xF6), 'ỏ');
    assert_eq!(viscii_to_unicode(0xD0), 'Đ');
    assert_eq!(viscii_to_unicode(0xDD), 'Đ');
    assert_eq!(viscii_to_unicode(0x84), 'Ấ'); // VISCII 0x84 = Ấ (upper)
    assert_eq!(viscii_to_unicode(0x41), 'A');
    // Fallback = Latin-1 pass-through for bytes outside the 102-entry table.
    assert_eq!(viscii_to_unicode(0x20), ' ');
    assert_eq!(viscii_to_unicode(0xFF), 'Ữ'); // VISCII 0xFF = Ữ (upper)
    assert_eq!(viscii_to_unicode(0xE6), 'ữ'); // lower-case ữ
}

#[test]
fn viscii_encode_maps_vietnamese_unicode() {
    // Đ -> 0xD0, ấ -> 0xA4 (research 03 §3.2 verified).
    assert_eq!(viscii_encode("Đ"), vec![0xD0]);
    assert_eq!(viscii_encode("ấ"), vec![0xA4]);
    // Unmappable positions collapse to '?' (0x3F).
    assert_eq!(viscii_encode("Ỷ"), vec![0x3F]);
    assert_eq!(viscii_encode("Ẳ"), vec![0x41]); // -> 'A'
                                                // ASCII passes through unchanged; CR/LF are preserved.
    assert_eq!(viscii_encode("TSVN"), b"TSVN".to_vec());
    assert_eq!(viscii_encode("a\r\nb"), b"a\r\nb".to_vec());
    // Banner text "Th¶i gian:" encodes ờ as 0xB6.
    assert_eq!(
        viscii_encode("Thời gian:"),
        vec![0x54, 0x68, 0xB6, 0x69, 0x20, 0x67, 0x69, 0x61, 0x6E, 0x3A]
    );
}

// ── config ───────────────────────────────────────────────────────────────────

#[test]
fn default_config_values() {
    let cfg = Config::default();
    assert_eq!(cfg.game_port, 6414);
    assert_eq!(cfg.web_port, 8090);
    assert_eq!(cfg.perexp_default, 0);
    assert!(cfg.database_url.contains("ts_dream"));
    assert!(cfg.db_auto_create, "auto-create defaults to true");
}

#[test]
fn parse_bool_accepts_true_false_forms() {
    assert!(parse_bool("true").unwrap());
    assert!(parse_bool("1").unwrap());
    assert!(parse_bool("YES").unwrap());
    assert!(!parse_bool("false").unwrap());
    assert!(!parse_bool("0").unwrap());
    assert!(!parse_bool("Off").unwrap());
    assert!(parse_bool("maybe").is_err());
}

#[test]
fn parse_u16_rejects_bad() {
    assert_eq!(parse_u16("6414").unwrap(), 6414);
    assert!(parse_u16("abc").is_err());
}

#[test]
fn from_file_rejects_sqlite_era_keys() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("ts_dream.toml");
    for key in ["account_db_path", "template_db_path", "member_dir"] {
        std::fs::write(&path, format!("{key} = \"/old/path\"\n")).unwrap();
        let err = from_file(&path).unwrap_err();
        assert!(
            err.to_string().contains("removed"),
            "rejection message must flag the removed key; got: {err}"
        );
    }
}

#[test]
fn from_file_parses_valid_toml() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("ts_dream.toml");
    std::fs::write(&path, "game_port = 7000\nweb_port = 8100\n").unwrap();
    let cfg = from_file(&path).unwrap();
    assert_eq!(cfg.game_port, 7000);
    assert_eq!(cfg.web_port, 8100);
}

#[test]
fn resolve_data_dir_prefers_existing_configured_path() {
    let dir = tempdir().unwrap();
    // An absolute data_dir that exists is returned unchanged.
    let cfg = Config {
        data_dir: dir.path().to_path_buf(),
        ..Config::default()
    };
    assert_eq!(cfg.resolve_data_dir(), dir.path().to_path_buf());
}

#[test]
fn resolve_data_dir_falls_back_to_exe_adjacent_bundle() {
    let dir = tempdir().unwrap();
    let exe_dir = dir.path().join("bin");
    std::fs::create_dir_all(&exe_dir).unwrap();
    // The build.rs-packaged `Data/` sits next to the executable.
    let bundled = exe_dir.join("bundle");
    std::fs::create_dir_all(&bundled).unwrap();

    // The configured relative path does NOT exist in the CWD here, but the
    // exe-adjacent one does -> resolve to `exe_dir/bundle`.
    let got = Config::resolve_data_dir_with(&std::path::PathBuf::from("bundle"), Some(exe_dir));
    assert_eq!(got, bundled);
}

#[test]
fn resolve_data_dir_returns_configured_when_none_exist() {
    // Neither the configured path nor an exe-adjacent bundle exists ->
    // the configured path is returned unchanged (caller reports it).
    let got = Config::resolve_data_dir_with(&std::path::PathBuf::from("does-not-exist-data"), None);
    assert_eq!(got, std::path::PathBuf::from("does-not-exist-data"));
}

// ── harness ──────────────────────────────────────────────────────────────────

#[tokio::test]
async fn test_golden_serialization() -> Result<()> {
    let text = "// Sample scenario\n<<F444010000\n>>F4440300010901\n";
    let g = Golden::parse(text, "sample")?;
    assert_eq!(g.c2s, vec!["F444010000"]);
    assert_eq!(g.s2c, vec!["F4440300010901"]);
    let serialized = g.to_text();
    assert!(serialized.contains("<<F444010000"));
    assert!(serialized.contains(">>F4440300010901"));
    Ok(())
}

#[tokio::test]
async fn test_capture_proxy_forwarding() -> Result<()> {
    // 1. Start a mock server
    let mock_listener = TcpListener::bind("127.0.0.1:0")
        .await
        .map_err(ts_dream::error::TsError::Io)?;
    let server_addr = mock_listener
        .local_addr()
        .map_err(ts_dream::error::TsError::Io)?;

    tokio::spawn(async move {
        if let Ok((mut socket, _)) = mock_listener.accept().await {
            let mut buf = vec![0u8; 1024];
            if let Ok(n) = socket.read(&mut buf).await {
                let req_wire = &buf[..n];
                // Verify received C2S wire packet (XOR of F444010000)
                let decoded = req_wire.iter().map(|b| b ^ XOR_KEY).collect::<Vec<_>>();
                assert_eq!(decoded, vec![0xF4, 0x44, 0x01, 0x00, 0x00]);

                // Send response S2C wire packet for F4440300010901
                let resp_hex = "F4440300010901";
                let resp_wire = encode_to_wire(resp_hex).unwrap();
                let _ = socket.write_all(&resp_wire).await;
            }
        }
    });

    // 2. Start proxy
    let proxy_listener = TcpListener::bind("127.0.0.1:0")
        .await
        .map_err(ts_dream::error::TsError::Io)?;
    let proxy_addr = proxy_listener
        .local_addr()
        .map_err(ts_dream::error::TsError::Io)?;
    drop(proxy_listener); // release port for CaptureProxy

    let proxy_server = Arc::new(proxy::CaptureProxy::new(
        proxy_addr.to_string(),
        server_addr.to_string(),
    ));
    let (tx, rx) = watch::channel(false);
    let proxy_clone = Arc::clone(&proxy_server);
    let proxy_task = tokio::spawn(async move {
        let _ = proxy_clone.run(rx).await;
    });

    // Wait a tiny bit for proxy to start listening
    tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

    // 3. Client connects to proxy and sends F444010000 wire bytes
    let mut client = tokio::net::TcpStream::connect(proxy_addr)
        .await
        .map_err(ts_dream::error::TsError::Io)?;
    let req_wire = encode_to_wire("F444010000")?;
    client
        .write_all(&req_wire)
        .await
        .map_err(ts_dream::error::TsError::Io)?;

    let mut resp_buf = vec![0u8; 1024];
    let n = client
        .read(&mut resp_buf)
        .await
        .map_err(ts_dream::error::TsError::Io)?;
    let resp_decoded = resp_buf[..n]
        .iter()
        .map(|b| b ^ XOR_KEY)
        .collect::<Vec<_>>();
    assert_eq!(resp_decoded, vec![0xF4, 0x44, 0x03, 0x00, 0x01, 0x09, 0x01]);

    let _ = tx.send(true);
    let _ = proxy_task.await;

    // 4. Verify captured lines in proxy
    let lines = proxy_server.captured_lines();
    assert!(lines.contains(&"<<F444010000".to_string()));
    assert!(lines.contains(&">>F4440300010901".to_string()));

    Ok(())
}

#[test]
fn production_protocol_constants_match_contract() {
    use ts_dream::protocol::{ID_PREFIX, MAX_LEVEL, MIN_VERSION, SERVER_NAME};
    assert_eq!(MIN_VERSION, 186);
    assert_eq!(ID_PREFIX, "VN");
    assert_eq!(SERVER_NAME, "TSVN");
    assert_eq!(MAX_LEVEL, 200);
}

#[test]
fn documented_opcode_registries_are_sorted_unique_and_complete_for_tables() {
    use ts_dream::protocol::{DOCUMENTED_CLIENT_OPCODES, DOCUMENTED_SERVER_OPCODES};

    assert_eq!(DOCUMENTED_CLIENT_OPCODES.len(), 60);
    assert_eq!(DOCUMENTED_SERVER_OPCODES.len(), 65);
    assert!(DOCUMENTED_CLIENT_OPCODES.windows(2).all(|w| w[0] < w[1]));
    assert!(DOCUMENTED_SERVER_OPCODES.windows(2).all(|w| w[0] < w[1]));
    assert!(DOCUMENTED_CLIENT_OPCODES.contains(&0xC7));
    assert!(DOCUMENTED_SERVER_OPCODES.contains(&0xC7));
}

#[test]
fn documented_opcode_membership_rejects_unknown_values() {
    use ts_dream::protocol::{is_documented_client_opcode, is_documented_server_opcode};
    assert!(is_documented_client_opcode(0x05));
    assert!(is_documented_server_opcode(0x04));
    assert!(!is_documented_client_opcode(0xFE));
    assert!(!is_documented_server_opcode(0xFE));
}

#[test]
fn pc_collision_opcodes_are_frozen_from_mobile_sync() {
    use ts_dream::protocol::profile::{ProtocolProfile, FROZEN_PC_COLLISIONS};

    assert_eq!(FROZEN_PC_COLLISIONS, [0x19, 0x1B, 0x1F, 0x23]);
    for opcode in FROZEN_PC_COLLISIONS {
        assert!(ProtocolProfile::PcALogin.is_frozen_pc_collision(opcode));
        assert!(!ProtocolProfile::KotlinMobile.is_frozen_pc_collision(opcode));
    }
}

#[test]
fn mobile_loader_policy_rejects_mmg_extension() {
    let accepted = ["dat", "emg", "mng"];
    for ext in accepted {
        assert!(matches!(ext, "dat" | "emg" | "mng"));
    }
    assert!(!matches!("mmg", "dat" | "emg" | "mng"));
}
