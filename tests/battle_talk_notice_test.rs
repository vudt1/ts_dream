//! Tests for Opcode 0x35 Sub 0x06 (NPC Talk / Banner) and Sub 0x0B (Chat / Notice).
//!
//! Client decompile evidence:
//! - Dispatcher `0078a89c` -> `FUN_007952ab` (Case 46 / Opcode 0x35):
//!   - Sub 0x06 -> `FUN_0064e064`: Reads `mode (1B)`, `actor_id (4B LE)`, `tpl_id (4B LE)`,
//!     creates `THuman`, registers to battle HUD (`FUN_0063c1d0`) and displays toast banner 2000ms.
//!     Đối chiếu Bear C#: `PacketCreator(53, 6)` (`k=1`, `actor_id=0`, `tpl_id=dialog_id`).
//!   - Sub 0x0B -> `FUN_0075c518`: Reads repeated `[type:1B][len:1B][str:len B]` records (max 21),
//!     prepends formatting prefix (types 1, 2, 3) and pushes directly into battle chat `TTalkMsgForm`.

use ts_dream::battle::packets::{
    self, battle_notice_type, notice_type,
};
use ts_dream::encoding::{viscii_decode, viscii_encode};

fn frame_payload_len(frame: &str) -> usize {
    let lo = u8::from_str_radix(&frame[4..6], 16).expect("valid lo byte");
    let hi = u8::from_str_radix(&frame[6..8], 16).expect("valid hi byte");
    lo as usize | ((hi as usize) << 8)
}

fn frame_total_bytes(frame: &str) -> usize {
    frame.len() / 2
}

// ============================================================================
// Ticket 16: Builder 35 06 (Thoại NPC / banner diễn hoạt trong trận)
// ============================================================================

#[test]
fn test_battle_npc_talk_byte_exact() {
    // k = 1, actor_id = 10001 (0x00002711 -> 11 27 00 00), tpl_id = 20002 (0x00004E22 -> 22 4E 00 00)
    let frame = packets::battle_npc_talk(1, 10001, 20002);
    assert_eq!(frame, "F4440B0035060111270000224E0000");

    assert_eq!(frame_payload_len(&frame), 11, "Payload must be exactly 11 bytes");
    assert_eq!(frame_total_bytes(&frame), 15, "Frame must be exactly 15 bytes (4 header + 11 payload)");
}

#[test]
fn test_battle_npc_talk_endianness_and_bounds() {
    // Zero bounds
    let zero_frame = packets::battle_npc_talk(0, 0, 0);
    assert_eq!(zero_frame, "F4440B003506000000000000000000");
    assert_eq!(frame_payload_len(&zero_frame), 11);

    // Max values (0xFF, 0xFFFFFFFF, 0xFFFFFFFF)
    let max_frame = packets::battle_npc_talk(0xFF, 0xFFFFFFFF, 0xFFFFFFFF);
    assert_eq!(max_frame, "F4440B003506FFFFFFFFFFFFFFFFFF");
    assert_eq!(frame_payload_len(&max_frame), 11);

    // Endianness verification (actor_id 0x12345678 -> 78 56 34 12, tpl_id 0xAABBCCDD -> DD CC BB AA)
    let endian_frame = packets::battle_npc_talk(2, 0x12345678, 0xAABBCCDD);
    assert_eq!(endian_frame, "F4440B0035060278563412DDCCBBAA");
}

#[test]
fn test_battle_npc_dialog_bear_convention() {
    // Bear C# PacketCreator(53, 6): k=1, actor_id=0, tpl_id=dialog_id (16-bit expanded to 32-bit LE)
    let dialog_id: u16 = 1001; // 0x03E9 -> E9 03 00 00
    let frame = packets::battle_npc_dialog(dialog_id);
    let expected = packets::battle_npc_talk(1, 0, 1001);

    assert_eq!(frame, expected);
    assert_eq!(frame, "F4440B0035060100000000E9030000");
    assert_eq!(frame_payload_len(&frame), 11);
    assert_eq!(frame_total_bytes(&frame), 15);

    // Bounds check for u16::MAX (65535 -> FF FF 00 00)
    let max_dialog = packets::battle_npc_dialog(u16::MAX);
    assert_eq!(max_dialog, "F4440B0035060100000000FFFF0000");
}

// ============================================================================
// Ticket 17: Builder 35 0B (Tin nhắn / bảng tin thông báo chiến đấu)
// ============================================================================

#[test]
fn test_battle_notice_type_constants() {
    assert_eq!(battle_notice_type::SYSTEM, 1);
    assert_eq!(battle_notice_type::NOTICE, 2);
    assert_eq!(battle_notice_type::COMBAT, 3);

    // Alias must re-export same constants
    assert_eq!(notice_type::SYSTEM, 1);
    assert_eq!(notice_type::NOTICE, 2);
    assert_eq!(notice_type::COMBAT, 3);
}

#[test]
fn test_battle_chat_notice_empty() {
    let frame = packets::battle_chat_notice(&[]);
    assert_eq!(frame, "F4440200350B");
    assert_eq!(frame_payload_len(&frame), 2);
    assert_eq!(frame_total_bytes(&frame), 6);
}

#[test]
fn test_battle_chat_notice_single_ascii() {
    // type 1 (SYSTEM), text "Ready!" (6 bytes: 52 65 61 64 79 21)
    let frame = packets::battle_chat_notice(&[(battle_notice_type::SYSTEM, "Ready!")]);
    // Payload: 35 0B (2B) + 01 (type) + 06 (len) + 526561647921 (6B) = 10 bytes payload (0x0A LE -> 0A 00)
    assert_eq!(frame, "F4440A00350B0106526561647921");
    assert_eq!(frame_payload_len(&frame), 10);
    assert_eq!(frame_total_bytes(&frame), 14);
}

#[test]
fn test_battle_system_message_helper() {
    let text = "Trận chiến bắt đầu!";
    let frame = packets::battle_system_message(text);
    let expected = packets::battle_chat_notice(&[(1, text)]);
    assert_eq!(frame, expected);
    assert!(frame.starts_with("F444"));
    assert_eq!(&frame[8..14], "350B01"); // 35 0B + type 1
}

#[test]
fn test_battle_notice_helper() {
    let text = "Cảnh báo đối thủ!";
    let frame = packets::battle_notice(battle_notice_type::NOTICE, text);
    let expected = packets::battle_chat_notice(&[(2, text)]);
    assert_eq!(frame, expected);
    assert_eq!(&frame[8..14], "350B02"); // 35 0B + type 2
}

#[test]
fn test_battle_chat_notice_vietnamese_viscii_roundtrip() {
    let sample_messages = [
        "Chiến thắng rực rỡ!",
        "Quân địch đã bỏ chạy!",
        "Hào kiệt xuất hiện tại chiến trường!",
        "Triệu Tử Long dũng mãnh vô song!",
        "Nguyễn Văn Ý nhận được 5000 Tiền!",
        "Đinh Bộ Lĩnh thống nhất giang sơn!",
    ];

    for (idx, &msg) in sample_messages.iter().enumerate() {
        let msg_type = ((idx % 3) + 1) as u8;
        let frame = packets::battle_chat_notice(&[(msg_type, msg)]);

        // Validate framing
        let p_len = frame_payload_len(&frame);
        assert_eq!(frame_total_bytes(&frame), 4 + p_len);

        // Header: F4 44 <p_len LE> 35 0B
        assert_eq!(&frame[0..4], "F444");
        assert_eq!(&frame[8..12], "350B");

        // Entry 1: <type:1B><len:1B><viscii_hex>
        let entry_type = u8::from_str_radix(&frame[12..14], 16).unwrap();
        assert_eq!(entry_type, msg_type);

        let entry_len = u8::from_str_radix(&frame[14..16], 16).unwrap() as usize;
        let text_hex = &frame[16..16 + entry_len * 2];

        // Decode hex bytes
        let mut raw_bytes = Vec::with_capacity(entry_len);
        for chunk in text_hex.as_bytes().chunks(2) {
            let s = std::str::from_utf8(chunk).unwrap();
            raw_bytes.push(u8::from_str_radix(s, 16).unwrap());
        }

        // Must match direct viscii_encode
        assert_eq!(raw_bytes, viscii_encode(msg));

        // Must round-trip back to identical UTF-8 text
        let decoded = viscii_decode(&raw_bytes);
        assert_eq!(decoded, msg, "Failed round-trip for: {msg}");
    }
}

#[test]
fn test_battle_chat_notice_multiple_entries_batching() {
    let entries = [
        (battle_notice_type::SYSTEM, "Hệ thống: Trận đấu kịch tính."),
        (battle_notice_type::NOTICE, "Cảnh báo: Tướng địch ra chiêu."),
        (battle_notice_type::COMBAT, "Sát thương: Gây 1500 damage!"),
    ];

    let frame = packets::battle_chat_notice(&entries);
    let p_len = frame_payload_len(&frame);
    assert_eq!(frame_total_bytes(&frame), 4 + p_len);

    // Verify structure: 35 0B followed by 3 records
    assert_eq!(&frame[8..12], "350B");

    let mut cursor = 12;
    for &(expected_type, text) in &entries {
        let rec_type = u8::from_str_radix(&frame[cursor..cursor + 2], 16).unwrap();
        assert_eq!(rec_type, expected_type);
        cursor += 2;

        let rec_len = u8::from_str_radix(&frame[cursor..cursor + 2], 16).unwrap() as usize;
        cursor += 2;

        let rec_hex = &frame[cursor..cursor + rec_len * 2];
        cursor += rec_len * 2;

        let mut bytes = Vec::new();
        for chunk in rec_hex.as_bytes().chunks(2) {
            let s = std::str::from_utf8(chunk).unwrap();
            bytes.push(u8::from_str_radix(s, 16).unwrap());
        }
        assert_eq!(viscii_decode(&bytes), text);
    }

    assert_eq!(cursor, frame.len(), "All frame bytes must be accounted for");
}

#[test]
fn test_battle_chat_notice_cap_at_21_entries() {
    // Client FUN_0075c518 loop condition is `local_20 < 0x15` (max 21 records).
    // If 25 entries are provided, only the first 21 should be included.
    let many_entries: Vec<(u8, &str)> = (0..25)
        .map(|i| ((i % 3 + 1) as u8, "Dòng tin thông báo"))
        .collect();

    let frame = packets::battle_chat_notice(&many_entries);

    // Count records in frame:
    let mut cursor = 12; // skip F444 + len(2B) + 350B(2B) = 12 hex chars
    let mut record_count = 0;
    while cursor < frame.len() {
        cursor += 2; // skip type
        let rec_len = u8::from_str_radix(&frame[cursor..cursor + 2], 16).unwrap() as usize;
        cursor += 2 + rec_len * 2;
        record_count += 1;
    }

    assert_eq!(record_count, 21, "Must be capped at exactly 21 records");
}

#[test]
fn test_battle_chat_notice_long_string_truncation_safety() {
    // Wire protocol length field is 1 byte (max 255).
    // A string exceeding 255 bytes must be cleanly clamped to 255 without panic or framing error.
    let long_text = "A".repeat(300);
    let frame = packets::battle_chat_notice(&[(1, &long_text)]);

    assert_eq!(&frame[8..12], "350B");
    assert_eq!(&frame[12..14], "01"); // type 1
    assert_eq!(&frame[14..16], "FF"); // length 255 (0xFF)

    let encoded_chars = &frame[16..16 + 255 * 2];
    assert_eq!(encoded_chars.len(), 510); // 255 bytes * 2 hex chars
    assert_eq!(encoded_chars, "41".repeat(255));

    // Full frame length must be 4 header + (2 + 1 + 1 + 255) = 263 bytes (526 hex chars)
    assert_eq!(frame_payload_len(&frame), 259);
    assert_eq!(frame_total_bytes(&frame), 263);
}

#[test]
fn test_battle_chat_notice_empty_string_entry() {
    let frame = packets::battle_chat_notice(&[(1, "")]);
    assert_eq!(frame, "F4440400350B0100");
    assert_eq!(frame_payload_len(&frame), 4); // 35 0B + 01 (type) + 00 (len 0)
    assert_eq!(frame_total_bytes(&frame), 8);
}

#[test]
fn test_battle_chat_notice_capital_o_tilde_and_nfd_normalization() {
    // Probe regression: 'Õ' (U+00D5) is at 0xD5 in Latin-1, but MUST map to 0xA0 in VISCII 1.1 (RFC 1456).
    // In VISCII, byte 0xD5 represents 'ạ'. A naive Latin-1 pass-through corrupts 'Õ' into 'ạ'.
    let msg_with_o_tilde = "VÕ THẦN XUẤT THẾ!";
    let frame = packets::battle_system_message(msg_with_o_tilde);
    assert!(frame.starts_with("F444"));

    // Extract VISCII bytes from frame
    let len = u8::from_str_radix(&frame[14..16], 16).unwrap() as usize;
    let hex_slice = &frame[16..16 + len * 2];
    let mut raw_bytes = Vec::new();
    for chunk in hex_slice.as_bytes().chunks(2) {
        let s = std::str::from_utf8(chunk).unwrap();
        raw_bytes.push(u8::from_str_radix(s, 16).unwrap());
    }

    // Index 1 is 'Õ' -> must be 0xA0
    assert_eq!(raw_bytes[1], 0xA0, "Capital Õ must map to VISCII 0xA0, NOT 0xD5");
    assert_eq!(viscii_decode(&raw_bytes), msg_with_o_tilde);

    // Test NFD decomposed input (e.g., 'O' + U+0303)
    let nfd_o_tilde = "V\u{004F}\u{0303} L\u{00C2}M"; // "VÕ LÂM" in NFD
    let frame_nfd = packets::battle_system_message(nfd_o_tilde);
    let len_nfd = u8::from_str_radix(&frame_nfd[14..16], 16).unwrap() as usize;
    let hex_nfd = &frame_nfd[16..16 + len_nfd * 2];
    let mut raw_nfd = Vec::new();
    for chunk in hex_nfd.as_bytes().chunks(2) {
        let s = std::str::from_utf8(chunk).unwrap();
        raw_nfd.push(u8::from_str_radix(s, 16).unwrap());
    }
    assert_eq!(raw_nfd[1], 0xA0, "NFD decomposed Õ must normalize and map to 0xA0");
    assert_eq!(viscii_decode(&raw_nfd), "VÕ LÂM");
}

// ============================================================================
// Opcode 0x35 distinctness across the entire family
// ============================================================================

#[test]
fn test_opcode_35_all_subcodes_distinct_and_well_formed() {
    let f_35_01 = packets::purge_status(1, 2, 1);
    let f_35_03 = packets::force_state7(1, 2);
    let f_35_04 = packets::drop_item(100, 1, 2, 3, 4);
    let f_35_05 = packets::acting(1, 2);
    let f_35_06 = packets::battle_npc_talk(1, 100, 200);
    let f_35_07 = packets::move_cell(1, 2, 3, 4);
    let f_35_0b = packets::battle_system_message("Test");
    let f_35_0e = packets::unit_action(1, 2, 500);
    let f_35_0f = packets::battle_unit_status(1, 2, b'B', 1, 500);

    let all_frames = [
        ("3501", &f_35_01),
        ("3503", &f_35_03),
        ("3504", &f_35_04),
        ("3505", &f_35_05),
        ("3506", &f_35_06),
        ("3507", &f_35_07),
        ("350B", &f_35_0b),
        ("350E", &f_35_0e),
        ("350F", &f_35_0f),
    ];

    for (sub, frame) in &all_frames {
        assert!(frame.starts_with("F444"), "Frame must start with F444");
        assert_eq!(&frame[8..12], *sub, "Frame must have subcode {sub}");
        assert_eq!(
            frame_total_bytes(frame),
            4 + frame_payload_len(frame),
            "Length header must match total payload for {sub}"
        );
    }
}
