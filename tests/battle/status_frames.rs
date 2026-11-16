//! Unit tests for opcode 0x35 sub 0x0E (unit action) and sub 0x0F (unit status buff/debuff).
//!
//! Client decompile evidence:
//! - Main dispatcher `0078a89c`:
//!   `byte[opcode 0x35 + 0x78A8EE] = 46` -> `FUN_007952ab`
//! - `FUN_007952ab`:
//!   - Sub 0x0E -> `FUN_0064fbd8`: reads row, col, LE16 val, calls vtable `unit + 0x1c`
//!   - Sub 0x0F -> `FUN_0064fd34`: reads row, col, status_char, flag/turns, LE16 val,
//!     updates unit status slots (`+0x5be`..'G') and calls `FUN_0058f5e0` to refresh HUD.

use ts_dream::battle::packets;

fn frame_len(frame: &str) -> usize {
    let lo = u8::from_str_radix(&frame[4..6], 16).expect("valid lo byte");
    let hi = u8::from_str_radix(&frame[6..8], 16).expect("valid hi byte");
    lo as usize | ((hi as usize) << 8)
}

fn frame_total_bytes(frame: &str) -> usize {
    frame.len() / 2
}

#[test]
fn unit_action_35_0e_frame_is_byte_exact() {
    // row 1, col 2, val 0x03E8 (1000 LE -> E8 03)
    let frame = packets::unit_action(1, 2, 1000);
    assert_eq!(frame, "F4440600350E0102E803");
    assert_eq!(frame_len(&frame), 6, "Payload is 6 bytes: 35 0E + row + col + u16_le");
    assert_eq!(frame_total_bytes(&frame), 10, "4 header bytes + 6 payload bytes");

    // Alias must match exactly
    assert_eq!(packets::battle_unit_action(1, 2, 1000), frame);
}

#[test]
fn unit_action_35_0e_endianness_and_bounds() {
    // val = 0 -> "0000"
    assert_eq!(packets::unit_action(0, 0, 0), "F4440600350E00000000");

    // val = 0x1234 -> "3412" (Little-Endian)
    assert_eq!(packets::unit_action(3, 4, 0x1234), "F4440600350E03043412");

    // val = 0xFFFF -> "FFFF"
    assert_eq!(packets::unit_action(2, 1, 0xFFFF), "F4440600350E0201FFFF");
}

#[test]
fn battle_unit_status_35_0f_frame_is_byte_exact() {
    // row 2, col 3, status_char 'B' (0x42), flag 1, val 500 (0x01F4 -> F4 01)
    let frame = packets::battle_unit_status(2, 3, b'B', 1, 500);
    assert_eq!(frame, "F4440800350F02034201F401");
    assert_eq!(frame_len(&frame), 8, "Payload is 8 bytes: 35 0F + row + col + char + flag + u16_le");
    assert_eq!(frame_total_bytes(&frame), 12, "4 header bytes + 8 payload bytes");

    // Alias must match exactly
    assert_eq!(packets::unit_status(2, 3, b'B', 1, 500), frame);
}

#[test]
fn battle_unit_status_35_0f_status_chars_and_flags() {
    // Test status chars 'B' through 'G' supported in FUN_0064fd34
    let chars = [b'B', b'C', b'D', b'E', b'F', b'G'];
    for &ch in &chars {
        let frame = packets::battle_unit_status(0, 1, ch, 3, 100);
        let expected = format!("F4440800350F0001{:02X}036400", ch);
        assert_eq!(frame, expected);
        assert_eq!(frame_len(&frame), 8);
    }

    // Test turn / flag variations
    assert_eq!(
        packets::battle_unit_status(1, 1, b'C', 0, 0),
        "F4440800350F010143000000"
    );
    assert_eq!(
        packets::battle_unit_status(1, 1, b'C', 0xFF, 0xABCD),
        "F4440800350F010143FFCDAB"
    );
}

#[test]
fn opcode_35_frames_are_mutually_distinct() {
    let row = 2;
    let col = 3;
    let f_35_03 = packets::force_state7(row, col);
    let f_35_05 = packets::acting(row, col);
    let f_35_07 = packets::move_cell(row, col, row, col);
    let f_35_0e = packets::unit_action(row, col, 100);
    let f_35_0f = packets::battle_unit_status(row, col, b'B', 1, 100);

    assert!(f_35_03.starts_with("F44404003503"));
    assert!(f_35_05.starts_with("F44404003505"));
    assert!(f_35_07.starts_with("F44406003507"));
    assert!(f_35_0e.starts_with("F4440600350E"));
    assert!(f_35_0f.starts_with("F4440800350F"));

    assert_ne!(f_35_0e, f_35_03);
    assert_ne!(f_35_0e, f_35_05);
    assert_ne!(f_35_0e, f_35_07);
    assert_ne!(f_35_0e, f_35_0f);
    assert_ne!(f_35_0f, f_35_03);
    assert_ne!(f_35_0f, f_35_05);
    assert_ne!(f_35_0f, f_35_07);
}

#[test]
fn battle_status_constants_and_helpers_work_accurately() {
    // 1. Verify byte values for all 6 documented client status categories
    assert_eq!(packets::STATUS_DEF_MODIFIER, b'B');
    assert_eq!(packets::STATUS_ATK_MODIFIER, b'C');
    assert_eq!(packets::STATUS_INT_MODIFIER, b'D');
    assert_eq!(packets::STATUS_DMG_BONUS_PCT, b'E');
    assert_eq!(packets::STATUS_SEAL_RESIST_PCT, b'F');
    assert_eq!(packets::STATUS_SP_REDUCE_PCT, b'G');

    // 2. Relative stat modifier encoding: base is 1000 in client FUN_0058faf0
    assert_eq!(packets::stat_modifier_val(0), 1000);
    assert_eq!(packets::stat_modifier_val(50), 1050); // DEF/ATK/INT +50
    assert_eq!(packets::stat_modifier_val(-50), 950); // DEF/ATK/INT -50
    assert_eq!(packets::stat_modifier_val(120), 1120);

    // Frame with stat modifier
    let def_buff = packets::battle_unit_status(
        1,
        2,
        packets::STATUS_DEF_MODIFIER,
        3,
        packets::stat_modifier_val(50),
    );
    // 1050 = 0x041A -> LE 1A 04
    assert_eq!(def_buff, "F4440800350F010242031A04");

    // 3. Clear unit status HUD slot (flag_or_turns = 0, val = 0)
    let clear_atk = packets::clear_unit_status(1, 2, packets::STATUS_ATK_MODIFIER);
    assert_eq!(clear_atk, "F4440800350F010243000000");

    // 4. Purge status frames (35 01, matching Bear C# and client FUN_0064cd74)
    assert_eq!(packets::PURGE_TYPE_DISABLE, 1);
    assert_eq!(packets::PURGE_TYPE_BUFF, 2);
    assert_eq!(packets::PURGE_TYPE_DEBUFF, 3);
    assert_eq!(packets::PURGE_TYPE_AURA, 5);

    let purge_disable = packets::purge_status(2, 3, packets::PURGE_TYPE_DISABLE);
    assert_eq!(purge_disable, "F444070035010203010000");
    assert_eq!(frame_len(&purge_disable), 7, "Payload is 7 bytes: 35 01 + row + col + purge_type + 0000");

    let purge_buff = packets::purge_status(0, 1, packets::PURGE_TYPE_BUFF);
    assert_eq!(purge_buff, "F444070035010001020000");

    let purge_debuff = packets::purge_status(3, 4, packets::PURGE_TYPE_DEBUFF);
    assert_eq!(purge_debuff, "F444070035010304030000");
}

