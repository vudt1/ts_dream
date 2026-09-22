//! Quest & actor-state synchronization (Opcode `0x18`, Checkpoint 5).
//!
//! Server -> client only channel: the client's `FUN_00790ed5` switch reads the
//! sub byte straight out of the payload (its string starts at the sub byte —
//! index 0 — so every field offset below is "1-based position inside
//! `[sub, data...]`"), then mutates `LocalActor`:
//!
//! | Sub | Frame | Client handler |
//! |:---:|:---|:---|
//! | `01` | `[18][01][ItemID: 2B LE][Count: 1B]` | `FUN_00720ca8` add/merge quest item |
//! | `02` | `[18][02][ItemID: 2B LE][Count: 1B]` | `FUN_00720df0` consume quest item |
//! | `03` | `[18][03]` | toast "quest capacity full" (2000 ms) |
//! | `04` | `[18][04][ItemID: 2B LE]` | `FUN_00720f00` clear quest item |
//! | `05` | `[18][05][Mark: 2B LE][Flag: 1B]` | `FUN_00721088` quest-dont flag |
//! | `06` | `[18][06] + N * [Slot: 1B][QuestID: 2B LE][MarkStep: 1B]` | `FUN_0072bb6c` quest log |
//! | `07` | `[18][07] + N * [Mark: 2B LE][Flag: 1B]` | `FUN_0072ba54` quest-dont bulk |
//! | `08` | `[18][08][CharID: 4B LE][Kind: 2B LE][Flag: 1B]` | `FUN_00729a88` actor state flag |
//!
//! The wire codecs live in [`QuestSyncCodec`] (Checkpoint 1); this module owns
//! the **session-facing half**: it mutates `Session` quest state, mirrors every
//! mutation into exactly one frame, and builds the login bulk sync.
//!
//! ### Capacity rules (client bounds, verified in the decompile)
//! - Quest items and quest task rows share **one** 200-row array on the client
//!   (`FUN_00720ca8` and `FUN_0072bb6c` both write `0x654 + slot * 3`), so both
//!   collections here draw slots from a single pool (`next_free_slot`). This
//!   is stricter than the Bear server (which numbers task slots
//!   `TaskQuest.Count + 1` and can collide with an item-owned row).
//! - A quest-item stack never exceeds 255 (`FUN_00720ca8` refuses the merge
//!   instead of splitting), and a removal larger than the owned count is
//!   refused outright (`FUN_00720df0` requires `count <= owned`). The server
//!   refuses both too, so the two bags cannot desync.
//! - Slots are `1..=200` and quest-dont marks are `1..=300`: outside those
//!   ranges the client raises `_BoundErr` (hard error), so the mutators reject
//!   them before anything reaches the wire.
//!
//! Both capacity failures surface as [`QuestSyncCodec::build_quest_full_hex`]
//! (`0x18 Sub 0x03`), the client's only "quest capacity full" signal.

use crate::protocol::codecs::quest_sync::{
    QuestDontEntry, QuestSyncCodec, QuestTaskEntry,
};
use crate::server::dispatcher::HandleOutcome;
use crate::server::session::{Conn, InventoryItem, Session};

/// Highest quest-entry slot the client accepts (`if (200 < slot) _BoundErr`
/// in `FUN_0072bb6c` / `FUN_00720ca8` / `FUN_00720df0`).
pub const QUEST_SLOT_MAX: u8 = 200;

/// Lowest / highest quest-dont mark: the client computes `mark - 1` and bounds
/// it to `299` (`FUN_0072ba54`, `FUN_00721088`), so `0` underflows and `301`
/// overflows — both raise `_BoundErr`.
pub const QUEST_DONT_MARK_MIN: u16 = 1;
pub const QUEST_DONT_MARK_MAX: u16 = 300;

/// Per-row stack cap of the quest bag (`iVar9 = 0xff - count` in
/// `FUN_00720ca8`).
pub const QUEST_ITEM_STACK_CAP: u16 = 0xFF;

/// Slots occupied in the client's shared quest-entry array (quest items **and**
/// quest task rows both live at `LocalActor + 0x654 + slot * 3`).
fn occupied_slots(session: &Session) -> std::collections::HashSet<u8> {
    let mut used = std::collections::HashSet::new();
    for item in &session.quest_items {
        if item.id > 0 {
            used.insert(item.slot);
        }
    }
    for (_, (slot, _)) in &session.quest_tasks {
        used.insert(*slot);
    }
    used
}

/// First free row in `1..=QUEST_SLOT_MAX`, shared by quest items and quest
/// task rows so the two collections can never claim the same client row.
fn next_free_slot(session: &Session) -> Option<u8> {
    let used = occupied_slots(session);
    (1..=QUEST_SLOT_MAX).find(|slot| !used.contains(slot))
}

/// Push the "quest capacity full" toast — the client's only capacity signal
/// (`0x18 Sub 0x03`, `FUN_00790ed5` case 3).
fn send_full(out: &mut HandleOutcome, reason: &str, id: u16) {
    tracing::debug!(id, reason, "quest capacity full toast (0x18 Sub 0x03) emitted");
    out.send(QuestSyncCodec::build_quest_full_hex());
}

/// `0x18 Sub 0x01` — add `count` of `item_id` to the quest bag.
///
/// Mirrors `FUN_00720ca8`: an existing stack absorbs the count only when the
/// whole add fits under [`QUEST_ITEM_STACK_CAP`], a new item takes the next
/// free shared row, and any refusal emits the full toast instead of a partial
/// write. Returns `true` when the state changed and the frame was sent.
pub fn add_quest_item(conn: &mut Conn, out: &mut HandleOutcome, item_id: u16, count: u8) -> bool {
    if item_id == 0 || count == 0 {
        return false;
    }
    if let Some(idx) = conn
        .session
        .quest_items
        .iter()
        .position(|item| item.id == item_id)
    {
        let owned = u16::from(conn.session.quest_items[idx].count);
        if u16::from(count) > QUEST_ITEM_STACK_CAP.saturating_sub(owned) {
            // `FUN_00720ca8` returns 0 (no write) when the merge would carry
            // the byte past 0xFF — refuse here too rather than desync.
            send_full(out, "quest item stack would exceed 255", item_id);
            return false;
        }
        conn.session.quest_items[idx].count += count;
        out.send(QuestSyncCodec::build_item_add_hex(item_id, count));
        return true;
    }
    let Some(slot) = next_free_slot(&conn.session) else {
        send_full(out, "quest bag has no free row (200 rows used)", item_id);
        return false;
    };
    conn.session.quest_items.push(InventoryItem {
        slot,
        id: item_id,
        count,
        ..Default::default()
    });
    out.send(QuestSyncCodec::build_item_add_hex(item_id, count));
    true
}

/// `0x18 Sub 0x02` — consume `count` of `item_id` from the quest bag.
///
/// Mirrors `FUN_00720df0`: removing more than owned is refused (returns `0`,
/// no frame), an empty stack frees its row, and a successful removal emits the
/// frame with the **requested** count. Returns the count actually removed.
pub fn remove_quest_item(conn: &mut Conn, out: &mut HandleOutcome, item_id: u16, count: u8) -> u32 {
    if item_id == 0 || count == 0 {
        return 0;
    }
    let Some(idx) = conn
        .session
        .quest_items
        .iter()
        .position(|item| item.id == item_id)
    else {
        return 0;
    };
    let owned = u32::from(conn.session.quest_items[idx].count);
    if u32::from(count) > owned {
        tracing::debug!(
            item_id,
            count,
            owned,
            "quest item removal refused: client requires count <= owned"
        );
        return 0;
    }
    conn.session.quest_items[idx].count -= count;
    if conn.session.quest_items[idx].count == 0 {
        conn.session.quest_items.remove(idx);
    }
    out.send(QuestSyncCodec::build_item_remove_hex(item_id, count));
    u32::from(count)
}

/// `0x18 Sub 0x04` — drop every stack of `item_id` from the quest bag in one
/// frame (`FUN_00720f00`). Returns `true` when an entry existed (only then is
/// the frame sent — the client toasts "not found" for an unknown id).
pub fn clear_quest_item(conn: &mut Conn, out: &mut HandleOutcome, item_id: u16) -> bool {
    if item_id == 0 {
        return false;
    }
    let before = conn.session.quest_items.len();
    conn.session
        .quest_items
        .retain(|item| item.id != item_id);
    if conn.session.quest_items.len() == before {
        return false;
    }
    out.send(QuestSyncCodec::build_item_clear_hex(item_id));
    true
}

/// `0x18 Sub 0x05` — set (`flag != 0`) or clear (`flag == 0`) one quest-dont
/// mark, then emit the single-entry frame. Marks outside
/// `1..=QUEST_DONT_MARK_MAX` are rejected before the wire (the client raises
/// `_BoundErr` on them). Returns `false` when the mark was rejected.
pub fn set_quest_dont(conn: &mut Conn, out: &mut HandleOutcome, mark: u16, flag: u8) -> bool {
    if !(QUEST_DONT_MARK_MIN..=QUEST_DONT_MARK_MAX).contains(&mark) {
        tracing::debug!(
            mark,
            "quest-dont mark rejected: client accepts 1..={QUEST_DONT_MARK_MAX}"
        );
        return false;
    }
    if flag == 0 {
        conn.session.quest_dont.remove(&mark);
    } else {
        conn.session.quest_dont.insert(mark);
    }
    out.send(QuestSyncCodec::build_dont_single_hex(mark, flag));
    true
}

/// `0x18 Sub 0x06` — write one quest-log row `(slot, quest_id, mark_step)`.
///
/// A quest already in the log keeps its row (only the step changes); a new one
/// takes the next free shared row. Returns `false` when `quest_id` is 0 or the
/// shared 200-row array is exhausted (full toast, no state change).
pub fn set_quest_task(conn: &mut Conn, out: &mut HandleOutcome, quest_id: u16, mark_step: u8) -> bool {
    if quest_id == 0 {
        tracing::debug!("quest task rejected: quest_id 0 is the client's free-row marker");
        return false;
    }
    let slot = match conn.session.quest_tasks.get(&quest_id) {
        Some((slot, _)) => *slot,
        None => match next_free_slot(&conn.session) {
            Some(slot) => slot,
            None => {
                send_full(out, "quest log has no free row (200 rows used)", quest_id);
                return false;
            }
        },
    };
    conn.session
        .quest_tasks
        .insert(quest_id, (slot, mark_step));
    out.send(QuestSyncCodec::build_task_entry_hex(slot, quest_id, mark_step));
    true
}

/// `0x18 Sub 0x08` — push an actor state flag for this session's character
/// (`Kind = 1` Bad-Luck-God `+0x448`, `Kind = 2` secondary `+0x455`; `Flag = 0`
/// plays `sound\WA0006.wav` + expiry toast on the client). Unknown kinds are
/// ignored client-side, so none are rejected here.
pub fn send_actor_state_flag(conn: &Conn, out: &mut HandleOutcome, kind: u16, flag: u8) {
    out.send(QuestSyncCodec::build_state_flag_hex(conn.session.id, kind, flag));
}

/// Full login sync (Checkpoint 5 §2): bulk quest log (`0x18 Sub 0x06`),
/// bulk quest-dont (`0x18 Sub 0x07`), then one add frame per quest item
/// (`0x18 Sub 0x01`).
///
/// Pure over `&Session` (no packet side effects) so it can be appended to the
/// `Logined1` sequence. Rows are emitted in **slot order** — `HashMap` /
/// `HashSet` iteration is unordered and the frame stream must be stable. An
/// empty quest state yields **no frames at all**, so a fresh character's
/// login byte stream keeps its golden parity.
pub fn sync_frames(session: &Session) -> Vec<String> {
    let mut frames = Vec::new();

    if !session.quest_tasks.is_empty() {
        let mut entries: Vec<QuestTaskEntry> = session
            .quest_tasks
            .iter()
            .map(|(quest_id, (slot, mark_step))| QuestTaskEntry {
                slot: *slot,
                quest_id: *quest_id,
                mark_step: *mark_step,
            })
            .collect();
        entries.sort_by_key(|entry| entry.slot);
        frames.push(QuestSyncCodec::build_task_bulk_hex(&entries));
    }

    if !session.quest_dont.is_empty() {
        let mut marks: Vec<u16> = session.quest_dont.iter().copied().collect();
        marks.sort_unstable();
        let entries: Vec<QuestDontEntry> = marks
            .into_iter()
            .map(|mark| QuestDontEntry { mark, flag: 1 })
            .collect();
        frames.push(QuestSyncCodec::build_dont_bulk_hex(&entries));
    }

    let mut items = session.quest_items.clone();
    items.sort_by_key(|item| item.slot);
    for item in items.iter().filter(|item| item.id > 0 && item.count > 0) {
        frames.push(QuestSyncCodec::build_item_add_hex(item.id, item.count));
    }

    frames
}
