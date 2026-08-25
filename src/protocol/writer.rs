//! Fluent binary packet writer.
//!
//! Provides a byte buffer builder for constructing TS Online binary packets,
//! with Little-Endian primitive serialization, VISCII 1.1 string encoding,
//! and standard `F4 44` framing with `0xAD` XOR wire encryption.

use crate::encoding::viscii_encode;
use crate::protocol::{MAGIC, XOR_KEY};

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
        out.extend_from_slice(&MAGIC);
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::reader::PacketReader;

    #[test]
    fn test_write_primitives() {
        let mut writer = PacketWriter::empty();
        writer
            .write_u8(0x12)
            .write_u16_le(0x1234)
            .write_u32_le(0x12345678)
            .write_u64_le(0x8000000000000001)
            .write_i8(-2)
            .write_i16_le(-16)
            .write_i32_le(-65536)
            .write_f64_le(12345.6789)
            .write_bool(true)
            .write_bool(false);

        let mut reader = PacketReader::new(writer.as_slice());
        assert_eq!(reader.read_u8().unwrap(), 0x12);
        assert_eq!(reader.read_u16_le().unwrap(), 0x1234);
        assert_eq!(reader.read_u32_le().unwrap(), 0x12345678);
        assert_eq!(reader.read_u64_le().unwrap(), 0x8000000000000001);
        assert_eq!(reader.read_i8().unwrap(), -2);
        assert_eq!(reader.read_i16_le().unwrap(), -16);
        assert_eq!(reader.read_i32_le().unwrap(), -65536);
        assert_eq!(reader.read_f64_le().unwrap(), 12345.6789);
        assert!(reader.read_bool().unwrap());
        assert!(!reader.read_bool().unwrap());
        assert_eq!(reader.remaining(), 0);
    }

    #[test]
    fn test_write_viscii_pascal() {
        let mut writer = PacketWriter::empty();
        writer.write_viscii_pascal("TSVN");
        assert_eq!(writer.as_slice(), &[4, 0x54, 0x53, 0x56, 0x4E]);

        let mut reader = PacketReader::new(writer.as_slice());
        assert_eq!(reader.read_viscii_pascal().unwrap(), "TSVN");

        // Vietnamese text
        let mut writer_vn = PacketWriter::empty();
        writer_vn.write_viscii_pascal("Thời gian:");
        let mut reader_vn = PacketReader::new(writer_vn.as_slice());
        assert_eq!(reader_vn.read_viscii_pascal().unwrap(), "Thời gian:");
    }

    #[test]
    fn test_write_viscii_fixed() {
        // Short text padded to fixed len
        let mut writer = PacketWriter::empty();
        writer.write_viscii_fixed("TS", 6);
        assert_eq!(writer.as_slice(), &[0x54, 0x53, 0x00, 0x00, 0x00, 0x00]);

        let mut reader = PacketReader::new(writer.as_slice());
        assert_eq!(reader.read_viscii_fixed(6).unwrap(), "TS");

        // Truncated text
        let mut writer_trunc = PacketWriter::empty();
        writer_trunc.write_viscii_fixed("TSOnline", 4);
        assert_eq!(writer_trunc.as_slice(), &[0x54, 0x53, 0x4F, 0x6E]);
    }

    #[test]
    fn test_build_frame_and_wire() {
        // Opcode 0x03, Sub 0x01 with no extra payload -> body len = 2
        let writer = PacketWriter::new(0x03, 0x01);
        let frame = writer.build_frame();
        assert_eq!(frame, vec![0xF4, 0x44, 0x02, 0x00, 0x03, 0x01]);

        let wire = writer.build_wire();
        let expected_wire: Vec<u8> = frame.iter().map(|b| b ^ 0xAD).collect();
        assert_eq!(wire, expected_wire);

        // Opcode only
        let writer_op = PacketWriter::opcode(0x0A);
        let frame_op = writer_op.build_frame();
        assert_eq!(frame_op, vec![0xF4, 0x44, 0x01, 0x00, 0x0A]);
    }

    #[test]
    fn test_writer_methods() {
        let mut writer = PacketWriter::new(0x06, 0x01);
        assert_eq!(writer.len(), 2);
        assert!(!writer.is_empty());

        writer.write_bytes(&[0xAA, 0xBB]);
        assert_eq!(writer.len(), 4);
        assert_eq!(writer.as_slice(), &[0x06, 0x01, 0xAA, 0xBB]);

        let bytes = writer.into_bytes();
        assert_eq!(bytes, vec![0x06, 0x01, 0xAA, 0xBB]);
    }

    #[test]
    fn test_empty_string_and_default() {
        let mut writer = PacketWriter::default();
        assert!(writer.is_empty());
        assert_eq!(writer.len(), 0);

        writer.write_viscii_pascal("");
        assert_eq!(writer.as_slice(), &[0]); // 1-byte length prefix 0
        assert_eq!(writer.len(), 1);

        let mut reader = PacketReader::new(writer.as_slice());
        assert_eq!(reader.read_viscii_pascal().unwrap(), "");
    }

    #[test]
    fn test_wire_xor_parity() {
        // Build frame with opcode 0x17 sub 0x05
        let writer = PacketWriter::new(0x17, 0x05);
        let frame = writer.build_frame();
        assert_eq!(frame, vec![0xF4, 0x44, 0x02, 0x00, 0x17, 0x05]);

        let wire = writer.build_wire();
        // Decode XOR 0xAD again should equal frame
        let decoded: Vec<u8> = wire.iter().map(|b| b ^ XOR_KEY).collect();
        assert_eq!(decoded, frame);
    }
}
