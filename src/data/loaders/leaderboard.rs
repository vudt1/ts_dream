//! Mobile-compatible `LeaderboardInfo_C.dat`/`LeaderboardInfo.Dat` loader.

use crate::data::reader::StrictDatReader;
use crate::data::tables::LeaderboardDef;
use crate::error::Result;
use std::collections::HashMap;

pub struct LeaderboardDatLoader;

impl LeaderboardDatLoader {
    pub fn load(bytes: &[u8]) -> Result<HashMap<u8, LeaderboardDef>> {
        let mut reader = StrictDatReader::new(bytes);
        let count = reader.read_u32()? as usize;
        let mut result = HashMap::with_capacity(count.min(100_000));
        for _ in 0..count {
            let id = reader.read_u8()?;
            let main_tag_text = reader.read_u32()?;
            let sub_tag_text = reader.read_u32()?;
            let score_text = reader.read_u32()?;
            let name_text = reader.read_u32()?;
            let award_id = reader.read_u8()?;
            let score_format = reader.read_unicode_string()?;
            let state_enable = reader.read_bool()?;
            let rank_award_flag_ids = [reader.read_u16()?, reader.read_u16()?, reader.read_u16()?];
            let bit_function = reader.read_u8()?;
            result.insert(
                id,
                LeaderboardDef {
                    id,
                    main_tag_text,
                    sub_tag_text,
                    score_text,
                    name_text,
                    award_id,
                    score_format,
                    state_enable,
                    rank_award_flag_ids,
                    bit_function,
                },
            );
        }
        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::LeaderboardDatLoader;

    #[test]
    fn empty_leaderboard_catalog_is_valid() {
        assert!(LeaderboardDatLoader::load(&0u32.to_le_bytes())
            .unwrap()
            .is_empty());
    }

    #[test]
    fn truncated_leaderboard_row_is_rejected() {
        let data = [1u8, 0, 0, 0, 1, 0, 0, 0];
        assert!(LeaderboardDatLoader::load(&data).is_err());
    }
}
