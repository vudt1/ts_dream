//! Quest & actor-state synchronization (Opcode `0x18`).
//!
//! Server -> client only channel (`S→C 0x18`):
//! - `03`: `[18][03]` — Quest Ledger capacity full (200 slots used)
//! - `04`: `[18][04][QuestID: 2B LE]` — remove quest task row by id
//! - `05`: `[18][05][BitID: 2B LE][Flag: 1B]` — single `quest_dont` bit (`1..=2400`)
//! - `06`: `[18][06] + N * [Slot: 1B][QuestID: 2B LE][MarkStep: 1B]` — Quest Ledger (`1..=200`)
//! - `07`: `[18][07] + N * [ByteIdx: 2B LE][Bitmask: 1B]` — bulk `quest_dont` bitmask (`1..=300`)
//! - `08`: `[18][08][CharID: 4B LE][Kind: 2B LE][Flag: 1B]` — actor state flag (Ghost=1, God=2)
//!
//! All `Item.dat` items (including quest items) live in `homdo` (`0x17`, 25 slots),
//! never on `0x18`. Any `C→S 0x18` packet (such as `0x18 01/02` from the client
//! debug `TMissionForm`) is rejected unconditionally for all accounts including GM.

use std::collections::{BTreeMap, HashSet};

use crate::protocol::codecs::quest_sync::{QuestDontEntry, QuestSyncCodec, QuestTaskEntry};
use crate::server::dispatcher::{HandleOutcome, OpcodeCtx};
use crate::server::session::Session;

/// Highest Quest Ledger slot accepted by the client (`1..=200`).
pub const QUEST_SLOT_MAX: u8 = 200;

/// Valid `quest_dont` bit_id range (`Mark.dat.position` in `1..=2400`, 300 bytes * 8 bits).
pub const QUEST_DONT_MARK_MIN: u16 = 1;
pub const QUEST_DONT_MARK_MAX: u16 = 2400;

/// Reject any `C→S 0x18` packet (including `0x18 01/02` from debug `TMissionForm`)
/// unconditionally for all accounts including GM.
pub fn handle_client_quest_sync(ctx: &mut OpcodeCtx<'_>) {
    tracing::warn!(
        char_id = ctx.conn.session.id,
        gm_level = ctx.conn.session.gm_level,
        sub = ctx.sub,
        payload_len = ctx.payload.len(),
        "rejected C->S 0x18 quest sync packet (server->client only)"
    );
}

/// Slots `1..=200` currently occupied in the Quest Ledger (`quest_tasks`).
fn occupied_slots(session: &Session) -> HashSet<u8> {
    session
        .quest_tasks
        .values()
        .map(|(slot, _)| *slot)
        .filter(|&slot| (1..=QUEST_SLOT_MAX).contains(&slot))
        .collect()
}

/// First free slot in `1..=QUEST_SLOT_MAX`.
fn next_free_slot(session: &Session) -> Option<u8> {
    let used = occupied_slots(session);
    (1..=QUEST_SLOT_MAX).find(|slot| !used.contains(slot))
}

/// Push the "quest capacity full" toast (`S→C 0x18 03`).
fn send_full(out: &mut HandleOutcome, reason: &str, id: u16) {
    tracing::debug!(id, reason, "quest capacity full toast (0x18 Sub 0x03) emitted");
    out.send(QuestSyncCodec::build_quest_full_hex());
}

/// Allocate or reuse a valid `1..=QUEST_SLOT_MAX` slot for `quest_id` and update
/// `session.quest_tasks` without emitting wire frames.
///
/// Returns `None` if `quest_id == 0` or all 200 Quest Ledger slots are full.
pub fn upsert_quest_task_slot(
    session: &mut Session,
    quest_id: u16,
    mark_step: u8,
) -> Option<u8> {
    if quest_id == 0 {
        return None;
    }
    let existing = session
        .quest_tasks
        .get(&quest_id)
        .map(|(slot, _)| *slot)
        .filter(|&slot| (1..=QUEST_SLOT_MAX).contains(&slot));
    let slot = match existing {
        Some(slot) => slot,
        None => next_free_slot(session)?,
    };
    session.quest_tasks.insert(quest_id, (slot, mark_step));
    Some(slot)
}

/// `0x18 Sub 0x05` — set (`flag != 0`) or clear (`flag == 0`) one `quest_dont`
/// bit (`1..=2400`) and emit `S→C 0x18 05 [bit_id: u16 LE][flag: u8]`.
pub fn set_quest_dont(
    session: &mut Session,
    out: &mut HandleOutcome,
    bit_id: u16,
    flag: u8,
) -> bool {
    if !(QUEST_DONT_MARK_MIN..=QUEST_DONT_MARK_MAX).contains(&bit_id) {
        tracing::debug!(
            bit_id,
            "quest-dont bit_id rejected: valid range is 1..={QUEST_DONT_MARK_MAX}"
        );
        return false;
    }
    if flag == 0 {
        session.quest_dont.remove(&bit_id);
    } else {
        session.quest_dont.insert(bit_id);
    }
    out.send(QuestSyncCodec::build_dont_single_hex(bit_id, flag));
    true
}

/// `0x18 Sub 0x06` — write one Quest Ledger row `(slot 1..=200, quest_id, mark_step)`.
///
/// When all 200 slots are full and `quest_id` is new, refuses All-or-Nothing and
/// emits `S→C 0x18 03`.
pub fn set_quest_task(
    session: &mut Session,
    out: &mut HandleOutcome,
    quest_id: u16,
    mark_step: u8,
) -> bool {
    if quest_id == 0 {
        tracing::debug!("quest task rejected: quest_id 0 is the client's free-row marker");
        return false;
    }
    let Some(slot) = upsert_quest_task_slot(session, quest_id, mark_step) else {
        send_full(out, "quest ledger has no free row (200 rows used)", quest_id);
        return false;
    };
    out.send(QuestSyncCodec::build_task_entry_hex(slot, quest_id, mark_step));
    true
}

/// `0x18 Sub 0x04` — drop one Quest Ledger row by `quest_id` and free its slot.
pub fn remove_quest_task(session: &mut Session, out: &mut HandleOutcome, quest_id: u16) -> bool {
    if quest_id == 0 {
        return false;
    }
    let Some((_slot, _)) = session.quest_tasks.remove(&quest_id) else {
        return false;
    };
    out.send(QuestSyncCodec::build_item_clear_hex(quest_id));
    tracing::debug!(quest_id, "quest task removed (0x18 Sub 0x04 clear-by-id)");
    true
}

/// `0x18 Sub 0x08` — push an actor state flag for this session's character
/// (`Kind = 1` Ghost, `Kind = 2` God).
pub fn send_actor_state_flag(session: &Session, out: &mut HandleOutcome, kind: u16, flag: u8) {
    out.send(QuestSyncCodec::build_state_flag_hex(session.id, kind, flag));
}

/// Full login sync (`Logined1`):
/// 1. `0x18 06`: bulk Quest Ledger rows (`slot 1..=200`, sorted by slot)
/// 2. `0x18 07`: bulk `quest_dont` bitmask (`[byte_idx 1..=300: u16 LE][bitmask: u8]`)
/// 3. `0x18 08`: actor state flags (`ghost` / `god`) when active
pub fn sync_frames(session: &Session) -> Vec<String> {
    let mut frames = Vec::new();

    if !session.quest_tasks.is_empty() {
        let mut entries: Vec<QuestTaskEntry> = session
            .quest_tasks
            .iter()
            .filter_map(|(&quest_id, &(slot, mark_step))| {
                if (1..=QUEST_SLOT_MAX).contains(&slot) {
                    Some(QuestTaskEntry {
                        slot,
                        quest_id,
                        mark_step,
                    })
                } else {
                    None
                }
            })
            .collect();
        entries.sort_by_key(|entry| entry.slot);
        if !entries.is_empty() {
            frames.push(QuestSyncCodec::build_task_bulk_hex(&entries));
        }
    }

    if !session.quest_dont.is_empty() {
        let mut byte_masks: BTreeMap<u16, u8> = BTreeMap::new();
        for &bit_id in &session.quest_dont {
            if (QUEST_DONT_MARK_MIN..=QUEST_DONT_MARK_MAX).contains(&bit_id) {
                let zero_based = bit_id - 1;
                let byte_idx = (zero_based / 8) + 1; // 1..=300
                let bit_offset = (zero_based % 8) as u8;
                *byte_masks.entry(byte_idx).or_insert(0) |= 1u8 << bit_offset;
            }
        }
        if !byte_masks.is_empty() {
            let entries: Vec<QuestDontEntry> = byte_masks
                .into_iter()
                .map(|(byte_idx, bitmask)| QuestDontEntry {
                    mark: byte_idx,
                    flag: bitmask,
                })
                .collect();
            frames.push(QuestSyncCodec::build_dont_bulk_hex(&entries));
        }
    }

    if session.ghost > 0 {
        frames.push(QuestSyncCodec::build_state_flag_hex(
            session.id,
            1,
            session.ghost,
        ));
    }
    if session.god > 0 {
        frames.push(QuestSyncCodec::build_state_flag_hex(session.id, 2, 1));
    }

    frames
}
