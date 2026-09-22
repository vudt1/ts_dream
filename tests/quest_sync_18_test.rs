//! Integration tests for Checkpoint 5: quest & actor-state synchronization
//! (Opcode `0x18`).
//!
//! Scope (per `.scratch/op-working/research-checkpoint-5-quest-sync.md`):
//! 1. Builder hex parity for all 8 sub-opcodes (§4 of the research).
//! 2. Session-facing mutations — quest item add/merge/remove/clear, quest-dont
//!    flags, quest-log rows — each mirroring exactly one frame, including the
//!    client's refusal rules read out of the decompile:
//!    - `FUN_00720ca8` refuses a merge that would carry a stack past 255,
//!    - `FUN_00720df0` refuses `count > owned`,
//!    - slots are `1..=200` and marks `1..=300` (`_BoundErr` outside),
//!    - quest items and quest task rows share **one** 200-row pool
//!      (`0x654 + slot * 3` is written by both `FUN_00720ca8` and
//!      `FUN_0072bb6c`), so a task row can never claim an item's row.
//! 3. The `Logined1` login sequence carries the bulk sync (research §2:
//!    task log -> quest dont -> quest items) **only** when the session holds
//!    quest state, so a fresh character's byte stream keeps golden parity.

use ts_dream::protocol::codecs::quest_sync::QuestSyncCodec;
use ts_dream::server::dispatcher::HandleOutcome;
use ts_dream::server::handlers::quest_sync;
use ts_dream::server::session::{Conn, InventoryItem, Session};

/// Frames whose main opcode (byte 4 of `F4 44 lenLE <op>`) is `0x18`.
fn op18_frames(frames: &[String]) -> Vec<&str> {
    frames
        .iter()
        .filter(|frame| {
            hex::decode(frame)
                .map(|bytes| bytes.len() >= 5 && bytes[4] == 0x18)
                .unwrap_or(false)
        })
        .map(String::as_str)
        .collect()
}

// ============================================================================
// §4 research builder parity (all 8 sub-opcodes)
// ============================================================================

#[test]
fn test_research_builder_hexes_match_wire() {
    // Sub 0x01 / 0x02: quest item add / remove (5-byte payload).
    assert_eq!(
        QuestSyncCodec::build_item_add_hex(10001, 2),
        "F44405001801112702"
    );
    assert_eq!(
        QuestSyncCodec::build_item_remove_hex(10001, 1),
        "F44405001802112701"
    );

    // Sub 0x03: capacity-full toast; Sub 0x04: clear quest item.
    assert_eq!(QuestSyncCodec::build_quest_full_hex(), "F44402001803");
    assert_eq!(
        QuestSyncCodec::build_item_clear_hex(10001),
        "F444040018041127"
    );

    // Sub 0x05: quest-dont single; Sub 0x07: quest-dont bulk (3 B/entry).
    assert_eq!(
        QuestSyncCodec::build_dont_single_hex(101, 1),
        "F44405001805650001"
    );
    assert_eq!(
        QuestSyncCodec::build_dont_bulk_hex(&[
            ts_dream::protocol::codecs::quest_sync::QuestDontEntry {
                mark: 101,
                flag: 1
            },
            ts_dream::protocol::codecs::quest_sync::QuestDontEntry {
                mark: 102,
                flag: 1
            },
        ]),
        "F44408001807650001660001"
    );

    // Sub 0x06: quest task, single entry. NOTE(research): §4 line 115 writes
    // `F4440500...` (length 5) but the entry is 4 bytes after the 2-byte
    // `[18][06]` prefix -> payload 6 -> `0600`. The decompile
    // (`FUN_0072bb6c`, 4-byte stride) and `tests/wire_codec_14_18_test.rs`
    // both confirm `0600`; the research hex is a typo.
    assert_eq!(
        QuestSyncCodec::build_task_entry_hex(1, 10801, 3),
        "F4440600180601312A03"
    );
    assert_eq!(
        QuestSyncCodec::build_task_bulk_hex(&[
            ts_dream::protocol::codecs::quest_sync::QuestTaskEntry {
                slot: 1,
                quest_id: 10801,
                mark_step: 3,
            },
            ts_dream::protocol::codecs::quest_sync::QuestTaskEntry {
                slot: 2,
                quest_id: 10802,
                mark_step: 1,
            },
        ]),
        "F4440A00180601312A0302322A01"
    );

    // Sub 0x08: actor state flag (Thần xui leaves -> flag 0).
    assert_eq!(
        QuestSyncCodec::build_state_flag_hex(1001, 1, 0),
        "F44409001808E9030000010000"
    );
}

// ============================================================================
// Session-facing quest item mutations (Sub 0x01 / 0x02 / 0x04)
// ============================================================================

#[test]
fn test_add_merge_remove_clear_quest_item() {
    let mut conn = Conn::default();
    let mut out = HandleOutcome::default();

    // New item takes shared row 1 and emits Sub 0x01.
    assert!(quest_sync::add_quest_item(&mut conn.session, &mut out, 10001, 2));
    assert_eq!(out.outgoing[0].frame, "F44405001801112702");
    assert_eq!(conn.session.quest_items.len(), 1);
    assert_eq!(conn.session.quest_items[0].slot, 1);
    assert_eq!(conn.session.quest_items[0].count, 2);

    // Same id merges into the same row (client `FUN_00720ca8` merge path).
    assert!(quest_sync::add_quest_item(&mut conn.session, &mut out, 10001, 3));
    assert_eq!(
        conn.session.quest_items.len(),
        1,
        "merge must not open a second row"
    );
    assert_eq!(conn.session.quest_items[0].count, 5);
    assert_eq!(out.outgoing[1].frame, "F44405001801112703");

    // Successful removal emits Sub 0x02 with the requested count.
    assert_eq!(quest_sync::remove_quest_item(&mut conn.session, &mut out, 10001, 2), 2);
    assert_eq!(out.outgoing[2].frame, "F44405001802112702");
    assert_eq!(conn.session.quest_items[0].count, 3);

    // Client parity: `FUN_00720df0` requires count <= owned -> refuse both
    // the write and the frame (a sent frame the client rejects would toast
    // "failed" while the server bag keeps the item).
    assert_eq!(quest_sync::remove_quest_item(&mut conn.session, &mut out, 10001, 9), 0);
    assert_eq!(conn.session.quest_items[0].count, 3);
    assert_eq!(
        out.outgoing.len(),
        3,
        "a refused removal must not emit a frame"
    );

    // Removing the last of a stack frees the row.
    assert_eq!(quest_sync::remove_quest_item(&mut conn.session, &mut out, 10001, 3), 3);
    assert!(conn.session.quest_items.is_empty());
    assert_eq!(out.outgoing.len(), 4);

    // Sub 0x04 clears everything for an id, once.
    assert!(quest_sync::add_quest_item(&mut conn.session, &mut out, 26012, 1));
    assert!(quest_sync::clear_quest_item(&mut conn.session, &mut out, 26012));
    // 26012 = 0x659C -> LE `9C65`.
    assert_eq!(out.outgoing[5].frame, "F444040018049C65");
    assert!(conn.session.quest_items.is_empty());
    assert!(!quest_sync::clear_quest_item(&mut conn.session, &mut out, 26012));
    assert_eq!(
        out.outgoing.len(),
        6,
        "clearing an absent id must not emit a frame"
    );
}

#[test]
fn test_quest_item_capacity_refuses_with_full_toast() {
    let mut conn = Conn::default();
    let mut out = HandleOutcome::default();

    // Stack cap: a merge past 255 is refused wholesale (client returns 0 and
    // writes nothing), and the client's only signal is Sub 0x03.
    assert!(quest_sync::add_quest_item(&mut conn.session, &mut out, 10001, 255));
    assert!(!quest_sync::add_quest_item(&mut conn.session, &mut out, 10001, 1));
    assert_eq!(conn.session.quest_items[0].count, 255, "refused merge must not write");
    assert_eq!(out.outgoing.last().unwrap().frame, "F44402001803");

    // Row cap: the shared quest array holds 200 rows (row 1 is 10001 above,
    // rows 2..=200 take ids 10002..=10200).
    for id in 10002..=10200 {
        assert!(
            quest_sync::add_quest_item(&mut conn.session, &mut out, id, 1),
            "id {id} must still fit before the 200-row cap"
        );
    }
    assert_eq!(conn.session.quest_items.len(), quest_sync::QUEST_SLOT_MAX as usize);

    let before = out.outgoing.len();
    assert!(!quest_sync::add_quest_item(&mut conn.session, &mut out, 20001, 1));
    assert_eq!(conn.session.quest_items.len(), 200, "full bag must not grow");
    assert_eq!(
        out.outgoing[before].frame,
        "F44402001803",
        "row exhaustion must surface as the capacity-full toast"
    );
    assert_eq!(out.outgoing.len(), before + 1, "no add frame may follow the toast");

    // A zero-count add is a no-op (nothing on the wire, nothing in state).
    assert!(!quest_sync::add_quest_item(&mut conn.session, &mut out, 30001, 0));
    assert_eq!(out.outgoing.len(), before + 1);
}

// ============================================================================
// Quest-dont flags (Sub 0x05) and actor state flag (Sub 0x08)
// ============================================================================

#[test]
fn test_quest_dont_marks_and_client_bounds() {
    let mut conn = Conn::default();
    let mut out = HandleOutcome::default();

    // Set: mark 101 = 0x0065 LE -> `6500`, flag 1.
    assert!(quest_sync::set_quest_dont(&mut conn.session, &mut out, 101, 1));
    assert!(conn.session.quest_dont.contains(&101));
    assert_eq!(out.outgoing[0].frame, "F44405001805650001");

    // Bounds: the client computes `mark - 1` bounded to 299, so 0 underflows
    // and 301 overflows into `_BoundErr` -> reject before the wire.
    assert!(!quest_sync::set_quest_dont(&mut conn.session, &mut out, 0, 1));
    assert!(!quest_sync::set_quest_dont(&mut conn.session, &mut out, 301, 1));
    assert_eq!(
        out.outgoing.len(),
        1,
        "out-of-range marks must not reach the wire"
    );

    // Upper edge 300 = 0x012C LE -> `2C01` is still accepted.
    assert!(quest_sync::set_quest_dont(&mut conn.session, &mut out, 300, 1));
    assert_eq!(out.outgoing[1].frame, "F444050018052C0101");

    // Clearing emits flag 0 and drops the mark.
    assert!(quest_sync::set_quest_dont(&mut conn.session, &mut out, 101, 0));
    assert!(!conn.session.quest_dont.contains(&101));
    assert_eq!(out.outgoing[2].frame, "F44405001805650000");
}

#[test]
fn test_actor_state_flag_frame() {
    let mut conn = Conn::default();
    let mut out = HandleOutcome::default();
    conn.session.id = 1001; // 0x000003E9 -> LE `E9030000`

    // Kind 1 (Bad-Luck-God), flag 0 = expiry (client plays WA0006.wav).
    quest_sync::send_actor_state_flag(&conn.session, &mut out, 1, 0);
    assert_eq!(out.outgoing[0].frame, "F44409001808E9030000010000");
}

// ============================================================================
// Shared quest-entry pool: quest items and quest log rows (Sub 0x06)
// ============================================================================

#[test]
fn test_quest_task_rows_share_pool_with_items() {
    let mut conn = Conn::default();
    let mut out = HandleOutcome::default();

    // Item takes row 1 -> the first quest task must land on row 2, proving
    // both collections draw from the client's single `0x654 + slot * 3` array
    // (Bear's `TaskQuest.Count + 1` numbering would have collided here).
    assert!(quest_sync::add_quest_item(&mut conn.session, &mut out, 10001, 1));
    assert!(quest_sync::set_quest_task(&mut conn.session, &mut out, 10801, 3));
    assert_eq!(
        out.outgoing[1].frame,
        "F4440600180602312A03",
        "quest 10801 must take slot 2 (slot 1 belongs to the item)"
    );
    assert_eq!(conn.session.quest_tasks[&10801], (2, 3));

    // Re-saving the same quest keeps its row and only advances the step.
    assert!(quest_sync::set_quest_task(&mut conn.session, &mut out, 10801, 4));
    assert_eq!(out.outgoing[2].frame, "F4440600180602312A04");
    assert_eq!(conn.session.quest_tasks.len(), 1);

    // The next quest takes row 3.
    assert!(quest_sync::set_quest_task(&mut conn.session, &mut out, 10802, 1));
    assert_eq!(out.outgoing[3].frame, "F4440600180603322A01");

    // quest_id 0 is the client's free-row marker -> rejected, no frame.
    assert!(!quest_sync::set_quest_task(&mut conn.session, &mut out, 0, 1));
    assert_eq!(out.outgoing.len(), 4);
}

// ============================================================================
// Login bulk sync (research §2): task log -> quest dont -> quest items
// ============================================================================

#[test]
fn test_login_sequence_syncs_quest_state_only_when_present() {
    // Fresh character: no quest state -> not a single 0x18 frame, so the
    // golden Logined1 byte stream is untouched.
    let fresh = Session::new();
    let frames = ts_dream::server::spawn::build_logined_sequence_session(&fresh);
    assert!(
        op18_frames(&frames).is_empty(),
        "an empty quest state must add no opcode-0x18 frames"
    );

    // Populated state -> bulk task log, bulk quest dont, then the item adds.
    let mut s = Session::new();
    s.quest_tasks.insert(10801, (1, 3));
    s.quest_dont.insert(101);
    s.quest_items.push(InventoryItem {
        slot: 1,
        id: 10001,
        count: 2,
        ..Default::default()
    });
    // A second task proves the bulk frame carries slot-ordered entries.
    s.quest_tasks.insert(10802, (2, 1));

    let frames = ts_dream::server::spawn::build_logined_sequence_session(&s);
    assert_eq!(
        op18_frames(&frames),
        vec![
            // 0x18 Sub 0x06 bulk: 2 entries x 4 bytes (slot order 1, 2).
            "F4440A00180601312A0302322A01",
            // 0x18 Sub 0x07 bulk: mark 101 flag 1.
            "F44405001807650001",
            // 0x18 Sub 0x01: quest item 10001 x2.
            "F44405001801112702",
        ],
        "login must sync quest log, then quest dont, then quest items"
    );
}
