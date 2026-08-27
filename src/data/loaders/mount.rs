//! Mobile-compatible `Mounts_C.dat`/`Mounts.Dat` loader.

use crate::data::reader::StrictDatReader;
use crate::data::tables::MountDef;
use crate::error::Result;
use std::collections::HashMap;

pub struct MountDatLoader;

impl MountDatLoader {
    pub fn load(bytes: &[u8]) -> Result<HashMap<u16, MountDef>> {
        let mut reader = StrictDatReader::new(bytes);
        let count = reader.read_u32()? as usize;
        let mut defs = HashMap::with_capacity(count.min(100_000));
        for _ in 0..count {
            let npc_id = reader.read_u16()?;
            let flag_id = reader.read_u16()?;
            let source = reader.read_u32()?;
            let scale = f32::from(reader.read_u8()?) * 0.01;
            if npc_id != 0 {
                defs.insert(
                    npc_id,
                    MountDef {
                        npc_id,
                        flag_id,
                        source,
                        scale,
                    },
                );
            }
        }
        Ok(defs)
    }
}

#[cfg(test)]
mod tests {
    use super::MountDatLoader;

    #[test]
    fn empty_mount_catalog_is_valid() {
        assert!(MountDatLoader::load(&0u32.to_le_bytes())
            .unwrap()
            .is_empty());
    }

    #[test]
    fn truncated_mount_row_is_rejected() {
        let data = [1u8, 0, 0, 0, 1, 0];
        assert!(MountDatLoader::load(&data).is_err());
    }
}
