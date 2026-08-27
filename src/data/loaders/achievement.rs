//! Mobile-compatible `AchievementData_C.dat`/`AchievementData.Dat` loader.

use crate::data::reader::StrictDatReader;
use crate::data::tables::AchievementDef;
use crate::error::Result;
use std::collections::HashMap;

pub struct AchievementDatLoader;

impl AchievementDatLoader {
    pub fn load(bytes: &[u8]) -> Result<HashMap<u16, AchievementDef>> {
        let mut reader = StrictDatReader::new(bytes);
        let count = reader.read_u32()? as usize;
        let mut result = HashMap::with_capacity(count.min(100_000));
        for _ in 0..count {
            let id = reader.read_u16()?;
            let _name = reader.read_u32()?;
            let main_tag = reader.read_u8()?;
            let sub_tag = reader.read_u8()?;
            let sort_id = reader.read_u8()?;
            let show_kind = reader.read_u8()?;
            let _content = reader.read_u32()?;
            let score = reader.read_u8()?;
            let condition_kind = reader.read_u8()?;
            let condition_kind_value = reader.read_u32()?;
            let condition_opr = reader.read_u8()?;
            let condition_value = reader.read_u32()?;
            let item_id = reader.read_u16()?;
            let item_count = reader.read_u8()?;
            let complete_flag = reader.read_u16()?;
            let get_flag = reader.read_u16()?;
            let channel = reader.read_u8()?;
            let channel_content = reader.read_u32()?;
            if id != 0 {
                result.insert(
                    id,
                    AchievementDef {
                        id,
                        main_tag,
                        sub_tag,
                        sort_id,
                        show_kind,
                        score,
                        condition_kind,
                        condition_kind_value,
                        condition_opr,
                        condition_value,
                        item_id,
                        item_count,
                        complete_flag,
                        get_flag,
                        channel,
                        channel_content,
                    },
                );
            }
        }
        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::AchievementDatLoader;

    #[test]
    fn empty_achievement_catalog_is_valid() {
        assert!(AchievementDatLoader::load(&0u32.to_le_bytes())
            .unwrap()
            .is_empty());
    }

    #[test]
    fn truncated_achievement_row_is_rejected() {
        let data = [1u8, 0, 0, 0, 1, 0];
        assert!(AchievementDatLoader::load(&data).is_err());
    }
}
