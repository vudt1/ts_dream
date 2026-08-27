//! Mobile-compatible Dispatch and DispatchBonus loaders.

use crate::data::reader::StrictDatReader;
use crate::data::tables::{DispatchBonusDef, DispatchDef};
use crate::error::Result;
use std::collections::HashMap;

pub struct DispatchDatLoader;

impl DispatchDatLoader {
    pub fn load_dispatch(bytes: &[u8]) -> Result<HashMap<u8, HashMap<u8, DispatchDef>>> {
        let mut reader = StrictDatReader::new(bytes);
        let count = reader.read_u32()? as usize;
        let mut result: HashMap<u8, HashMap<u8, DispatchDef>> = HashMap::new();
        for _ in 0..count {
            let main_kind = reader.read_u8()?;
            let sub_kind = reader.read_u8()?;
            let min_lv = reader.read_u8()?;
            let award_id = reader.read_u16()?;
            let exp_kind = reader.read_u8()?;
            let exp = reader.read_u32()?;
            result.entry(main_kind).or_default().insert(
                sub_kind,
                DispatchDef {
                    min_lv,
                    award_id,
                    exp_kind,
                    exp,
                },
            );
        }
        Ok(result)
    }

    pub fn load_bonus(bytes: &[u8]) -> Result<HashMap<u8, DispatchBonusDef>> {
        let mut reader = StrictDatReader::new(bytes);
        let count = reader.read_u32()? as usize;
        let mut result = HashMap::with_capacity(count.min(100_000));
        for _ in 0..count {
            let id = reader.read_u8()?;
            result.insert(
                id,
                DispatchBonusDef {
                    condition_kind: reader.read_u8()?,
                    condition_value: reader.read_u8()?,
                    effect_index: reader.read_u8()?,
                    effect_kind: reader.read_u8()?,
                    effect_value: reader.read_u16()?,
                    effect_content: reader.read_u32()?,
                },
            );
        }
        Ok(result)
    }
}
