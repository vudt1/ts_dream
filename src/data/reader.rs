//! Binary `.Dat` reader for TS Online client and server data files.
//!
//! Provides Little-Endian numeric reading with optional XOR decryption
//! and number offsets, PC-style reversed string reading (VISCII/Big5),
//! Unicode UTF-16LE strings, and `DecodeAll` block decryption.

use crate::encoding;

/// A byte-oriented reader for TS Online `.Dat` binary files.
#[derive(Debug, Clone)]
pub struct DatReader {
    data: Vec<u8>,
    position: usize,
    number_offset: i32,
    xor1: u32,
    xor2: u32,
    xor4: u32,
}

impl DatReader {
    /// Create a new `DatReader` with raw bytes and no XOR/offset decryption.
    pub fn new(data: Vec<u8>) -> Self {
        Self {
            data,
            position: 0,
            number_offset: 0,
            xor1: 0,
            xor2: 0,
            xor4: 0,
        }
    }

    /// Create a new `DatReader` with decryption parameters.
    pub fn with_keys(
        data: Vec<u8>,
        number_offset: i32,
        xor1: u32,
        xor2: u32,
        xor4: u32,
    ) -> Self {
        Self {
            data,
            position: 0,
            number_offset,
            xor1,
            xor2,
            xor4,
        }
    }

    /// Returns true if more bytes are available to read.
    pub fn can_read(&self) -> bool {
        self.position < self.data.len()
    }

    /// Returns the current read position.
    pub fn position(&self) -> usize {
        self.position
    }

    /// Returns the total length in bytes.
    pub fn len(&self) -> usize {
        self.data.len()
    }

    /// Returns true if the underlying data is empty.
    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    /// Returns the number of unread bytes remaining.
    pub fn remaining(&self) -> usize {
        self.data.len().saturating_sub(self.position)
    }

    /// Move the read pointer to `pos`.
    pub fn seek(&mut self, pos: usize) {
        self.position = pos.min(self.data.len());
    }

    /// Read raw slice of `size` bytes.
    pub fn read_bytes(&mut self, size: usize) -> Vec<u8> {
        if self.position + size > self.data.len() {
            let available = self.data.len().saturating_sub(self.position);
            let slice = self.data[self.position..self.position + available].to_vec();
            self.position = self.data.len();
            return slice;
        }
        let slice = self.data[self.position..self.position + size].to_vec();
        self.position += size;
        slice
    }

    /// Helper to read numbers with optional XOR, offset, and sign conversion.
    fn read_number(&mut self, size: usize, compute_xor: bool, compute_offset: bool) -> i64 {
        if self.position + size > self.data.len() {
            return 0;
        }

        let mut result: u64 = 0;
        for i in 0..size {
            result |= (self.data[self.position + i] as u64) << (i * 8);
        }
        self.position += size;

        if compute_xor {
            match size {
                1 => {
                    if self.xor1 != 0 {
                        result ^= self.xor1 as u64;
                    }
                }
                2 => {
                    if self.xor2 != 0 {
                        result ^= self.xor2 as u64;
                    }
                }
                4 => {
                    if self.xor4 != 0 {
                        result ^= self.xor4 as u64;
                    }
                }
                _ => {}
            }
        }

        let mut signed_res = result as i64;
        if compute_offset && self.number_offset != 0 {
            signed_res -= self.number_offset as i64;
        }

        signed_res
    }

    /// Read unsigned 1 byte (u8).
    pub fn read_u8(&mut self) -> u8 {
        self.read_number(1, true, true) as u8
    }

    /// Read unsigned 1 byte without XOR or offset.
    pub fn read_u8_raw(&mut self) -> u8 {
        self.read_number(1, false, false) as u8
    }

    /// Read unsigned 1 byte (alias for read_u8).
    pub fn read_byte(&mut self) -> u8 {
        self.read_u8()
    }

    /// Read signed 1 byte (i8).
    pub fn read_i8(&mut self) -> i8 {
        self.read_number(1, true, true) as i8
    }

    /// Read unsigned 2 bytes Little-Endian (u16).
    pub fn read_u16(&mut self) -> u16 {
        self.read_number(2, true, true) as u16
    }

    /// Read unsigned 2 bytes Little-Endian without XOR or offset.
    pub fn read_u16_raw(&mut self) -> u16 {
        self.read_number(2, false, false) as u16
    }

    /// Read unsigned 2 bytes (alias for read_u16).
    pub fn read_uint16(&mut self) -> u16 {
        self.read_u16()
    }

    /// Read signed 2 bytes Little-Endian (i16).
    pub fn read_i16(&mut self) -> i16 {
        self.read_number(2, true, true) as i16
    }

    /// Read signed 2 bytes (alias for read_i16).
    pub fn read_int16(&mut self) -> i16 {
        self.read_i16()
    }

    /// Read unsigned 4 bytes Little-Endian (u32).
    pub fn read_u32(&mut self) -> u32 {
        self.read_number(4, true, true) as u32
    }

    /// Read unsigned 4 bytes Little-Endian without XOR or offset.
    pub fn read_u32_raw(&mut self) -> u32 {
        self.read_number(4, false, false) as u32
    }

    /// Read unsigned 4 bytes (alias for read_u32).
    pub fn read_uint32(&mut self) -> u32 {
        self.read_u32()
    }

    /// Read signed 4 bytes Little-Endian (i32).
    pub fn read_i32(&mut self) -> i32 {
        self.read_number(4, true, true) as i32
    }

    /// Read signed 4 bytes (alias for read_i32).
    pub fn read_int32(&mut self) -> i32 {
        self.read_i32()
    }

    /// Read IEEE 754 8-byte double precision float Little-Endian (f64).
    pub fn read_f64(&mut self) -> f64 {
        if self.position + 8 > self.data.len() {
            return 0.0;
        }
        let bytes: [u8; 8] = self.data[self.position..self.position + 8]
            .try_into()
            .unwrap_or([0; 8]);
        self.position += 8;
        f64::from_le_bytes(bytes)
    }

    /// Read double (alias for read_f64).
    pub fn read_double(&mut self) -> f64 {
        self.read_f64()
    }

    /// Read boolean (1 byte == 1).
    pub fn read_bool(&mut self) -> bool {
        self.read_u8() == 1
    }

    /// Read PC-style string raw bytes: [count: 1B] [padding...] [reversed bytes].
    /// Total bytes consumed = 1 + size.
    pub fn read_string_bytes(&mut self, size: usize) -> Vec<u8> {
        if self.position + 1 + size > self.data.len() {
            return Vec::new();
        }

        let count = self.data[self.position] as usize;
        self.position += 1;

        let result = if count > 0 && count <= size {
            let start = self.position + size - count;
            let mut raw = self.data[start..start + count].to_vec();
            raw.reverse();
            raw
        } else {
            Vec::new()
        };

        self.position += size;
        result
    }

    /// Read PC-style string and decode to Unicode string (using VISCII/Latin-1 mapping).
    pub fn read_string(&mut self, size: usize) -> String {
        let raw = self.read_string_bytes(size);
        if raw.is_empty() {
            return String::new();
        }
        raw.iter()
            .map(|&b| encoding::viscii_to_unicode(b))
            .collect()
    }

    /// Read fixed-length ASCII string without reversal: [count: 1B] [ascii data: size bytes].
    pub fn read_fixed_ascii(&mut self, size: usize) -> String {
        if self.position + 1 + size > self.data.len() {
            return String::new();
        }
        let count = self.data[self.position] as usize;
        self.position += 1;

        let result = if count > 0 && count <= size {
            String::from_utf8_lossy(&self.data[self.position..self.position + count]).to_string()
        } else {
            String::new()
        };
        self.position += size;
        result
    }

    /// Read mobile-style Unicode string: [len: 2B LE] [UTF-16LE bytes].
    pub fn read_unicode_string(&mut self) -> String {
        let byte_len = self.read_u16_raw() as usize;
        if byte_len == 0 || self.position + byte_len > self.data.len() {
            return String::new();
        }
        let raw = &self.data[self.position..self.position + byte_len];
        self.position += byte_len;

        let u16s: Vec<u16> = raw
            .chunks_exact(2)
            .map(|c| u16::from_le_bytes([c[0], c[1]]))
            .collect();
        String::from_utf16_lossy(&u16s)
    }

    /// Decrypt entire buffer using TS Online block-decoding algorithm (`DecodeAll`).
    ///
    /// If `size` is `None`, reads a 4-byte LE integer for size.
    /// If `count` is `None`, reads a 4-byte LE integer for count.
    pub fn decode_all(&mut self, size_opt: Option<usize>, count_opt: Option<usize>) {
        self.position = 0;

        let size = match size_opt {
            Some(s) => s,
            None => self.read_u32_raw() as usize,
        };

        let count = match count_opt {
            Some(c) => c,
            None => self.read_u32_raw() as usize,
        };

        if size == 0 || count == 0 {
            return;
        }

        let pos = self.position;
        let key0_offset = pos + size * (count + 1);
        let key1_offset = pos + size * (count + 2);

        if key1_offset + size > self.data.len() {
            return;
        }

        let key0 = self.data[key0_offset..key0_offset + size].to_vec();
        let key1 = self.data[key1_offset..key1_offset + size].to_vec();

        let mut decoded = Vec::with_capacity(size * count);
        for i in 1..=count {
            let block_offset = pos + size * i;
            if block_offset + size > self.data.len() {
                break;
            }
            let block = &self.data[block_offset..block_offset + size];
            let key = if i % 2 == 1 { &key1 } else { &key0 };
            for j in 0..size {
                decoded.push(block[j] ^ key[j]);
            }
        }

        self.data = decoded;
        self.position = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_numeric_reads() {
        let bytes = vec![0x12, 0x34, 0x56, 0x78, 0x9A, 0xBC, 0xDE, 0xF0];
        let mut reader = DatReader::new(bytes);
        assert_eq!(reader.read_u8(), 0x12);
        assert_eq!(reader.read_u16(), 0x5634);
        assert_eq!(reader.read_u32(), 0xDEBC9A78);
    }

    #[test]
    fn test_offset_and_xor() {
        // Test with offset = 3, xor1 = 211
        let val: u8 = 10;
        let encoded: u8 = (val + 3) ^ 211;
        let mut reader = DatReader::with_keys(vec![encoded], 3, 211, 0, 0);
        assert_eq!(reader.read_u8(), 10);
    }

    #[test]
    fn test_pc_reversed_string() {
        // 1 byte count=4, 6 bytes buffer, reversed string at the end
        // String "ABCD" -> bytes [4, padding: 0x00, 0x00, 'D', 'C', 'B', 'A']
        let bytes = vec![4, 0x00, 0x00, b'D', b'C', b'B', b'A'];
        let mut reader = DatReader::new(bytes);
        let s = reader.read_string(6);
        assert_eq!(s, "ABCD");
    }
}
