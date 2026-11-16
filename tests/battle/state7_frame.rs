//! Ticket 12 — frame `35 03` (force a grid unit into state 7 / attack animation).
//!
//! Byte layout re-derived from client `aLogin.exe` `FUN_0064cfec`
//! (`.scratch/client-pseudo-op-code/battle/02_battle_positions.md §2.7`) using
//! the `HANDOFF-client-reverse.md §0.2` index convention (`RP[0]` = sub,
//! `content[j] == RP[j-1]`):
//!
//! ```text
//! content[0] = 0x35   MainOp
//! content[1] = 0x03   SubOp
//! content[2] = grp    (0..3, BoundErr when > 3)
//! content[3] = slot   (0..4, BoundErr when > 4)
//! ```
//!
//! Content is 4 bytes, so the `LEN` field is 4 and the on-wire frame is 8
//! bytes (`F444` + LE16(4) + content). The handler only acts when the target
//! cell is occupied (`cell+0x48 != 0`): it flushes the 21 FX slots and calls
//! `FUN_00647708(unit, 7)`.
//!
//! NOTE: the builder is deliberately **not** wired into `runner.rs` — when the
//! client expects this frame is deferred to ticket 14 (runtime capture).

use ts_dream::battle::packets;

/// Payload length declared in the `F444` header, in bytes.
fn frame_len(frame: &str) -> usize {
    let lo = u8::from_str_radix(&frame[4..6], 16).unwrap() as usize;
    let hi = u8::from_str_radix(&frame[6..8], 16).unwrap() as usize;
    lo | (hi << 8)
}

/// Total on-wire length implied by the hex string.
fn frame_total_bytes(frame: &str) -> usize {
    frame.len() / 2
}

#[test]
fn force_state7_frame_is_byte_exact() {
    // grp 3, slot 2 (a player cell).
    let frame = packets::force_state7(3, 2);
    assert_eq!(frame, "F444040035030302");
    assert_eq!(frame_len(&frame), 4, "LEN = 35 03 + 2 coordinate bytes");
    assert_eq!(frame_total_bytes(&frame), 8, "4-byte header + 4-byte payload");
}

#[test]
fn force_state7_payload_is_exactly_four_bytes() {
    let frame = packets::force_state7(0, 0);
    // Strip the 4-byte `F444` + LE16 header: the payload must be 4 bytes.
    let payload = &frame[8..];
    assert_eq!(payload, "35030000");
    assert_eq!(payload.len() / 2, 4, "35 03 payload must be exactly 4 bytes");
    // Only the two coordinate bytes follow `35 03`.
    assert_eq!(&payload[4..], "0000");
    assert_eq!(payload[4..].len() / 2, 2, "35 03 has no extra payload bytes");
}

#[test]
fn force_state7_keeps_grid_bounds() {
    // Both bounds are inclusive: grp 0..=3, slot 0..=4.
    assert_eq!(packets::force_state7(0, 0), "F444040035030000");
    assert_eq!(packets::force_state7(3, 4), "F444040035030304");
}

#[test]
fn force_state7_encodes_each_coordinate_independently() {
    assert_eq!(packets::force_state7(1, 2), "F444040035030102");
    assert_eq!(packets::force_state7(2, 3), "F444040035030203");
}

#[test]
fn force_state7_is_distinct_from_other_35_frames() {
    // 35 03 (state 7) must not be confused with 35 05 (acting) or 35 07 (move).
    let state7 = packets::force_state7(3, 2);
    assert!(state7.starts_with("F44404003503"));
    assert_ne!(state7, packets::acting(3, 2));
    assert_ne!(state7, packets::move_cell(3, 2, 3, 2));
}
