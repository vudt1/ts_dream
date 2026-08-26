//! NPC loader for `Npc.dat` (612 KB binary).
//!
//! Provides `NpcDatLoader` and `NpcDef`.

use crate::data::tables::Npc;
use crate::encoding::{self, GarbleSpec};
use crate::error::{Result, TsError};
use std::collections::HashMap;

/// NPC definition parsed from `Npc.dat`.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct NpcDef {
    pub id: u16,
    pub name: String,
    pub name_viscii: Vec<u8>,
    pub kind: u8,
    pub pic_id: u16,
    pub mask_id: u16,
    pub color_tints: [u32; 4],
    pub can_be_catch: bool,
    pub not_pet: u8,
    pub body_kind: u8,
    pub weapon_kind: u8,
    pub level: u8,
    pub hp_base: u32,
    pub sp_base: u32,
    pub hpx: u16,
    pub spx: u16,
    pub intelligence: u16,
    pub atk: u16,
    pub def: u16,
    pub agi: u16,
    pub moral: u8,
    pub moral_value: u16,
    pub element: u8,
    pub skills: [u16; 4],
    pub drops: [u16; 6],
    pub special_skill: u16,
    pub turn: u8,
    pub passive_skill: u16,
    pub passive_skill_lv: u8,
    pub saddle_kind: u16,
    pub upgrade_item_id: u16,
    pub upgrade_skill: u16,
    pub limits: [u8; 2],
    pub ride_offset_h: i32,
    pub pic_offset_x: i32,
    pub pic_offset_y: i32,
    pub hud_offset_h: i32,
    pub shadow_kind: u8,
    pub rare: u8,
    pub garble: Option<GarbleSpec>,
}

impl NpcDef {
    /// Convert to in-memory `Npc` table record.
    pub fn to_npc(&self) -> Npc {
        Npc {
            id: self.id as i64,
            name: self.name_viscii.clone(),
            lv: self.level as i64,
            thuoctinh: self.element as i64,
            hp: self.hp_base as i64,
            sp: self.sp_base as i64,
            hpx: self.hpx as i64,
            spx: self.spx as i64,
            int1: self.intelligence as i64,
            atk: self.atk as i64,
            def: self.def as i64,
            agi: self.agi as i64,
            skill: [
                self.skills[0] as i64,
                self.skills[1] as i64,
                self.skills[2] as i64,
                self.skills[3] as i64,
            ],
            item: [
                self.drops[0] as i64,
                self.drops[1] as i64,
                self.drops[2] as i64,
                self.drops[3] as i64,
                self.drops[4] as i64,
                self.drops[5] as i64,
            ],
            bat: self.not_pet as i64,
            reborn: self.turn as i64,
            garble: self.garble.clone(),
        }
    }
}

pub struct NpcDatLoader;

impl NpcDatLoader {
    pub const FIELD_LENGTH: usize = 92;

    #[inline]
    fn dec32(val: u32) -> u32 {
        (val ^ 0x0BAE_B716).wrapping_sub(1)
    }

    #[inline]
    fn dec16(val: u16) -> u16 {
        (val ^ 0x5209).wrapping_sub(1)
    }

    #[inline]
    fn dec8(val: u8) -> u8 {
        if val != 200 {
            (val ^ 0xC8).wrapping_sub(1)
        } else {
            255
        }
    }

    /// Load all NPC definitions from binary slice of `Npc.dat`.
    pub fn load(bytes: &[u8]) -> Result<HashMap<u16, NpcDef>> {
        if !bytes.len().is_multiple_of(Self::FIELD_LENGTH) {
            return Err(TsError::Data(format!(
                "Invalid Npc.dat size {} (not multiple of {})",
                bytes.len(),
                Self::FIELD_LENGTH
            )));
        }

        let total_records = bytes.len() / Self::FIELD_LENGTH;
        let mut result = HashMap::with_capacity(total_records);

        // Record 0 and 1 are dummy in TS Online, start from 1
        for i in 1..total_records {
            let offset = i * Self::FIELD_LENGTH;
            let chunk = &bytes[offset..offset + Self::FIELD_LENGTH];

            let namelen = chunk[0] as usize;
            let (name_str, name_viscii) = if namelen > 0 && namelen <= 14 {
                let start = 1 + 14 - namelen;
                let mut sub = chunk[start..start + namelen].to_vec();
                sub.reverse();
                let decoded_str: String = sub
                    .iter()
                    .map(|&b| encoding::viscii_to_unicode(b))
                    .collect();
                (decoded_str, sub)
            } else {
                (String::new(), Vec::new())
            };

            let kind = Self::dec8(chunk[15]);
            let id = Self::dec16(u16::from_le_bytes([chunk[16], chunk[17]]));
            if id == 0 {
                continue;
            }

            let pic_id = Self::dec16(u16::from_le_bytes([chunk[18], chunk[19]]));
            let mask_id = Self::dec16(u16::from_le_bytes([chunk[20], chunk[21]]));

            let mut color_tints = [0u32; 4];
            for (j, tint) in color_tints.iter_mut().enumerate() {
                let off = 22 + j * 4;
                *tint = Self::dec32(u32::from_le_bytes([
                    chunk[off],
                    chunk[off + 1],
                    chunk[off + 2],
                    chunk[off + 3],
                ]));
            }

            let not_pet = Self::dec8(chunk[38]);
            let body_kind = Self::dec8(chunk[39]);
            let weapon_kind = Self::dec8(chunk[40]);
            let level = Self::dec8(chunk[41]);
            let hp_base = Self::dec32(u32::from_le_bytes([
                chunk[42], chunk[43], chunk[44], chunk[45],
            ]));
            let sp_base = Self::dec32(u32::from_le_bytes([
                chunk[46], chunk[47], chunk[48], chunk[49],
            ]));

            let hpx = Self::dec16(u16::from_le_bytes([chunk[50], chunk[51]]));
            let spx = Self::dec16(u16::from_le_bytes([chunk[52], chunk[53]]));
            let intelligence = Self::dec16(u16::from_le_bytes([chunk[54], chunk[55]]));
            let atk = Self::dec16(u16::from_le_bytes([chunk[56], chunk[57]]));
            let def = Self::dec16(u16::from_le_bytes([chunk[58], chunk[59]]));
            let agi = Self::dec16(u16::from_le_bytes([chunk[60], chunk[61]]));
            let moral = Self::dec8(chunk[62]);
            let moral_value = Self::dec16(u16::from_le_bytes([chunk[63], chunk[64]]));
            let element = Self::dec8(chunk[65]);

            let skill1 = Self::dec16(u16::from_le_bytes([chunk[66], chunk[67]]));
            let skill2 = Self::dec16(u16::from_le_bytes([chunk[68], chunk[69]]));
            let skill3 = Self::dec16(u16::from_le_bytes([chunk[70], chunk[71]]));

            let drop1 = Self::dec16(u16::from_le_bytes([chunk[72], chunk[73]]));
            let drop2 = Self::dec16(u16::from_le_bytes([chunk[74], chunk[75]]));
            let drop3 = Self::dec16(u16::from_le_bytes([chunk[76], chunk[77]]));
            let drop4 = Self::dec16(u16::from_le_bytes([chunk[78], chunk[79]]));
            let drop5 = Self::dec16(u16::from_le_bytes([chunk[80], chunk[81]]));
            let drop6 = Self::dec16(u16::from_le_bytes([chunk[82], chunk[83]]));

            let _unk1 = Self::dec8(chunk[84]);
            let skill4 = Self::dec16(u16::from_le_bytes([chunk[85], chunk[86]]));
            let turn = Self::dec8(chunk[87]);
            let _unk2 = Self::dec16(u16::from_le_bytes([chunk[88], chunk[89]]));
            let _unk3 = Self::dec16(u16::from_le_bytes([chunk[90], chunk[91]]));

            let garble = encoding::compute_garble(&name_str);

            result.insert(
                id,
                NpcDef {
                    id,
                    name: name_str,
                    name_viscii,
                    kind,
                    pic_id,
                    mask_id,
                    color_tints,
                    can_be_catch: not_pet == 0,
                    not_pet,
                    body_kind,
                    weapon_kind,
                    level,
                    hp_base,
                    sp_base,
                    hpx,
                    spx,
                    intelligence,
                    atk,
                    def,
                    agi,
                    moral,
                    moral_value,
                    element,
                    skills: [skill1, skill2, skill3, skill4],
                    drops: [drop1, drop2, drop3, drop4, drop5, drop6],
                    special_skill: 0,
                    turn,
                    passive_skill: 0,
                    passive_skill_lv: 0,
                    saddle_kind: 0,
                    upgrade_item_id: 0,
                    upgrade_skill: 0,
                    limits: [0, 0],
                    ride_offset_h: 0,
                    pic_offset_x: 0,
                    pic_offset_y: 0,
                    hud_offset_h: 0,
                    shadow_kind: 0,
                    rare: 0,
                    garble,
                },
            );
        }

        Ok(result)
    }
}
