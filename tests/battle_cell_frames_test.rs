//! Ticket 03 — grid-cell frames `0B 01` (reset) and `35 07` (move).
//!
//! Byte layouts are re-derived from client `aLogin.exe` using the
//! `HANDOFF-client-reverse.md §0.2` index convention (`content[0]` = main
//! opcode, `RP[0]` = sub opcode):
//!
//! ```text
//! 0B 01  content[0]=0x0B content[1]=0x01 terms
//!        content[2] = row (grp, boundErr when > 3)
//!        content[3] = col (slot, boundErr when > 4)
//!        → minimum 4 bytes; a trailing `00` pad is optional and ignored
//!          (evidence 0064bb24:50-62, HANDOFF §3.2)
//!
//! 35 07  content[0]=0x35 content[1]=0x07
//!        content[2] = srcGrp  content[3] = srcSlot
//!        content[4] = dstGrp  content[5] = dstSlot
//!        → content 6 bytes / coordinate payload 4 bytes
//!          (evidence FUN_0064e3e0, 02_battle_positions.md §2.10)
//! ```

use std::collections::HashMap;

use ts_dream::battle::construction::Battle;
use ts_dream::battle::packets;
use ts_dream::battle::runner::{BattleData, Out};
use ts_dream::data::tables::{Item, Npc, Skill, TexpRow};

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

/// ---- `0B 01` cell reset ------------------------------------------------

#[test]
fn reset_cell_frame_is_byte_exact() {
    let frame = packets::reset_cell(3, 2);
    // F444 + LE16(4) + [0B][01][grp=03][slot=02] — no trailing pad byte.
    assert_eq!(frame, "F44404000B010302");
    assert_eq!(frame_len(&frame), 4, "payload = 0B 01 + 2 coordinate bytes");
    assert_eq!(frame_total_bytes(&frame), 8, "4-byte header + 4-byte payload");
}

#[test]
fn reset_cell_payload_is_exactly_four_bytes() {
    let frame = packets::reset_cell(0, 0);
    // Strip the 4-byte `F444` + LE16 header: the payload must be 4 bytes.
    let payload = &frame[8..];
    assert_eq!(payload, "0B010000");
    assert_eq!(payload.len() / 2, 4, "0B 01 payload must be exactly 4 bytes");
}

#[test]
fn reset_cell_keeps_grid_bounds() {
    // Both bounds are inclusive: row 0..=3, col 0..=4.
    assert_eq!(packets::reset_cell(0, 0), "F44404000B010000");
    assert_eq!(packets::reset_cell(3, 4), "F44404000B010304");
}

/// ---- `35 07` cell move -------------------------------------------------

#[test]
fn move_cell_frame_is_byte_exact() {
    // src (grp 3, slot 2) -> dst (grp 0, slot 1).
    let frame = packets::move_cell(3, 2, 0, 1);
    assert_eq!(frame, "F4440600350703020001");
    assert_eq!(frame_len(&frame), 6, "payload = 35 07 + 4 coordinate bytes");
    assert_eq!(frame_total_bytes(&frame), 10, "4-byte header + 6-byte payload");
}

#[test]
fn move_cell_content_is_six_bytes_payload_four() {
    let frame = packets::move_cell(0, 4, 3, 0);
    // Content (opcode + sub + 4 coords) = 6 bytes.
    let content = &frame[8..];
    assert_eq!(content, "350700040300");
    assert_eq!(content.len() / 2, 6, "35 07 content must be 6 bytes");
    // Coordinate payload after `35 07` = 4 bytes.
    assert_eq!(&content[4..], "00040300");
    assert_eq!(content[4..].len() / 2, 4, "35 07 coordinate payload is 4 bytes");
}

#[test]
fn move_cell_keeps_grid_bounds() {
    assert_eq!(packets::move_cell(0, 0, 0, 0), "F4440600350700000000");
    assert_eq!(packets::move_cell(3, 4, 3, 4), "F4440600350703040304");
}

#[test]
fn reset_and_move_are_distinct_frames() {
    let reset = packets::reset_cell(2, 2);
    let moved = packets::move_cell(2, 2, 3, 2);
    assert!(reset.starts_with("F44404000B01"));
    assert!(moved.starts_with("F44406003507"));
    assert_ne!(reset, moved);
}

/// ---- `finish()` emits the normalised reset frame -----------------------

fn table<'a>() -> (
    HashMap<i64, Npc>,
    HashMap<i64, Skill>,
    HashMap<i64, Item>,
    HashMap<i64, [i64; 4]>,
    HashMap<i64, ts_dream::battle::runner::PlayerSnapshot>,
    Vec<TexpRow>,
) {
    (
        HashMap::new(),
        HashMap::new(),
        HashMap::new(),
        HashMap::new(),
        HashMap::new(),
        Vec::new(),
    )
}

#[test]
fn finish_broadcasts_normalised_reset_frames() {
    let (npcs, skills, items, pet_slots, players, texps) = table();
    let data = BattleData::new(&npcs, &skills, &items, &pet_slots, &players, &texps, None, 0, 0);

    let mut battle = Battle::new(1, 0);
    {
        let c = battle.cell_mut(3, 2).unwrap();
        c.typ = 2;
        c.id = 7;
        c.hp = 100;
        c.hp_max = 100;
        c.lv = 10;
        c.team = 1;
    }

    let mut out = Vec::new();
    battle.finish(&data, 1, false, false, &mut out);

    // Player cell (grp 3, slot 2) and its pet row (grp 2, slot 2).
    let expected = format!(
        "{}{}{}",
        packets::hide_from_map(7),
        packets::reset_cell(3, 2),
        packets::reset_cell(2, 2),
    );
    assert!(
        out.contains(&Out::Broadcast(expected.clone())),
        "finish() must use the 4-byte reset builder, got {out:?}"
    );
    assert!(
        !expected.contains("F44405000B01"),
        "0B 01 must not carry the optional pad byte"
    );
}
