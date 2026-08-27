//! Mobile-compatible `Mark_C.dat`/`Mark.Dat` mission-mark loader.

use crate::data::reader::StrictDatReader;
use crate::data::tables::MarkDef;
use crate::error::Result;
use std::collections::HashMap;

pub struct MarkDatLoader;

impl MarkDatLoader {
    pub fn load(bytes: &[u8]) -> Result<(HashMap<u16, MarkDef>, HashMap<u16, u16>)> {
        let mut reader = StrictDatReader::new(bytes);
        let count = reader.read_u32()? as usize;
        let mut defs = HashMap::with_capacity(count.min(100_000));
        let mut bit_to_mission = HashMap::new();
        for _ in 0..count {
            let name = reader.read_unicode_string()?;
            let kind = reader.read_u8()?;
            let id = reader.read_u16()?;
            let bit_id = reader.read_u16()?;
            let gain_way = reader.read_u8()?;
            let description = reader.read_unicode_string()?;
            if id == 0 {
                continue;
            }
            defs.insert(
                id,
                MarkDef {
                    name,
                    kind,
                    bit_id,
                    gain_way,
                    description,
                },
            );
            if bit_id != 0 {
                bit_to_mission.insert(bit_id, id);
            }
        }
        Ok((defs, bit_to_mission))
    }
}

#[cfg(test)]
mod tests {
    use super::MarkDatLoader;

    #[test]
    fn empty_mark_catalog_is_valid() {
        assert!(MarkDatLoader::load(&0u32.to_le_bytes())
            .unwrap()
            .0
            .is_empty());
    }

    #[test]
    fn truncated_mark_row_is_rejected() {
        let data = [1u8, 0, 0, 0, 0, 0];
        assert!(MarkDatLoader::load(&data).is_err());
    }
}
