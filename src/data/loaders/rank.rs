//! Rank loader for `Rank.Dat` (PC fixed-record binary).
//!
//! Spec: `ts_mobile_client/Data/RankData.lua` (`RankData.New`) +
//! `ts_mobile_client/Logic/DataManager.lua::OnLoadRankData`.
//!
//! Record layout (43 bytes, `DatReader` keys `offset=9, xor1=0xFD,
//! `xor2=0xECEA`, `xor4=0x0B80F4B4` — the same key family as
//! `Item.dat`/`BlissBag.Dat`):
//! ```text
//! name(1 + 20 bytes, PC reversed string, no XOR)
//! + honor(u16, required battle-honor)
//! + 4 × (kind: u8, value: i32)  (rank attribute bonuses)
//! ```
//! Record 0 is an all-zero dummy (same convention as `Item.dat`/`Npc.dat`);
//! it is skipped via the empty-name guard. Map key is the 1-based record
//! index, mirroring the Lua loop counter (`rankDatas[id]`).

use crate::data::reader::DatReader;
use crate::encoding;
use crate::error::Result;
use std::collections::HashMap;

/// A military-rank definition (`Data_Ranks`), from PC `Rank.Dat`.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct RankDef {
    pub name: String,
    pub name_viscii: Vec<u8>,
    /// Required battle-honor (`honor`, `ReadUInt16`).
    pub honor: u16,
    /// Rank attribute bonuses: `(kind, value)` × 4, consumed by
    /// `RankData.GetAttribute(honor, kind)` (e.g. EquipMaxHp/EquipMaxSp).
    pub attributes: [(u8, i32); 4],
}

pub struct RankDatLoader;

impl RankDatLoader {
    pub const RECORD_SIZE: usize = 43;
    pub const ATTRIBUTE_COUNT: usize = 4;

    /// XOR/offset keys from `DataManager.OnLoadRankData`:
    /// `DatReader.New(file, 9, 253, 60650, 193000628)`.
    pub fn load(bytes: &[u8]) -> Result<HashMap<u16, RankDef>> {
        let mut reader = DatReader::with_keys(bytes.to_vec(), 9, 0xFD, 0xECEA, 0x0B80_F4B4);
        let mut result = HashMap::new();
        let mut index: u16 = 0;
        while reader.remaining() >= Self::RECORD_SIZE {
            index = index.saturating_add(1);
            let name_viscii = reader.read_string_bytes(20);
            let honor = reader.read_u16();
            let mut attributes = [(0u8, 0i32); Self::ATTRIBUTE_COUNT];
            for attr in attributes.iter_mut() {
                *attr = (reader.read_u8(), reader.read_i32());
            }
            // Dummy record 0 (and any nameless padding) carries no rank.
            if name_viscii.is_empty() {
                continue;
            }
            result.insert(
                index,
                RankDef {
                    name: encoding::cp1252_to_string(&name_viscii),
                    name_viscii,
                    honor,
                    attributes,
                },
            );
        }
        Ok(result)
    }
}
