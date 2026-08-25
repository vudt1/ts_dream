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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_read_primitives() {
        let data = [
            0x12,                   // u8
            0x34, 0x12,             // u16: 0x1234
            0x78, 0x56, 0x34, 0x12, // u32: 0x12345678
            0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x80, // u64: 0x8000000000000001
            0xFE,                   // i8: -2
            0xF0, 0xFF,             // i16: -16
            0x00, 0x00, 0xFF, 0xFF, // i32: -65536
            0x01,                   // bool: true
            0x00,                   // bool: false
        ];
        let mut reader = PacketReader::new(&data);

        assert_eq!(reader.read_u8().unwrap(), 0x12);
        assert_eq!(reader.read_u16_le().unwrap(), 0x1234);
        assert_eq!(reader.read_u32_le().unwrap(), 0x12345678);
        assert_eq!(reader.read_u64_le().unwrap(), 0x8000000000000001);
        assert_eq!(reader.read_i8().unwrap(), -2);
        assert_eq!(reader.read_i16_le().unwrap(), -16);
        assert_eq!(reader.read_i32_le().unwrap(), -65536);
        assert!(reader.read_bool().unwrap());
        assert!(!reader.read_bool().unwrap());
        assert_eq!(reader.remaining(), 0);
        assert!(!reader.is_readable());
    }

    #[test]
    fn test_read_f64() {
        let val = 12345.6789f64;
        let bytes = val.to_le_bytes();
        let mut reader = PacketReader::new(&bytes);
        assert_eq!(reader.read_f64_le().unwrap(), val);
    }

    #[test]
    fn test_read_bytes_zero_copy() {
        let data = b"Hello, world!";
        let mut reader = PacketReader::new(data);
        let part1 = reader.read_bytes(5).unwrap();
        assert_eq!(part1, b"Hello");
        reader.skip(2).unwrap(); // skip ", "
        let part2 = reader.read_bytes(6).unwrap();
        assert_eq!(part2, b"world!");
        assert_eq!(reader.remaining(), 0);
    }

    #[test]
    fn test_read_viscii_pascal() {
        // "TSVN" -> length 4 + bytes
        let data = [4, 0x54, 0x53, 0x56, 0x4E];
        let mut reader = PacketReader::new(&data);
        assert_eq!(reader.read_viscii_pascal().unwrap(), "TSVN");

        // VISCII Vietnamese: Đ (0xD0), ấ (0xA4), ă (0xE5), ỏ (0xF6)
        let data_vn = [4, 0xD0, 0xA4, 0xE5, 0xF6];
        let mut reader_vn = PacketReader::new(&data_vn);
        assert_eq!(reader_vn.read_viscii_pascal().unwrap(), "Đấăỏ");
    }

    #[test]
    fn test_read_viscii_fixed() {
        // Fixed 8 bytes with null padding: "TS" + 6 nulls
        let data = [0x54, 0x53, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00];
        let mut reader = PacketReader::new(&data);
        assert_eq!(reader.read_viscii_fixed(8).unwrap(), "TS");

        // Fixed 4 bytes full (no nulls): "TSVN"
        let data_full = [0x54, 0x53, 0x56, 0x4E];
        let mut reader_full = PacketReader::new(&data_full);
        assert_eq!(reader_full.read_viscii_fixed(4).unwrap(), "TSVN");
    }

    #[test]
    fn test_bounds_checking_errors() {
        let data = [0x01, 0x02];
        let mut reader = PacketReader::new(&data);

        // Attempting to read u32 (4 bytes) when only 2 remain
        assert!(reader.read_u32_le().is_err());
        assert_eq!(reader.position(), 0);

        // Attempting to skip past end
        assert!(reader.skip(5).is_err());

        // Attempting to set position out of bounds
        assert!(reader.set_position(10).is_err());

        // Valid read then EOF
        assert_eq!(reader.read_u8().unwrap(), 0x01);
        assert_eq!(reader.read_u8().unwrap(), 0x02);
        assert!(reader.read_u8().is_err());
    }

    #[test]
    fn test_boundary_integer_values() {
        let mut buf = Vec::new();
        buf.push(u8::MAX);
        buf.extend_from_slice(&u16::MAX.to_le_bytes());
        buf.extend_from_slice(&u32::MAX.to_le_bytes());
        buf.extend_from_slice(&u64::MAX.to_le_bytes());
        buf.push(i8::MIN as u8);
        buf.push(i8::MAX as u8);
        buf.extend_from_slice(&i16::MIN.to_le_bytes());
        buf.extend_from_slice(&i16::MAX.to_le_bytes());
        buf.extend_from_slice(&i32::MIN.to_le_bytes());
        buf.extend_from_slice(&i32::MAX.to_le_bytes());

        let mut reader = PacketReader::new(&buf);
        assert_eq!(reader.read_u8().unwrap(), u8::MAX);
        assert_eq!(reader.read_u16_le().unwrap(), u16::MAX);
        assert_eq!(reader.read_u32_le().unwrap(), u32::MAX);
        assert_eq!(reader.read_u64_le().unwrap(), u64::MAX);
        assert_eq!(reader.read_i8().unwrap(), i8::MIN);
        assert_eq!(reader.read_i8().unwrap(), i8::MAX);
        assert_eq!(reader.read_i16_le().unwrap(), i16::MIN);
        assert_eq!(reader.read_i16_le().unwrap(), i16::MAX);
        assert_eq!(reader.read_i32_le().unwrap(), i32::MIN);
        assert_eq!(reader.read_i32_le().unwrap(), i32::MAX);
        assert_eq!(reader.remaining(), 0);
    }

    #[test]
    fn test_empty_string_and_seek() {
        let data = [0x00, 0x01, 0x02, 0x03];
        let mut reader = PacketReader::new(&data);

        // Empty pascal string (length 0)
        assert_eq!(reader.read_viscii_pascal().unwrap(), "");
        assert_eq!(reader.position(), 1);

        // Seek forward and back
        reader.set_position(3).unwrap();
        assert_eq!(reader.read_u8().unwrap(), 0x03);
        reader.set_position(1).unwrap();
        assert_eq!(reader.read_u16_le().unwrap(), 0x0201);
    }
}
