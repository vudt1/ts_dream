//! EVOStatus loader for `EVOStatus.Dat`.

use crate::data::reader::DatReader;
use crate::error::{Result, TsError};
use std::collections::HashMap;

/// An equipment evolution status bonus definition.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct EVOStatusDef {
    pub id: u8,
    pub kind: u8,
    pub value: u8,
    pub item_id: u16,
    pub description: String,
}

pub struct EVOStatusDatLoader;

impl EVOStatusDatLoader {
    const RECORD_SIZE: usize = 260;

    /// Load EVOStatus definitions from binary slice of `EVOStatus.Dat`.
    pub fn load(bytes: &[u8]) -> Result<HashMap<u8, EVOStatusDef>> {
        if bytes.len() < Self::RECORD_SIZE * 3 {
            return Err(TsError::Data("EVOStatus.Dat buffer too small".to_string()));
        }

        let count = (bytes.len() / Self::RECORD_SIZE).saturating_sub(3);
        let mut reader = DatReader::new(bytes.to_vec());
        reader.decode_all(Some(Self::RECORD_SIZE), Some(count));

        let mut result = HashMap::new();
        while reader.can_read() && reader.remaining() >= Self::RECORD_SIZE {
            let id = reader.read_u8_raw();
            let kind = reader.read_u8_raw();
            let value = reader.read_u8_raw();
            let item_id = reader.read_u16_raw();
            let description = reader.read_string(254);

            if id != 0 {
                result.insert(
                    id,
                    EVOStatusDef {
                        id,
                        kind,
                        value,
                        item_id,
                        description,
                    },
                );
            }
        }

        Ok(result)
    }
}
