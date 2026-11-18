//! Server-optimized binary loader for `Item.dat` (370-byte records).
//!
//! Decodes only the columns required by server gameplay, crafting, shops,
//! inventory hydration, and battle, skipping client-only visual/UI columns
//! and 254-byte descriptions.

use crate::data::tables::Item;
use crate::error::{Result, TsError};
use std::collections::HashMap;

pub struct ItemDatLoader;

impl ItemDatLoader {
    pub const FIELD_LENGTH: usize = 370;

    #[inline]
    fn dec32(val: u32) -> u32 {
        (val ^ 0x0B80_F4B4).wrapping_sub(9)
    }

    #[inline]
    fn dec32s(val: u32) -> i32 {
        (val ^ 0x0B80_F4B4).wrapping_sub(109) as i32
    }

    #[inline]
    fn dec16(val: u16) -> u16 {
        (val ^ 0xEFC3).wrapping_sub(9)
    }

    #[inline]
    fn dec8(val: u8) -> u8 {
        (val ^ 0x9A).wrapping_sub(9)
    }

    #[inline]
    fn read_u16_le(chunk: &[u8], offset: usize) -> u16 {
        u16::from_le_bytes([chunk[offset], chunk[offset + 1]])
    }

    #[inline]
    fn read_u32_le(chunk: &[u8], offset: usize) -> u32 {
        u32::from_le_bytes([
            chunk[offset],
            chunk[offset + 1],
            chunk[offset + 2],
            chunk[offset + 3],
        ])
    }

    #[inline]
    fn decode_attr_value(kind: u16, raw: u32) -> i32 {
        let mut value = Self::dec32s(raw);
        if (65..=67).contains(&kind) {
            value += 100;
        }
        value
    }

    #[inline]
    fn apply_attr(item: &mut Item, kind: u16, value: i64, is_second: bool) {
        match kind {
            25 | 207 if !is_second || item.hp == 0 => item.hp = value,
            26 | 208 if !is_second || item.sp == 0 => item.sp = value,
            210 => {
                if is_second {
                    item.atk2 = value;
                } else {
                    item.atk1 = value;
                }
            }
            211 => {
                if is_second {
                    item.def2 = value;
                } else {
                    item.def1 = value;
                }
            }
            212 => {
                if is_second {
                    item.int2 = value;
                } else {
                    item.int1 = value;
                }
            }
            214 => {
                if is_second {
                    item.agi2 = value;
                } else {
                    item.agi1 = value;
                }
            }
            218 => {
                if is_second {
                    item.hpx2 = value;
                } else {
                    item.hpx1 = value;
                }
            }
            219 => {
                if is_second {
                    item.spx2 = value;
                } else {
                    item.spx1 = value;
                }
            }
            64 => {
                if is_second {
                    item.fai2 = value;
                } else {
                    item.fai1 = value;
                }
            }
            _ => {}
        }
    }

    /// Load server-relevant `Item` records directly from binary `Item.dat`.
    pub fn load(bytes: &[u8]) -> Result<HashMap<i64, Item>> {
        if !bytes.len().is_multiple_of(Self::FIELD_LENGTH) {
            return Err(TsError::Data(format!(
                "Invalid Item.dat size {} (not multiple of {})",
                bytes.len(),
                Self::FIELD_LENGTH
            )));
        }

        let total_records = bytes.len() / Self::FIELD_LENGTH;
        let mut result = HashMap::with_capacity(total_records);

        // Record 0 is an empty/dummy header in TS Online; start loop from 1.
        for i in 1..total_records {
            let offset = i * Self::FIELD_LENGTH;
            let chunk = &bytes[offset..offset + Self::FIELD_LENGTH];

            let id = Self::dec16(Self::read_u16_le(chunk, 22));
            if id == 0 {
                continue;
            }

            let namelen = chunk[0] as usize;
            let name = if namelen > 0 && namelen <= 20 {
                let start = 1 + 20 - namelen;
                let mut sub = chunk[start..start + namelen].to_vec();
                sub.reverse();
                sub
            } else {
                Vec::new()
            };

            let kind = Self::dec8(chunk[21]);

            // Offsets 24..32 (icon_id, _large_icon_num, male_pic_id, female_pic_id) are client-only.
            let attr1_kind = Self::dec16(Self::read_u16_le(chunk, 32));
            let attr2_kind = Self::dec16(Self::read_u16_le(chunk, 34));
            // Offsets 36..38 (attr1_item, attr2_item) are unused by the server.
            let attr1_value = Self::decode_attr_value(attr1_kind, Self::read_u32_le(chunk, 38));
            let attr2_value = Self::decode_attr_value(attr2_kind, Self::read_u32_le(chunk, 42));

            let compound_material = Self::dec8(chunk[46]);
            let compound_rank = Self::dec8(chunk[47]);
            let fit_type = Self::dec8(chunk[48]);
            // Offsets 49..83 (special_ability, color tints, display_material) are client-only/unused.
            let level = Self::dec8(chunk[83]);
            let price = Self::dec32(Self::read_u32_le(chunk, 84));
            let sell_price = Self::dec32(Self::read_u32_le(chunk, 88));
            // Offsets 92..98 (_equip_limit, open_used, threshold) are unused by the server.
            let element = Self::dec8(chunk[98]);
            let element_value = Self::dec32s(Self::read_u32_le(chunk, 99));
            let spare3 = Self::dec16(Self::read_u16_le(chunk, 103));
            // Offset 105 (restrict2) is unused by the server.
            let suit_id = Self::dec16(Self::read_u16_le(chunk, 106));
            // Offset 108 (spare5) is unused by the server.
            let role_count_index = Self::dec16(Self::read_u16_le(chunk, 109));
            // Offsets 111..370 (sort, role_count_value, btn_state, description) are client-only/unused.

            let id_i64 = i64::from(id);
            let mut item = Item {
                id: id_i64,
                name,
                level: i64::from(level),
                loai: i64::from(kind),
                equip_pos: i64::from(fit_type),
                thuoctinh: i64::from(element),
                value: i64::from(element_value),
                rb_pet_from: i64::from(spare3),
                rb_pet_to: i64::from(suit_id),
                add_pet: i64::from(role_count_index),
                compound_material: i64::from(compound_material),
                compound_rank: i64::from(compound_rank),
                price,
                sell_price,
                garble: None,
                ..Default::default()
            };

            Self::apply_attr(&mut item, attr1_kind, i64::from(attr1_value), false);
            Self::apply_attr(&mut item, attr2_kind, i64::from(attr2_value), true);

            result.insert(id_i64, item);
        }

        Ok(result)
    }
}
