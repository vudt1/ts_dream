//! Mobile-compatible `SceneSet_C.dat`/`SceneSet.Dat` loader.

use crate::data::reader::StrictDatReader;
use crate::data::tables::SceneSetDef;
use crate::error::Result;
use std::collections::HashMap;

pub struct SceneSetDatLoader;

impl SceneSetDatLoader {
    pub fn load(bytes: &[u8]) -> Result<HashMap<u16, SceneSetDef>> {
        let mut reader = StrictDatReader::new(bytes);
        let count = reader.read_u32()? as usize;
        let mut result = HashMap::with_capacity(count.min(100_000));
        for _ in 0..count {
            let name_id = reader.read_u32()?;
            let id = reader.read_u16()?;
            let def = SceneSetDef {
                name_id,
                kind: reader.read_u8()?,
                max_player: reader.read_u16()?,
                limit1: reader.read_u8()?,
                limit2: reader.read_u8()?,
                effect: reader.read_u8()?,
                min_lv: reader.read_u8()?,
                max_lv: reader.read_u8()?,
                view_setting: reader.read_u8()?,
                sub_id: reader.read_u8()?,
                limit3: reader.read_u8()?,
            };
            if id != 0 {
                result.insert(id, def);
            }
        }
        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::SceneSetDatLoader;

    #[test]
    fn empty_scene_set_catalog_is_valid() {
        assert!(SceneSetDatLoader::load(&0u32.to_le_bytes())
            .unwrap()
            .is_empty());
    }

    #[test]
    fn truncated_scene_set_row_is_rejected() {
        let data = [1u8, 0, 0, 0, 0, 0];
        assert!(SceneSetDatLoader::load(&data).is_err());
    }
}
