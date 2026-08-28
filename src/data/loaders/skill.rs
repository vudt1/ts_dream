//! Mobile `Skill_C.dat` loader (PC `Skill.Dat` parser lives in `skill_pc.rs`).
//!
//! The field order follows the Kotlin `SkillDatLoader`: count(Int32), then
//! Unicode name, primitive skill metadata, two prerequisite ids, and Unicode
//! description. The PC wire layer still owns packet encoding; this module only
//! produces the shared domain catalog.

use crate::data::reader::StrictDatReader;
use crate::data::tables::BinarySkillDef;
use crate::error::Result;
use std::collections::HashMap;

pub struct SkillDatLoader;

impl SkillDatLoader {
    pub fn load(bytes: &[u8]) -> Result<HashMap<u16, BinarySkillDef>> {
        let mut reader = StrictDatReader::new(bytes);
        let count = reader.read_u32()? as usize;
        let mut result = HashMap::with_capacity(count.min(100_000));
        for _ in 0..count {
            let name = reader.read_unicode_string()?;
            let kind = reader.read_u8()?;
            let id = reader.read_u16()?;
            let require_sp = reader.read_u16()?;
            let element = reader.read_u8()?;
            let numerical = reader.read_u32()?;
            let attribute = reader.read_u8()?;
            let level = reader.read_u8()?;
            let fight_way = reader.read_u8()?;
            let fight_area = reader.read_u8()?;
            let round = reader.read_u8()?;
            let spend_second = reader.read_u8()?;
            let hit_status = reader.read_u8()?;
            let how_much_times = reader.read_u8()?;
            let limit_lv = reader.read_u8()?;
            let learn_point = reader.read_u8()?;
            let level_up_point = reader.read_u8()?;
            let max_lv = reader.read_u8()?;
            let pre_skill_id1 = reader.read_u16()?;
            let atk_kind = reader.read_u16()?;
            let turn_kind = reader.read_u8()?;
            let pre_skill_id2 = reader.read_u16()?;
            let learn_limit = reader.read_u8()?;
            let use_limit = reader.read_u16()?;
            let fight_way_grow_type = reader.read_u16()?;
            let description = reader.read_unicode_string()?;
            if id == 0 {
                continue;
            }
            let mut pre_skills = Vec::with_capacity(6);
            if pre_skill_id1 != 0 {
                pre_skills.push(pre_skill_id1);
            }
            if pre_skill_id2 != 0 {
                pre_skills.push(pre_skill_id2);
            }
            if id == 13014 {
                pre_skills.push(13012);
            }
            if id == 14038 {
                pre_skills.extend([10020, 11020, 12020, 13019]);
            }
            result.insert(
                id,
                BinarySkillDef {
                    id,
                    name,
                    kind,
                    require_sp,
                    element,
                    numerical,
                    attribute,
                    level,
                    fight_way,
                    fight_area,
                    round,
                    spend_second,
                    hit_status,
                    how_much_times,
                    limit_lv,
                    learn_point,
                    level_up_point,
                    max_lv,
                    pre_skills,
                    atk_kind,
                    turn_kind,
                    learn_limit,
                    use_limit,
                    fight_way_grow_type,
                    description,
                },
            );
        }
        Ok(result)
    }
}
