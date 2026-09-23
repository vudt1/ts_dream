//! Loader for `Mark.Dat` (PC binary, 516 bytes/record, XOR 0x2774, offset -7)
//! and mobile `Mark_C.dat`.

use crate::error::{Result, TsError};
use std::collections::HashMap;

pub const MARK_RECORD_SIZE: usize = 516;
pub const MARK_XOR_KEY: u16 = 0x2774;
pub const MARK_OFFSET: u16 = 7;

pub struct MarkDatLoader;

impl MarkDatLoader {
    /// Load `quest_id -> position (mark)` mapping from `Mark.Dat`.
    pub fn load(bytes: &[u8]) -> Result<HashMap<u16, u16>> {
        if bytes.len() < MARK_RECORD_SIZE {
            return Err(TsError::Data("Mark.Dat buffer too small".to_string()));
        }

        let mut quest_marks = HashMap::new();

        // PC format (516-byte header + N * 516-byte records)
        if bytes.len() % MARK_RECORD_SIZE == 0 {
            let record_count = (bytes.len() / MARK_RECORD_SIZE).saturating_sub(1);
            quest_marks.reserve(record_count);

            for chunk in bytes[MARK_RECORD_SIZE..].chunks_exact(MARK_RECORD_SIZE) {
                let raw_id = u16::from_le_bytes([chunk[256], chunk[257]]);
                let id = (raw_id ^ MARK_XOR_KEY).wrapping_sub(MARK_OFFSET);

                let raw_pos = u16::from_le_bytes([chunk[258], chunk[259]]);
                let position = (raw_pos ^ MARK_XOR_KEY).wrapping_sub(MARK_OFFSET);

                if id > 0 {
                    quest_marks.insert(id, position);
                }
            }
            return Ok(quest_marks);
        }

        // Fallback for mobile Mark_C.dat format
        let mut reader = crate::data::reader::StrictDatReader::new(bytes);
        let count = reader.read_u32()? as usize;
        quest_marks.reserve(count.min(100_000));
        for _ in 0..count {
            let _name = reader.read_unicode_string()?;
            let _kind = reader.read_u8()?;
            let id = reader.read_u16()?;
            let bit_id = reader.read_u16()?;
            let _gain_way = reader.read_u8()?;
            let _description = reader.read_unicode_string()?;
            if id > 0 {
                quest_marks.insert(id, bit_id);
            }
        }

        Ok(quest_marks)
    }
}
