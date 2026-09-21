//! Quest and player state sync wire codecs (Opcode 0x18).
//!
//! Provides builders and parsers for S -> C quest packets:
//! - SubOp 0x01: Add quest item (`[0x18][0x01][ItemID: 2B LE][Count: 1B]`)
//! - SubOp 0x02: Remove quest item (`[0x18][0x02][ItemID: 2B LE][Count: 1B]`)
//! - SubOp 0x03: Quest bag full toast notification (`[0x18][0x03]`)
//! - SubOp 0x04: Clear quest item (`[0x18][0x04][ItemID: 2B LE]`)
//! - SubOp 0x05: Single quest dont flag (`[0x18][0x05][Mark: 2B LE][Flag: 1B]`)
//! - SubOp 0x06: Quest task log entry / entries (`[0x18][0x06][Slot: 1B][QuestID: 2B LE][MarkStep: 1B]...`)
//! - SubOp 0x07: Bulk quest dont flags (`[0x18][0x07][Mark: 2B LE][Flag: 1B]...`)
//! - SubOp 0x08: Actor state flag, e.g. Bad Luck God (`[0x18][0x08][CharID: 4B LE][Kind: 2B LE][Flag: 1B]`)

use crate::error::Result;
use crate::protocol::encoder;
use crate::protocol::reader::PacketReader;
use crate::protocol::writer::PacketWriter;
use crate::protocol::OP_ITEM_INFO;

/// Opcode 0x18 alias for quest sync.
pub const OP_QUEST_SYNC: u8 = OP_ITEM_INFO;

/// Sub-opcodes for Opcode 0x18.
pub const SUB_QUEST_ITEM_ADD: u8 = 0x01;
pub const SUB_QUEST_ITEM_REMOVE: u8 = 0x02;
pub const SUB_QUEST_FULL: u8 = 0x03;
pub const SUB_QUEST_ITEM_CLEAR: u8 = 0x04;
pub const SUB_QUEST_DONT_SINGLE: u8 = 0x05;
pub const SUB_QUEST_TASK_LOG: u8 = 0x06;
pub const SUB_QUEST_DONT_BULK: u8 = 0x07;
pub const SUB_ACTOR_STATE_FLAG: u8 = 0x08;

/// Single Quest Task Log item: (Slot, QuestID, MarkStep).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct QuestTaskEntry {
    pub slot: u8,
    pub quest_id: u16,
    pub mark_step: u8,
}

/// Single Quest Dont entry: (Mark, Flag).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct QuestDontEntry {
    pub mark: u16,
    pub flag: u8,
}

/// Standalone codec for Opcode 0x18 quest synchronization frames.
pub struct QuestSyncCodec;

impl QuestSyncCodec {
    /// Builds `0x18 Sub 0x01`: Add quest item.
    /// Payload: `[0x18, 0x01, ItemID: 2B LE, Count: 1B]`
    pub fn build_item_add_frame(item_id: u16, count: u8) -> Vec<u8> {
        let mut writer = PacketWriter::new(OP_QUEST_SYNC, SUB_QUEST_ITEM_ADD);
        writer.write_u16_le(item_id);
        writer.write_u8(count);
        writer.build_frame()
    }

    pub fn build_item_add_hex(item_id: u16, count: u8) -> String {
        encoder::hex(&Self::build_item_add_frame(item_id, count))
    }

    /// Builds `0x18 Sub 0x02`: Remove quest item.
    /// Payload: `[0x18, 0x02, ItemID: 2B LE, Count: 1B]`
    pub fn build_item_remove_frame(item_id: u16, count: u8) -> Vec<u8> {
        let mut writer = PacketWriter::new(OP_QUEST_SYNC, SUB_QUEST_ITEM_REMOVE);
        writer.write_u16_le(item_id);
        writer.write_u8(count);
        writer.build_frame()
    }

    pub fn build_item_remove_hex(item_id: u16, count: u8) -> String {
        encoder::hex(&Self::build_item_remove_frame(item_id, count))
    }

    /// Builds `0x18 Sub 0x03`: Quest bag full notice toast.
    /// Frame: `F4 44 02 00 18 03`
    pub fn build_quest_full_frame() -> [u8; 6] {
        [0xF4, 0x44, 0x02, 0x00, OP_QUEST_SYNC, SUB_QUEST_FULL]
    }

    pub fn build_quest_full_hex() -> &'static str {
        "F44402001803"
    }

    /// Builds `0x18 Sub 0x04`: Clear quest item by ID.
    /// Payload: `[0x18, 0x04, ItemID: 2B LE]`
    pub fn build_item_clear_frame(item_id: u16) -> Vec<u8> {
        let mut writer = PacketWriter::new(OP_QUEST_SYNC, SUB_QUEST_ITEM_CLEAR);
        writer.write_u16_le(item_id);
        writer.build_frame()
    }

    pub fn build_item_clear_hex(item_id: u16) -> String {
        encoder::hex(&Self::build_item_clear_frame(item_id))
    }

    /// Builds `0x18 Sub 0x05`: Update single Quest Dont flag.
    /// Payload: `[0x18, 0x05, Mark: 2B LE, Flag: 1B]`
    pub fn build_dont_single_frame(mark: u16, flag: u8) -> Vec<u8> {
        let mut writer = PacketWriter::new(OP_QUEST_SYNC, SUB_QUEST_DONT_SINGLE);
        writer.write_u16_le(mark);
        writer.write_u8(flag);
        writer.build_frame()
    }

    pub fn build_dont_single_hex(mark: u16, flag: u8) -> String {
        encoder::hex(&Self::build_dont_single_frame(mark, flag))
    }

    /// Builds `0x18 Sub 0x06`: Single Quest Task log entry.
    /// Payload: `[0x18, 0x06, Slot: 1B, QuestID: 2B LE, MarkStep: 1B]`
    pub fn build_task_entry_frame(slot: u8, quest_id: u16, mark_step: u8) -> Vec<u8> {
        let mut writer = PacketWriter::new(OP_QUEST_SYNC, SUB_QUEST_TASK_LOG);
        writer.write_u8(slot);
        writer.write_u16_le(quest_id);
        writer.write_u8(mark_step);
        writer.build_frame()
    }

    pub fn build_task_entry_hex(slot: u8, quest_id: u16, mark_step: u8) -> String {
        encoder::hex(&Self::build_task_entry_frame(slot, quest_id, mark_step))
    }

    /// Builds `0x18 Sub 0x06`: Bulk Quest Task log entries (4 bytes per entry).
    /// Payload: `[0x18, 0x06] + N * [Slot: 1B, QuestID: 2B LE, MarkStep: 1B]`
    pub fn build_task_bulk_frame(entries: &[QuestTaskEntry]) -> Vec<u8> {
        let mut writer = PacketWriter::new(OP_QUEST_SYNC, SUB_QUEST_TASK_LOG);
        for entry in entries {
            writer.write_u8(entry.slot);
            writer.write_u16_le(entry.quest_id);
            writer.write_u8(entry.mark_step);
        }
        writer.build_frame()
    }

    pub fn build_task_bulk_hex(entries: &[QuestTaskEntry]) -> String {
        encoder::hex(&Self::build_task_bulk_frame(entries))
    }

    /// Builds `0x18 Sub 0x07`: Bulk Quest Dont entries (3 bytes per entry).
    /// Payload: `[0x18, 0x07] + N * [Mark: 2B LE, Flag: 1B]`
    pub fn build_dont_bulk_frame(entries: &[QuestDontEntry]) -> Vec<u8> {
        let mut writer = PacketWriter::new(OP_QUEST_SYNC, SUB_QUEST_DONT_BULK);
        for entry in entries {
            writer.write_u16_le(entry.mark);
            writer.write_u8(entry.flag);
        }
        writer.build_frame()
    }

    pub fn build_dont_bulk_hex(entries: &[QuestDontEntry]) -> String {
        encoder::hex(&Self::build_dont_bulk_frame(entries))
    }

    /// Builds `0x18 Sub 0x08`: Character state flag (e.g. Bad Luck God).
    /// Payload: `[0x18, 0x08, CharID: 4B LE, Kind: 2B LE, Flag: 1B]`
    pub fn build_state_flag_frame(char_id: u32, kind: u16, flag: u8) -> Vec<u8> {
        let mut writer = PacketWriter::new(OP_QUEST_SYNC, SUB_ACTOR_STATE_FLAG);
        writer.write_u32_le(char_id);
        writer.write_u16_le(kind);
        writer.write_u8(flag);
        writer.build_frame()
    }

    pub fn build_state_flag_hex(char_id: u32, kind: u16, flag: u8) -> String {
        encoder::hex(&Self::build_state_flag_frame(char_id, kind, flag))
    }

    /// Deserializes a single task entry from reader.
    pub fn decode_task_entry(reader: &mut PacketReader<'_>) -> Result<QuestTaskEntry> {
        let slot = reader.read_u8()?;
        let quest_id = reader.read_u16_le()?;
        let mark_step = reader.read_u8()?;
        Ok(QuestTaskEntry {
            slot,
            quest_id,
            mark_step,
        })
    }

    /// Deserializes a single quest dont entry from reader.
    pub fn decode_dont_entry(reader: &mut PacketReader<'_>) -> Result<QuestDontEntry> {
        let mark = reader.read_u16_le()?;
        let flag = reader.read_u8()?;
        Ok(QuestDontEntry { mark, flag })
    }
}
