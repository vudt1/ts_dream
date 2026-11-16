//! Unit & Integration tests for Checkpoint 1: Wire Codecs of Opcode 0x14 and 0x18.
//!
//! Tests verify:
//! - 14-byte EveResult binary encoding & decoding (1:1 match with eve.emg).
//! - 17-byte talk step body & 21-byte full frame serialization (F4 44 11 00 14 01 00 ...).
//! - 11-byte actor lock/unlock frames (0x14 Sub 0x2C).
//! - 6-byte end talk frame (F4 44 02 00 14 08).
//! - 8 sub-opcodes of Opcode 0x18 (Quest items, Quest dont, Quest task log, Actor state flag).
//! - Little-endian byte order, wire framing, and roundtrip deserialization via PacketReader & PacketWriter.

use ts_dream::data::loaders::EveResult;
use ts_dream::protocol::codecs::npc_talk::{
    NpcTalkCodec, TalkLockMode, EVE_RESULT_SIZE, TALK_STEP_BODY_SIZE,
};
use ts_dream::protocol::codecs::quest_sync::{
    QuestDontEntry, QuestSyncCodec, QuestTaskEntry, OP_QUEST_SYNC, SUB_QUEST_FULL,
};
use ts_dream::protocol::reader::PacketReader;
use ts_dream::protocol::writer::PacketWriter;

// ============================================================================
// Opcode 0x14: NpcEvent Wire Codec Tests
// ============================================================================

#[test]
fn test_opcode_14_eve_result_exact_14_bytes() {
    let original = EveResult {
        result_group_no: 1,
        result_no: 1,
        result_type: 1,  // Talk
        result_class: 3, // NpcTeam
        parameter: 1,    // MapObjectID = 1
        parameter_style: 0,
        result_value: 0,
        result_mean_no: 10364, // 0x287C
    };

    let encoded = NpcTalkCodec::encode_eve_result(&original);
    assert_eq!(encoded.len(), EVE_RESULT_SIZE);

    // Expected byte sequence (14 bytes):
    // [01, 00] [01] [01] [03] [01, 00] [00] [00, 00, 00, 00] [7C, 28]
    let expected = [
        0x01, 0x00, // result_group_no = 1
        0x01, // result_no = 1
        0x01, // result_type = 1
        0x03, // result_class = 3
        0x01, 0x00, // parameter = 1
        0x00, // parameter_style = 0
        0x00, 0x00, 0x00, 0x00, // result_value = 0
        0x7C, 0x28, // result_mean_no = 10364
    ];
    assert_eq!(encoded, expected);

    // Roundtrip via PacketReader
    let mut reader = PacketReader::new(&encoded);
    let decoded = reader.read_eve_result().expect("Must decode EveResult");
    assert_eq!(decoded, original);
    assert_eq!(reader.remaining(), 0);
}

#[test]
fn test_opcode_14_talk_step_frame_and_hex() {
    // Player reply step (class = 7, parameter = 0, mean_no = 10367 = 0x287F)
    let res = EveResult {
        result_group_no: 1,
        result_no: 2,
        result_type: 1,  // Talk
        result_class: 7, // Player
        parameter: 0,
        parameter_style: 0,
        result_value: 0,
        result_mean_no: 10367,
    };

    let body = NpcTalkCodec::build_talk_step_body(&res);
    assert_eq!(body.len(), TALK_STEP_BODY_SIZE);
    assert_eq!(body[0], 0x14); // OP_NPC_EVENT
    assert_eq!(body[1], 0x01); // SUB_TALK_STEP
    assert_eq!(body[2], 0x00); // Pad

    let frame = NpcTalkCodec::build_talk_step_frame(&res);
    assert_eq!(frame.len(), 21); // 4 bytes header + 17 bytes body
    assert_eq!(&frame[0..2], &[0xF4, 0x44]); // Magic
    assert_eq!(&frame[2..4], &[0x11, 0x00]); // Length 17 LE

    let hex = NpcTalkCodec::build_talk_step_hex(&res);
    assert_eq!(hex, "F44411001401000100020107000000000000007F28");

    // Test with PacketWriter helper
    let mut writer = PacketWriter::empty();
    writer.write_u8(0x14);
    writer.write_u8(0x01);
    writer.write_u8(0x00);
    writer.write_eve_result(&res);
    let writer_frame = writer.build_frame();
    assert_eq!(writer_frame, frame);
}

#[test]
fn test_opcode_14_lock_unlock_actor_frame() {
    let char_id: u32 = 1001; // 0x000003E9 -> LE: E9 03 00 00

    // Lock (Mode = 1)
    let lock_frame = NpcTalkCodec::build_talk_lock_frame(char_id, TalkLockMode::Lock);
    assert_eq!(lock_frame.len(), 11);
    assert_eq!(&lock_frame[0..4], &[0xF4, 0x44, 0x07, 0x00]); // Header + len 7
    assert_eq!(lock_frame[4], 0x14); // OP_NPC_EVENT
    assert_eq!(lock_frame[5], 0x2C); // SUB_LOCK_ACTOR
    assert_eq!(&lock_frame[6..10], &[0xE9, 0x03, 0x00, 0x00]);
    assert_eq!(lock_frame[10], 0x01); // Lock

    let lock_hex = NpcTalkCodec::build_talk_lock_hex(char_id, TalkLockMode::Lock);
    assert_eq!(lock_hex, "F4440700142CE903000001");

    // Unlock (Mode = 2)
    let unlock_frame = NpcTalkCodec::build_talk_lock_frame(char_id, TalkLockMode::Unlock);
    assert_eq!(unlock_frame[10], 0x02);
    let unlock_hex = NpcTalkCodec::build_talk_lock_hex(char_id, TalkLockMode::Unlock);
    assert_eq!(unlock_hex, "F4440700142CE903000002");
}

#[test]
fn test_opcode_14_end_talk_frame() {
    let frame = NpcTalkCodec::build_end_talk_frame();
    assert_eq!(frame, [0xF4, 0x44, 0x02, 0x00, 0x14, 0x08]);

    let hex = NpcTalkCodec::build_end_talk_hex();
    assert_eq!(hex, "F44402001408");
}

// ============================================================================
// Opcode 0x18: Quest Sync Wire Codec Tests
// ============================================================================

#[test]
fn test_opcode_18_item_add_remove() {
    let item_id: u16 = 10001; // 0x2711 -> LE: 11 27
    let count: u8 = 2;

    // SubOp 0x01: Add
    let add_frame = QuestSyncCodec::build_item_add_frame(item_id, count);
    assert_eq!(add_frame.len(), 9); // F4 44 05 00 18 01 11 27 02
    assert_eq!(QuestSyncCodec::build_item_add_hex(item_id, count), "F44405001801112702");

    // SubOp 0x02: Remove
    let remove_frame = QuestSyncCodec::build_item_remove_frame(item_id, 1);
    assert_eq!(remove_frame.len(), 9);
    assert_eq!(QuestSyncCodec::build_item_remove_hex(item_id, 1), "F44405001802112701");
}

#[test]
fn test_opcode_18_quest_full_and_clear() {
    // SubOp 0x03: Full toast
    let full_frame = QuestSyncCodec::build_quest_full_frame();
    assert_eq!(full_frame, [0xF4, 0x44, 0x02, 0x00, OP_QUEST_SYNC, SUB_QUEST_FULL]);
    assert_eq!(QuestSyncCodec::build_quest_full_hex(), "F44402001803");

    // SubOp 0x04: Clear
    let clear_frame = QuestSyncCodec::build_item_clear_frame(10001);
    assert_eq!(clear_frame.len(), 8); // F4 44 04 00 18 04 11 27
    assert_eq!(QuestSyncCodec::build_item_clear_hex(10001), "F444040018041127");
}

#[test]
fn test_opcode_18_quest_dont_single_and_bulk() {
    // SubOp 0x05: Single
    let single_hex = QuestSyncCodec::build_dont_single_hex(101, 1); // 101 = 0x0065 -> 65 00
    assert_eq!(single_hex, "F44405001805650001");

    // Deserialization test of single dont entry
    let payload = [0x65, 0x00, 0x01];
    let mut reader = PacketReader::new(&payload);
    let dont = reader.read_quest_dont_entry().expect("Must read dont entry");
    assert_eq!(dont.mark, 101);
    assert_eq!(dont.flag, 1);

    // SubOp 0x07: Bulk
    let entries = [
        QuestDontEntry { mark: 101, flag: 1 },
        QuestDontEntry { mark: 102, flag: 1 },
    ];
    let bulk_hex = QuestSyncCodec::build_dont_bulk_hex(&entries);
    // Payload length: 2 (op+sub) + 2*3 = 8 -> 0x08, 0x00
    assert_eq!(bulk_hex, "F44408001807650001660001");
}

#[test]
fn test_opcode_18_quest_task_single_and_bulk() {
    // SubOp 0x06: Single task entry (Payload 6 bytes: 18 06 01 31 2A 03)
    let single_hex = QuestSyncCodec::build_task_entry_hex(1, 10801, 3); // 10801 = 0x2A31 -> 31 2A
    assert_eq!(single_hex, "F4440600180601312A03");

    // Deserialization test of single task entry
    let payload = [0x01, 0x31, 0x2A, 0x03];
    let mut reader = PacketReader::new(&payload);
    let task = reader.read_quest_task_entry().expect("Must read task entry");
    assert_eq!(task.slot, 1);
    assert_eq!(task.quest_id, 10801);
    assert_eq!(task.mark_step, 3);

    // SubOp 0x06: Bulk task entries
    let entries = [
        QuestTaskEntry {
            slot: 1,
            quest_id: 10801,
            mark_step: 3,
        },
        QuestTaskEntry {
            slot: 2,
            quest_id: 10802,
            mark_step: 1,
        },
    ];
    let bulk_hex = QuestSyncCodec::build_task_bulk_hex(&entries);
    // Payload length: 2 (op+sub) + 2*4 = 10 -> 0x0A, 0x00
    assert_eq!(bulk_hex, "F4440A00180601312A0302322A01");
}

#[test]
fn test_opcode_18_actor_state_flag() {
    // SubOp 0x08: ActorStateFlag
    // CharID = 1001 (0xE9, 0x03, 0x00, 0x00), Kind = 1 (Bad Luck God: 0x01, 0x00), Flag = 0 (Gone)
    let flag_hex = QuestSyncCodec::build_state_flag_hex(1001, 1, 0);
    // Payload length: 2 (op+sub) + 4 (char) + 2 (kind) + 1 (flag) = 9 -> 0x09, 0x00
    assert_eq!(flag_hex, "F44409001808E9030000010000");
}

#[test]
fn test_opcode_18_bulk_decoding_roundtrip() {
    // Test bulk task log decoding
    let task_payload = [
        0x01, 0x31, 0x2A, 0x03, // Slot 1, Quest 10801, Step 3
        0x02, 0x32, 0x2A, 0x01, // Slot 2, Quest 10802, Step 1
    ];
    let mut reader = PacketReader::new(&task_payload);
    let mut tasks = Vec::new();
    while reader.remaining() >= 4 {
        tasks.push(reader.read_quest_task_entry().unwrap());
    }
    assert_eq!(tasks.len(), 2);
    assert_eq!(tasks[0].slot, 1);
    assert_eq!(tasks[0].quest_id, 10801);
    assert_eq!(tasks[0].mark_step, 3);
    assert_eq!(tasks[1].slot, 2);
    assert_eq!(tasks[1].quest_id, 10802);
    assert_eq!(tasks[1].mark_step, 1);
    assert_eq!(reader.remaining(), 0);

    // Test bulk quest dont decoding
    let dont_payload = [
        0x65, 0x00, 0x01, // Mark 101, Flag 1
        0x66, 0x00, 0x01, // Mark 102, Flag 1
    ];
    let mut reader2 = PacketReader::new(&dont_payload);
    let mut donts = Vec::new();
    while reader2.remaining() >= 3 {
        donts.push(reader2.read_quest_dont_entry().unwrap());
    }
    assert_eq!(donts.len(), 2);
    assert_eq!(donts[0].mark, 101);
    assert_eq!(donts[0].flag, 1);
    assert_eq!(donts[1].mark, 102);
    assert_eq!(donts[1].flag, 1);
    assert_eq!(reader2.remaining(), 0);
}

#[test]
fn test_opcode_14_talk_lock_mode_conversions() {
    assert_eq!(TalkLockMode::from_u8(0x01), Some(TalkLockMode::Lock));
    assert_eq!(TalkLockMode::from_u8(0x02), Some(TalkLockMode::Unlock));
    assert_eq!(TalkLockMode::from_u8(0x00), None);
    assert_eq!(TalkLockMode::from_u8(0x03), None);
}
