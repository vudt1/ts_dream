//! Item loader for `Item.dat` (3.09 MB binary).
//!
//! Provides `ItemDatLoader` and `ItemDef` matching all 54 data fields.

use crate::data::tables::Item;
use crate::encoding::{self, GarbleSpec};
use crate::error::{Result, TsError};
use std::collections::HashMap;

/// An item definition representing all 54 fields parsed from binary `Item.dat`.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct ItemDef {
    pub id: u16,
    pub name: String,
    pub name_viscii: Vec<u8>,
    pub kind: u8,
    pub icon_id: u16,
    pub male_pic_id: u16,
    pub female_pic_id: u16,
    pub attr1_kind: u16,
    pub attr1_item: u8,
    pub attr1_value: i32,
    pub attr2_kind: u16,
    pub attr2_item: u8,
    pub attr2_value: i32,
    pub material: u8,
    pub level: u8,
    pub fit_type: u8,
    pub special_ability: u16,
    pub male_color_tints: [u32; 4],
    pub female_color_tints: [u32; 4],
    pub open_used: u8,
    pub need_lv: u8,
    pub price: u32,
    pub sell_price: u32,
    pub gender: u8,
    pub restrict: u8,
    pub threshold: u32,
    pub element: u8,
    pub element_value: i32,
    pub skill_link: u16,
    pub turn: u8,
    pub gift_dot: u16,
    pub spare2: u8,
    pub spare3: u16, // rb_pet_from
    pub restrict2: u8,
    pub suit_id: u16, // rb_pet_to
    pub spare5: u8,
    pub direct_use: u8,
    pub role_count_index: u16, // add_pet
    pub role_count_value: i32,
    pub sort: u8,
    pub male_equip_switch: u8,
    pub female_equip_switch: u8,
    pub btn_state: u8,
    pub durable: u8,
    pub furnace_kind: u8,
    pub furnace_count: u32,
    pub quality: u8,
    pub auction_tag: u8,
    pub auction_sub_tag: u8,
    pub description: String,
    pub description_viscii: Vec<u8>,
    pub garble: Option<GarbleSpec>,
}

impl ItemDef {
    /// Convert to in-memory `Item` table record.
    pub fn to_item(&self) -> Item {
        let mut item = Item {
            id: self.id as i64,
            name: self.name_viscii.clone(),
            level: self.level as i64,
            loai: self.kind as i64,
            thuoctinh: self.element as i64,
            value: self.element_value as i64,
            rb_pet_from: self.spare3 as i64,
            rb_pet_to: self.suit_id as i64,
            add_pet: self.role_count_index as i64,
            garble: self.garble.clone(),
            ..Default::default()
        };

        // Map attr1 to stats
        match self.attr1_kind {
            25 | 207 => item.hp = self.attr1_value as i64,
            26 | 208 => item.sp = self.attr1_value as i64,
            210 => item.atk1 = self.attr1_value as i64,
            211 => item.def1 = self.attr1_value as i64,
            212 => item.int1 = self.attr1_value as i64,
            214 => item.agi1 = self.attr1_value as i64,
            218 => item.hpx1 = self.attr1_value as i64,
            219 => item.spx1 = self.attr1_value as i64,
            64 => item.fai1 = self.attr1_value as i64,
            _ => {}
        }

        // Map attr2 to stats
        match self.attr2_kind {
            25 | 207 if item.hp == 0 => item.hp = self.attr2_value as i64,
            26 | 208 if item.sp == 0 => item.sp = self.attr2_value as i64,
            210 => item.atk2 = self.attr2_value as i64,
            211 => item.def2 = self.attr2_value as i64,
            212 => item.int2 = self.attr2_value as i64,
            214 => item.agi2 = self.attr2_value as i64,
            218 => item.hpx2 = self.attr2_value as i64,
            219 => item.spx2 = self.attr2_value as i64,
            64 => item.fai2 = self.attr2_value as i64,
            _ => {}
        }

        item
    }
}

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

    /// Load all item definitions from binary slice of `Item.dat`.
    pub fn load(bytes: &[u8]) -> Result<HashMap<u16, ItemDef>> {
        if bytes.len() % Self::FIELD_LENGTH != 0 {
            return Err(TsError::Data(format!(
                "Invalid Item.dat size {} (not multiple of {})",
                bytes.len(),
                Self::FIELD_LENGTH
            )));
        }

        let total_records = bytes.len() / Self::FIELD_LENGTH;
        let mut result = HashMap::with_capacity(total_records);

        // Record 0 and 1 are empty/dummy in TS Online, start loop from 1
        for i in 1..total_records {
            let offset = i * Self::FIELD_LENGTH;
            let chunk = &bytes[offset..offset + Self::FIELD_LENGTH];

            let namelen = chunk[0] as usize;
            let (name_str, name_viscii) = if namelen > 0 && namelen <= 20 {
                let start = 1 + 20 - namelen;
                let mut sub = chunk[start..start + namelen].to_vec();
                sub.reverse();
                let decoded_str = encoding::cp1252_to_string(&sub);
                (decoded_str, sub)
            } else {
                (String::new(), Vec::new())
            };

            let kind = Self::dec8(chunk[21]);
            let id = Self::dec16(u16::from_le_bytes([chunk[22], chunk[23]]));
            if id == 0 {
                continue;
            }

            let icon_id = Self::dec16(u16::from_le_bytes([chunk[24], chunk[25]]));
            let _large_icon_num = Self::dec16(u16::from_le_bytes([chunk[26], chunk[27]]));
            let male_pic_id = Self::dec16(u16::from_le_bytes([chunk[28], chunk[29]]));
            let female_pic_id = Self::dec16(u16::from_le_bytes([chunk[30], chunk[31]]));

            let attr1_kind = Self::dec16(u16::from_le_bytes([chunk[32], chunk[33]]));
            let attr2_kind = Self::dec16(u16::from_le_bytes([chunk[34], chunk[35]]));
            let attr1_item = Self::dec8(chunk[36]);
            let attr2_item = Self::dec8(chunk[37]);
            let mut attr1_value = Self::dec32s(u32::from_le_bytes([
                chunk[38], chunk[39], chunk[40], chunk[41],
            ]));
            if (65..=67).contains(&attr1_kind) {
                attr1_value += 100;
            }
            let mut attr2_value = Self::dec32s(u32::from_le_bytes([
                chunk[42], chunk[43], chunk[44], chunk[45],
            ]));
            if (65..=67).contains(&attr2_kind) {
                attr2_value += 100;
            }

            let _contribute = Self::dec8(chunk[46]);
            let _sell_price_rate = Self::dec8(chunk[47]);
            let fit_type = Self::dec8(chunk[48]);
            let special_ability = Self::dec8(chunk[49]) as u16;

            let mut male_color_tints = [0u32; 4];
            for j in 0..4 {
                let off = 50 + j * 4;
                male_color_tints[j] = Self::dec32(u32::from_le_bytes([
                    chunk[off],
                    chunk[off + 1],
                    chunk[off + 2],
                    chunk[off + 3],
                ]));
            }
            let mut female_color_tints = [0u32; 4];
            for j in 0..4 {
                let off = 66 + j * 4;
                female_color_tints[j] = Self::dec32(u32::from_le_bytes([
                    chunk[off],
                    chunk[off + 1],
                    chunk[off + 2],
                    chunk[off + 3],
                ]));
            }

            let material = Self::dec8(chunk[82]);
            let level = Self::dec8(chunk[83]);
            let price = Self::dec32(u32::from_le_bytes([
                chunk[84], chunk[85], chunk[86], chunk[87],
            ]));
            let sell_price = Self::dec32(u32::from_le_bytes([
                chunk[88], chunk[89], chunk[90], chunk[91],
            ]));
            let _equip_limit = Self::dec8(chunk[92]);
            let open_used = Self::dec8(chunk[93]);
            let threshold = Self::dec32(u32::from_le_bytes([
                chunk[94], chunk[95], chunk[96], chunk[97],
            ]));
            let element = Self::dec8(chunk[98]);
            let element_value = Self::dec32s(u32::from_le_bytes([
                chunk[99], chunk[100], chunk[101], chunk[102],
            ]));

            let spare3 = Self::dec16(u16::from_le_bytes([chunk[103], chunk[104]]));
            let restrict2 = Self::dec8(chunk[105]);
            let suit_id = Self::dec16(u16::from_le_bytes([chunk[106], chunk[107]]));
            let spare5 = Self::dec8(chunk[108]);
            let role_count_index = Self::dec16(u16::from_le_bytes([chunk[109], chunk[110]]));
            let sort = Self::dec8(chunk[111]);
            let role_count_value =
                Self::dec16(u16::from_le_bytes([chunk[112], chunk[113]])) as i32;
            let btn_state = Self::dec8(chunk[114]);

            // Description at offset 115..370
            let desclen = chunk[115] as usize;
            let (desc_str, desc_viscii) = if desclen > 0 && desclen <= 254 {
                let start = 116 + 254 - desclen;
                let mut sub = chunk[start..start + desclen].to_vec();
                sub.reverse();
                let decoded_str: String = sub
                    .iter()
                    .map(|&b| encoding::viscii_to_unicode(b))
                    .collect();
                (decoded_str, sub)
            } else {
                (String::new(), Vec::new())
            };

            let garble = None;

            result.insert(
                id,
                ItemDef {
                    id,
                    name: name_str,
                    name_viscii,
                    kind,
                    icon_id,
                    male_pic_id,
                    female_pic_id,
                    attr1_kind,
                    attr1_item,
                    attr1_value,
                    attr2_kind,
                    attr2_item,
                    attr2_value,
                    material,
                    level,
                    fit_type,
                    special_ability,
                    male_color_tints,
                    female_color_tints,
                    open_used,
                    need_lv: level,
                    price,
                    sell_price,
                    gender: 0,
                    restrict: 0,
                    threshold,
                    element,
                    element_value,
                    skill_link: 0,
                    turn: 0,
                    gift_dot: 0,
                    spare2: 0,
                    spare3,
                    restrict2,
                    suit_id,
                    spare5,
                    direct_use: 0,
                    role_count_index,
                    role_count_value,
                    sort,
                    male_equip_switch: 0,
                    female_equip_switch: 0,
                    btn_state,
                    durable: 0,
                    furnace_kind: 0,
                    furnace_count: 0,
                    quality: 0,
                    auction_tag: 0,
                    auction_sub_tag: 0,
                    description: desc_str,
                    description_viscii: desc_viscii,
                    garble,
                },
            );
        }

        Ok(result)
    }
}
