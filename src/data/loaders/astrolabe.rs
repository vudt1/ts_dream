//! Astrolabe (本命燈) loader for `Astrolabe.Dat`.

use crate::data::reader::DatReader;
use crate::error::{Result, TsError};
use std::collections::HashMap;

/// An astrolabe definition with 10 level attribute increments.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct AstrolabeDef {
    pub index: u8,
    pub attributes: [(u8, u8); 10], // (value, need_point)
}

pub struct AstrolabeDatLoader;

impl AstrolabeDatLoader {
    /// Load astrolabe definitions from binary slice of `Astrolabe.Dat`.
    pub fn load(bytes: &[u8]) -> Result<HashMap<u8, AstrolabeDef>> {
        if bytes.len() < 8 {
            return Err(TsError::Data("Astrolabe.Dat buffer too small".to_string()));
        }

        let mut reader = DatReader::new(bytes.to_vec());
        reader.decode_all(None, None);

        let mut result = HashMap::new();
        while reader.can_read() && reader.remaining() >= 21 {
            let index = reader.read_u8_raw();
            let mut attrs = [(0u8, 0u8); 10];
            for attr in &mut attrs {
                attr.0 = reader.read_u8_raw();
                attr.1 = reader.read_u8_raw();
            }

            if index != 0 {
                result.insert(
                    index,
                    AstrolabeDef {
                        index,
                        attributes: attrs,
                    },
                );
            }
        }

        Ok(result)
    }
}
