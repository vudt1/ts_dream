//! Mobile-compatible `TeachInfo_C.dat`/`TeachInfo.Dat` loader.

use crate::data::reader::StrictDatReader;
use crate::error::Result;
use std::collections::HashMap;

#[derive(Debug, Clone, Default)]
pub struct TeachInfoResult {
    pub guide_last_bit_flag_id: HashMap<u8, u16>,
    pub guide_mark_flag_ids: HashMap<u8, u16>,
}

#[derive(Debug, Clone, Copy)]
struct StepInfo {
    step: u8,
    mark_flag_id: u16,
    bit_flag_id: u16,
}

pub struct TeachInfoDatLoader;

impl TeachInfoDatLoader {
    pub fn load(bytes: &[u8]) -> Result<TeachInfoResult> {
        let mut reader = StrictDatReader::new(bytes);
        let count = reader.read_u32()? as usize;
        let mut guide_steps: HashMap<u8, Vec<StepInfo>> = HashMap::new();
        for _ in 0..count {
            let guide_id = reader.read_u8()?;
            let step = reader.read_u8()?;
            let _title = reader.read_u32()?;
            let _pic_id = reader.read_u16()?;
            let _description = reader.read_unicode_string()?;
            let _style = reader.read_u8()?;
            let _size_offset = reader.read_i16()?;
            let _anchored_tag = reader.read_unicode_string()?;
            let _anchored_root = reader.read_unicode_string()?;
            let _anchored_object = reader.read_unicode_string()?;
            let _anchored_param = reader.read_u32()?;
            let mark_flag_id = reader.read_u16()?;
            let bit_flag_id = reader.read_u16()?;
            let _close_ui = reader.read_bool()?;
            guide_steps.entry(guide_id).or_default().push(StepInfo {
                step,
                mark_flag_id,
                bit_flag_id,
            });
        }

        let mut last_flags = HashMap::new();
        let mut first_marks = HashMap::new();
        for (guide_id, steps) in guide_steps {
            if let Some(last) = steps.iter().max_by_key(|entry| entry.step) {
                if last.bit_flag_id != 0 {
                    last_flags.insert(guide_id, last.bit_flag_id);
                }
            }
            if let Some(first) = steps.iter().min_by_key(|entry| entry.step) {
                if first.mark_flag_id != 0 {
                    first_marks.insert(guide_id, first.mark_flag_id);
                }
            }
        }
        Ok(TeachInfoResult {
            guide_last_bit_flag_id: last_flags,
            guide_mark_flag_ids: first_marks,
        })
    }
}
