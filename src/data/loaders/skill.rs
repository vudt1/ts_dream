//! Mobile-compatible `Skill_C.dat`/`Skill.Dat` loader.
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

#[cfg(test)]
mod tests {
    use super::SkillDatLoader;

    fn push_u16(data: &mut Vec<u8>, value: u16) {
        data.extend(value.to_le_bytes());
    }
    fn push_u32(data: &mut Vec<u8>, value: u32) {
        data.extend(value.to_le_bytes());
    }
    fn push_text(data: &mut Vec<u8>, value: &str) {
        let utf16: Vec<u16> = value.encode_utf16().collect();
        push_u16(data, (utf16.len() * 2) as u16);
        for unit in utf16 {
            push_u16(data, unit);
        }
    }
    fn push_skill(data: &mut Vec<u8>, id: u16, pre1: u16, pre2: u16) {
        push_text(data, "Fire");
        data.push(3);
        push_u16(data, id);
        push_u16(data, 44);
        data.push(2);
        push_u32(data, 0x1122_3344);
        data.extend([5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16]);
        push_u16(data, pre1);
        push_u16(data, 17);
        data.push(18);
        push_u16(data, pre2);
        data.push(19);
        push_u16(data, 20);
        push_u16(data, 21);
        push_text(data, "description");
    }

    #[test]
    fn empty_skill_catalog_is_valid() {
        let data = 0u32.to_le_bytes();
        assert!(SkillDatLoader::load(&data).unwrap().is_empty());
    }

    #[test]
    fn field_order_and_special_prerequisites_are_preserved() {
        let mut data = Vec::new();
        push_u32(&mut data, 3);
        push_skill(&mut data, 7, 101, 102);
        push_skill(&mut data, 13014, 0, 0);
        push_skill(&mut data, 14038, 0, 0);
        let result = SkillDatLoader::load(&data).unwrap();
        let normal = &result[&7];
        assert_eq!(normal.name, "Fire");
        assert_eq!(normal.kind, 3);
        assert_eq!(normal.require_sp, 44);
        assert_eq!(normal.numerical, 0x1122_3344);
        assert_eq!(normal.pre_skills, vec![101, 102]);
        assert_eq!(normal.description, "description");
        assert_eq!(result[&13014].pre_skills, vec![13012]);
        assert_eq!(result[&14038].pre_skills, vec![10020, 11020, 12020, 13019]);
    }

    #[test]
    fn truncated_skill_row_is_rejected() {
        let data = [1u8, 0, 0, 0, 0, 0];
        assert!(SkillDatLoader::load(&data).is_err());
    }
}
