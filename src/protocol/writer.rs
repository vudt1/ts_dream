//! Fluent binary packet writer.
//!
//! Provides a byte buffer builder for constructing TS Online binary packets,
//! with Little-Endian primitive serialization, VISCII 1.1 string encoding,
//! and standard `F4 44` framing with `0xAD` XOR wire encryption.

use crate::encoding::viscii_encode;
use crate::protocol::{HEADER_TS_MAGIC, XOR_KEY};

/// Fluent binary packet buffer builder.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct PacketWriter {
    buf: Vec<u8>,
}

impl PacketWriter {
    /// Create a new `PacketWriter` initialized with opcode and subcode (`[opcode, sub]`).
    pub fn new(opcode: u8, sub: u8) -> Self {
        let mut w = Self::with_capacity(32);
        w.write_u8(opcode);
        w.write_u8(sub);
        w
    }

    /// Create a new `PacketWriter` initialized with a single opcode byte (`[opcode]`).
    pub fn opcode(opcode: u8) -> Self {
        let mut w = Self::with_capacity(32);
        w.write_u8(opcode);
        w
    }

    /// Create an empty `PacketWriter` with the given capacity.
    pub fn with_capacity(cap: usize) -> Self {
        Self {
            buf: Vec::with_capacity(cap),
        }
    }

    /// Create an empty `PacketWriter`.
    pub fn empty() -> Self {
        Self { buf: Vec::new() }
    }

    /// Write an unsigned 8-bit integer.
    pub fn write_u8(&mut self, v: u8) -> &mut Self {
        self.buf.push(v);
        self
    }

    /// Write an unsigned 16-bit integer in Little-Endian.
    pub fn write_u16_le(&mut self, v: u16) -> &mut Self {
        self.buf.extend_from_slice(&v.to_le_bytes());
        self
    }

    /// Write an unsigned 32-bit integer in Little-Endian.
    pub fn write_u32_le(&mut self, v: u32) -> &mut Self {
        self.buf.extend_from_slice(&v.to_le_bytes());
        self
    }

    /// Write an unsigned 64-bit integer in Little-Endian.
    pub fn write_u64_le(&mut self, v: u64) -> &mut Self {
        self.buf.extend_from_slice(&v.to_le_bytes());
        self
    }

    /// Write a signed 8-bit integer.
    pub fn write_i8(&mut self, v: i8) -> &mut Self {
        self.buf.push(v as u8);
        self
    }

    /// Write a signed 16-bit integer in Little-Endian.
    pub fn write_i16_le(&mut self, v: i16) -> &mut Self {
        self.buf.extend_from_slice(&v.to_le_bytes());
        self
    }

    /// Write a signed 32-bit integer in Little-Endian.
    pub fn write_i32_le(&mut self, v: i32) -> &mut Self {
        self.buf.extend_from_slice(&v.to_le_bytes());
        self
    }

    /// Write a 64-bit floating point number in Little-Endian.
    pub fn write_f64_le(&mut self, v: f64) -> &mut Self {
        self.buf.extend_from_slice(&v.to_le_bytes());
        self
    }

    /// Write a boolean flag (1 byte: `1` for `true`, `0` for `false`).
    pub fn write_bool(&mut self, v: bool) -> &mut Self {
        self.write_u8(if v { 1 } else { 0 })
    }

    /// Write raw bytes to the buffer.
    pub fn write_bytes(&mut self, bytes: &[u8]) -> &mut Self {
        self.buf.extend_from_slice(bytes);
        self
    }

    /// Write a Pascal-style VISCII string (1-byte length prefix + VISCII 1.1 encoded bytes).
    pub fn write_viscii_pascal(&mut self, text: &str) -> &mut Self {
        let bytes = viscii_encode(text);
        let len = bytes.len().min(255) as u8;
        self.write_u8(len);
        self.write_bytes(&bytes[..len as usize]);
        self
    }

    /// Write a fixed-length VISCII string (truncated or null-padded to exact `len` bytes).
    pub fn write_viscii_fixed(&mut self, text: &str, len: usize) -> &mut Self {
        let bytes = viscii_encode(text);
        if bytes.len() >= len {
            self.write_bytes(&bytes[..len]);
        } else {
            self.write_bytes(&bytes);
            let padding = len - bytes.len();
            for _ in 0..padding {
                self.write_u8(0);
            }
        }
        self
    }

    /// Write a 35-byte `ThingData` struct.
    pub fn write_thing_data(&mut self, thing: &crate::protocol::codecs::ThingData) -> &mut Self {
        thing.encode(self);
        self
    }

    /// Write a `PlayerCard` struct.
    pub fn write_player_card(&mut self, card: &crate::protocol::codecs::PlayerCard) -> &mut Self {
        card.encode(self);
        self
    }

    /// Write a 20-byte `FriendExtra` struct.
    pub fn write_friend_extra(
        &mut self,
        extra: &crate::protocol::codecs::FriendExtra,
    ) -> &mut Self {
        extra.encode(self);
        self
    }

    /// Write a `BattleRoleData` struct.
    pub fn write_battle_role(
        &mut self,
        role: &crate::protocol::codecs::BattleRoleData,
    ) -> &mut Self {
        role.encode(self);
        self
    }

    /// Write a 14-byte `EveResult` struct to the buffer.
    pub fn write_eve_result(
        &mut self,
        result: &crate::data::loaders::EveResult,
    ) -> &mut Self {
        let bytes = crate::protocol::codecs::NpcTalkCodec::encode_eve_result(result);
        self.write_bytes(&bytes);
        self
    }

    /// Write a 4-byte `QuestTaskEntry` to the buffer.
    pub fn write_quest_task_entry(
        &mut self,
        entry: &crate::protocol::codecs::QuestTaskEntry,
    ) -> &mut Self {
        self.write_u8(entry.slot);
        self.write_u16_le(entry.quest_id);
        self.write_u8(entry.mark_step);
        self
    }

    /// Write a 3-byte `QuestDontEntry` to the buffer.
    pub fn write_quest_dont_entry(
        &mut self,
        entry: &crate::protocol::codecs::QuestDontEntry,
    ) -> &mut Self {
        self.write_u16_le(entry.mark);
        self.write_u8(entry.flag);
        self
    }

    /// Returns a slice of the internal body buffer.
    pub fn as_slice(&self) -> &[u8] {
        &self.buf
    }

    /// Returns the length of the internal body buffer in bytes.
    pub fn len(&self) -> usize {
        self.buf.len()
    }

    /// Returns `true` if the internal body buffer is empty.
    pub fn is_empty(&self) -> bool {
        self.buf.is_empty()
    }

    /// Consume into inner byte vector.
    pub fn into_bytes(self) -> Vec<u8> {
        self.buf
    }

    /// Build a complete decoded frame: header `F4 44` + 2B LE length + body bytes.
    pub fn build_frame(&self) -> Vec<u8> {
        let body_len = self.buf.len() as u16;
        let mut out = Vec::with_capacity(4 + self.buf.len());
        out.extend_from_slice(&HEADER_TS_MAGIC);
        out.extend_from_slice(&body_len.to_le_bytes());
        out.extend_from_slice(&self.buf);
        out
    }

    /// Build the on-wire encrypted frame (entire frame XOR'd with `0xAD`).
    pub fn build_wire(&self) -> Vec<u8> {
        let frame = self.build_frame();
        frame.into_iter().map(|b| b ^ XOR_KEY).collect()
    }
}
