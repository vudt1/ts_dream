//! Mobile-compatible `MountsGrow_C.dat`/`MountsGrow.Dat` loader.

use crate::data::reader::StrictDatReader;
use crate::data::tables::{MountAttributeGrow, MountGrowDef};
use crate::error::Result;
use std::collections::HashMap;

pub struct MountGrowDatLoader;

impl MountGrowDatLoader {
    pub fn load(bytes: &[u8]) -> Result<HashMap<u8, MountGrowDef>> {
        let mut reader = StrictDatReader::new(bytes);
        let count = reader.read_u32()? as usize;
        let mut defs = HashMap::with_capacity(count.min(100_000));
        for _ in 0..count {
            let level = reader.read_u8()?;
            let speed = reader.read_u8()?;
            let up_item_id = reader.read_u16()?;
            let up_item_count = reader.read_u8()?;
            let up_money = reader.read_u32()?;
            let mut attributes = Vec::with_capacity(5);
            for _ in 0..5 {
                attributes.push(MountAttributeGrow {
                    add_value: reader.read_u16()?,
                    up_item_id: reader.read_u16()?,
                    up_item_count: reader.read_u16()?,
                });
            }
            defs.insert(
                level,
                MountGrowDef {
                    speed,
                    up_item_id,
                    up_item_count,
                    up_money,
                    attributes,
                },
            );
        }
        Ok(defs)
    }
}

#[cfg(test)]
mod tests {
    use super::MountGrowDatLoader;

    #[test]
    fn empty_mount_grow_catalog_is_valid() {
        assert!(MountGrowDatLoader::load(&0u32.to_le_bytes())
            .unwrap()
            .is_empty());
    }

    #[test]
    fn truncated_mount_grow_row_is_rejected() {
        let data = [1u8, 0, 0, 0, 1, 1];
        assert!(MountGrowDatLoader::load(&data).is_err());
    }
}
