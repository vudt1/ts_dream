//! Zero-copy binary packet reader.
//!
//! Provides cursor-based reading over raw byte slices with bounds checking,
//! Little-Endian primitive parsing, and VISCII 1.1 string decoding.

use crate::encoding::viscii_to_unicode;
use crate::error::{Result, TsError};

/// Zero-copy cursor over a binary packet slice.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PacketReader<'a> {
    data: &'a [u8],
    pos: usize,
}

impl<'a> PacketReader<'a> {
    /// Create a new `PacketReader` over the given byte slice.
    pub fn new(data: &'a [u8]) -> Self {
        Self { data, pos: 0 }
    }

    /// Current cursor position in bytes.
    pub fn position(&self) -> usize {
        self.pos
    }

    /// Set the cursor position with bounds checking.
    pub fn set_position(&mut self, pos: usize) -> Result<()> {
        if pos > self.data.len() {
            return Err(TsError::Protocol(format!(
                "cannot set position to {pos}: total length is {}",
                self.data.len()
            )));
        }
        self.pos = pos;
        Ok(())
    }

    /// Number of remaining bytes available to read.
    pub fn remaining(&self) -> usize {
        self.data.len().saturating_sub(self.pos)
    }

    /// Returns `true` if there are still unread bytes available.
    pub fn is_readable(&self) -> bool {
        self.pos < self.data.len()
    }

    /// Skip `len` bytes ahead with bounds checking.
    pub fn skip(&mut self, len: usize) -> Result<()> {
        if self.pos + len > self.data.len() {
            return Err(TsError::Protocol(format!(
                "cannot skip {len} bytes: only {} remaining",
                self.remaining()
            )));
        }
        self.pos += len;
        Ok(())
    }

    /// Read an unsigned 8-bit integer.
    pub fn read_u8(&mut self) -> Result<u8> {
        let bytes = self.read_bytes(1)?;
        Ok(bytes[0])
    }

    /// Read an unsigned 16-bit integer in Little-Endian.
    pub fn read_u16_le(&mut self) -> Result<u16> {
        let bytes = self.read_bytes(2)?;
        Ok(u16::from_le_bytes([bytes[0], bytes[1]]))
    }

    /// Read an unsigned 32-bit integer in Little-Endian.
    pub fn read_u32_le(&mut self) -> Result<u32> {
        let bytes = self.read_bytes(4)?;
        Ok(u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
    }

    /// Read an unsigned 64-bit integer in Little-Endian.
    pub fn read_u64_le(&mut self) -> Result<u64> {
        let bytes = self.read_bytes(8)?;
        Ok(u64::from_le_bytes([
            bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7],
        ]))
    }

    /// Read a signed 8-bit integer.
    pub fn read_i8(&mut self) -> Result<i8> {
        let bytes = self.read_bytes(1)?;
        Ok(bytes[0] as i8)
    }

    /// Read a signed 16-bit integer in Little-Endian.
    pub fn read_i16_le(&mut self) -> Result<i16> {
        let bytes = self.read_bytes(2)?;
        Ok(i16::from_le_bytes([bytes[0], bytes[1]]))
    }

    /// Read a signed 32-bit integer in Little-Endian.
    pub fn read_i32_le(&mut self) -> Result<i32> {
        let bytes = self.read_bytes(4)?;
        Ok(i32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
    }

    /// Read a 64-bit floating point number in Little-Endian.
    pub fn read_f64_le(&mut self) -> Result<f64> {
        let bytes = self.read_bytes(8)?;
        Ok(f64::from_le_bytes([
            bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7],
        ]))
    }

    /// Read a boolean flag (1 byte: `0` -> `false`, non-zero -> `true`).
    pub fn read_bool(&mut self) -> Result<bool> {
        Ok(self.read_u8()? != 0)
    }

    /// Read a zero-copy sub-slice of `len` bytes.
    pub fn read_bytes(&mut self, len: usize) -> Result<&'a [u8]> {
        if self.pos + len > self.data.len() {
            return Err(TsError::Protocol(format!(
                "cannot read {len} bytes: only {} remaining",
                self.remaining()
            )));
        }
        let slice = &self.data[self.pos..self.pos + len];
        self.pos += len;
        Ok(slice)
    }

    /// Read a Pascal-style VISCII string (1-byte length prefix followed by VISCII 1.1 bytes).
    pub fn read_viscii_pascal(&mut self) -> Result<String> {
        let len = self.read_u8()? as usize;
        let bytes = self.read_bytes(len)?;
        Ok(bytes.iter().map(|&b| viscii_to_unicode(b)).collect())
    }

    /// Read a fixed-length VISCII string (reads `len` bytes, stops at first null byte if present).
    pub fn read_viscii_fixed(&mut self, len: usize) -> Result<String> {
        let bytes = self.read_bytes(len)?;
        let content = match bytes.iter().position(|&b| b == 0) {
            Some(pos) => &bytes[..pos],
            None => bytes,
        };
        Ok(content.iter().map(|&b| viscii_to_unicode(b)).collect())
    }

    /// Read a 35-byte `ThingData` struct from the stream.
    pub fn read_thing_data(&mut self) -> Result<crate::protocol::codecs::ThingData> {
        crate::protocol::codecs::ThingData::decode(self)
    }

    /// Read a `PlayerCard` struct from the stream.
    pub fn read_player_card(&mut self) -> Result<crate::protocol::codecs::PlayerCard> {
        crate::protocol::codecs::PlayerCard::decode(self)
    }

    /// Read a 20-byte `FriendExtra` struct from the stream.
    pub fn read_friend_extra(&mut self) -> Result<crate::protocol::codecs::FriendExtra> {
        crate::protocol::codecs::FriendExtra::decode(self)
    }

    /// Read a `BattleRoleData` struct from the stream.
    pub fn read_battle_role(&mut self) -> Result<crate::protocol::codecs::BattleRoleData> {
        crate::protocol::codecs::BattleRoleData::decode(self)
    }
}
