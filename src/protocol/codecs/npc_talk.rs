//! NPC dialogue and event wire codecs (Opcode 0x14).
//!
//! Handles serialization and deserialization of dialogue step frames (0x14 Sub 0x01),
//! actor lock/unlock frames (0x14 Sub 0x2C), and end-talk frames (0x14 Sub 0x08).

use crate::data::loaders::EveResult;
use crate::error::Result;
use crate::protocol::encoder;
use crate::protocol::reader::PacketReader;
use crate::protocol::writer::PacketWriter;
use crate::protocol::OP_NPC_EVENT;

/// Sub-opcodes for Opcode 0x14 (`OP_NPC_EVENT`).
pub const SUB_TALK_STEP: u8 = 0x01;
pub const SUB_END_TALK: u8 = 0x04; // C -> S end talk request
pub const SUB_TALK_CONTINUE: u8 = 0x06; // C -> S next step request
pub const SUB_CLOSE_TALK_WINDOW: u8 = 0x08; // S -> C close talk dialog form
pub const SUB_SELECT_MENU: u8 = 0x09; // C -> S menu option select
pub const SUB_LOCK_ACTOR: u8 = 0x2C; // S -> C lock/unlock character

/// Size of the raw EveResult binary payload in bytes.
pub const EVE_RESULT_SIZE: usize = 14;

/// Size of the full talk step body (Opcode + SubOp + Pad + 14B EveResult).
pub const TALK_STEP_BODY_SIZE: usize = 17;

/// Actor movement/action lock mode (`0x14 Sub 0x2C`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum TalkLockMode {
    /// Lock player movement and action when opening dialogue (`FUN_0071e288`).
    Lock = 0x01,
    /// Unlock player movement and action when ending dialogue (`FUN_0071f9ec`).
    Unlock = 0x02,
}

impl TalkLockMode {
    pub fn from_u8(val: u8) -> Option<Self> {
        match val {
            0x01 => Some(Self::Lock),
            0x02 => Some(Self::Unlock),
            _ => None,
        }
    }
}

/// Standalone codec for NPC dialogue and event frames (`OP_NPC_EVENT`).
pub struct NpcTalkCodec;

impl NpcTalkCodec {
    /// Encodes a 14-byte binary `EveResult` into a byte array matching `eve.emg` wire format:
    /// - `result_group_no` (2B LE)
    /// - `result_no` (1B)
    /// - `result_type` (1B)
    /// - `result_class` (1B)
    /// - `parameter` (2B LE)
    /// - `parameter_style` (1B)
    /// - `result_value` (4B LE)
    /// - `result_mean_no` (2B LE, Dialog ID)
    pub fn encode_eve_result(result: &EveResult) -> [u8; EVE_RESULT_SIZE] {
        let mut buf = [0u8; EVE_RESULT_SIZE];
        buf[0..2].copy_from_slice(&result.result_group_no.to_le_bytes());
        buf[2] = result.result_no;
        buf[3] = result.result_type;
        buf[4] = result.result_class;
        buf[5..7].copy_from_slice(&result.parameter.to_le_bytes());
        buf[7] = result.parameter_style;
        buf[8..12].copy_from_slice(&result.result_value.to_le_bytes());
        buf[12..14].copy_from_slice(&result.result_mean_no.to_le_bytes());
        buf
    }

    /// Decodes a 14-byte binary `EveResult` from a `PacketReader`.
    pub fn decode_eve_result(reader: &mut PacketReader<'_>) -> Result<EveResult> {
        let result_group_no = reader.read_u16_le()?;
        let result_no = reader.read_u8()?;
        let result_type = reader.read_u8()?;
        let result_class = reader.read_u8()?;
        let parameter = reader.read_u16_le()?;
        let parameter_style = reader.read_u8()?;
        let result_value = reader.read_i32_le()?;
        let result_mean_no = reader.read_u16_le()?;

        Ok(EveResult {
            result_group_no,
            result_no,
            result_type,
            result_class,
            parameter,
            parameter_style,
            result_value,
            result_mean_no,
        })
    }

    /// Builds a 17-byte dialogue step body: `[0x14, 0x01, 0x00, 14 bytes EveResult]`.
    pub fn build_talk_step_body(result: &EveResult) -> [u8; TALK_STEP_BODY_SIZE] {
        let mut body = [0u8; TALK_STEP_BODY_SIZE];
        body[0] = OP_NPC_EVENT;
        body[1] = SUB_TALK_STEP;
        body[2] = 0x00; // Fixed padding byte
        let res_bytes = Self::encode_eve_result(result);
        body[3..17].copy_from_slice(&res_bytes);
        body
    }

    /// Builds the decoded 21-byte frame for dialogue step:
    /// `F4 44 11 00 14 01 00 [14B EveResult]`
    pub fn build_talk_step_frame(result: &EveResult) -> Vec<u8> {
        let body = Self::build_talk_step_body(result);
        let mut writer = PacketWriter::with_capacity(TALK_STEP_BODY_SIZE);
        writer.write_bytes(&body);
        writer.build_frame()
    }

    /// Builds the hex string representation of the talk step frame.
    pub fn build_talk_step_hex(result: &EveResult) -> String {
        encoder::hex(&Self::build_talk_step_frame(result))
    }

    /// Builds the decoded 11-byte frame for lock/unlock actor:
    /// `F4 44 07 00 14 2C [CharID: 4B LE] [Mode: 1B]`
    pub fn build_talk_lock_frame(char_id: u32, mode: TalkLockMode) -> Vec<u8> {
        let mut writer = PacketWriter::new(OP_NPC_EVENT, SUB_LOCK_ACTOR);
        writer.write_u32_le(char_id);
        writer.write_u8(mode as u8);
        writer.build_frame()
    }

    /// Builds the hex string representation of lock/unlock actor frame.
    pub fn build_talk_lock_hex(char_id: u32, mode: TalkLockMode) -> String {
        encoder::hex(&Self::build_talk_lock_frame(char_id, mode))
    }

    /// Builds the decoded 6-byte frame for closing talk dialog UI:
    /// `F4 44 02 00 14 08`
    pub fn build_end_talk_frame() -> [u8; 6] {
        [0xF4, 0x44, 0x02, 0x00, OP_NPC_EVENT, SUB_CLOSE_TALK_WINDOW]
    }

    /// Hex string representation of the end talk frame (`F44402001408`).
    pub fn build_end_talk_hex() -> &'static str {
        "F44402001408"
    }
}
