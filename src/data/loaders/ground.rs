//! Ground map-terrain loader for `ground.mmg` (unencrypted).
//!
//! Port of `ts_mobile_server/.../data/loaders/GroundMmgLoader.kt`, itself a
//! port of `ts_mobile_client/Logic/DataManager.lua::OnLoadMapData`.
//! Source lives outside `Data/` (`CompreseData/Ground.mmg` on mobile, repo
//! root `Data/ground.mmg` here); see `resolve_data_file`.
//!
//! File layout:
//! ```text
//! bodies... + index(count × 29B) + count(u16)
//! index entry: name(1 + 20 bytes fixed ASCII, e.g. "12001.map")
//!   + position(i32) + size(i32)
//! body (MapData.New order): width(i32) + height(i32)
//!   + mapPicCount(u8) + pics(count × 6B)
//!   + blockWidth(u16) + blockHeight(u16) + blocks(w × h bytes)
//!   + waveCount(u16) + waves(count × 6B)
//!   + elementCount(u16) + elements(count × 8B)
//!   + npcCount(u8) + geolBaseAtt(u8)  ← extracted value
//! ```
//!
//! Reference-only parser: intentionally NOT wired into `GameData::load`
//! (undecided whether the server needs `ground.mmg` data). Call
//! `GroundMmgLoader::load` directly when a consumer lands.

use crate::data::reader::DatReader;
use crate::error::Result;
use std::collections::HashMap;

/// Loads `ground.mmg`, returning `map_id → geolBaseAtt` (entries with
/// `geolBaseAtt == 0` are dropped, matching the Kotlin loader).
pub struct GroundMmgLoader;

impl GroundMmgLoader {
    /// Index entry size: name(1 + 20) + position(4) + size(4) = 29.
    pub const INDEX_ENTRY_SIZE: usize = 29;
    /// Name field size for `read_fixed_ascii`.
    pub const NAME_FIELD_SIZE: usize = 20;

    pub fn load(bytes: &[u8]) -> Result<HashMap<u32, u8>> {
        let mut result = HashMap::new();
        if bytes.len() < 2 {
            return Ok(result);
        }
        let mut reader = DatReader::new(bytes.to_vec());

        // Tail index (mirrors `OnLoadMapData`: seek to `length - 2`).
        reader.seek(bytes.len() - 2);
        let count = reader.read_u16_raw() as usize;
        let index_start = bytes
            .len()
            .checked_sub(2 + count.saturating_mul(Self::INDEX_ENTRY_SIZE));
        let Some(index_start) = index_start else {
            return Ok(result);
        };

        // Parse index entries.
        let mut entries = Vec::with_capacity(count.min(100_000));
        reader.seek(index_start);
        for _ in 0..count {
            if reader.remaining() < Self::INDEX_ENTRY_SIZE {
                break;
            }
            let name = reader.read_fixed_ascii(Self::NAME_FIELD_SIZE);
            // `DatReader::new` carries zero keys, so plain reads are raw.
            let position = reader.read_i32();
            let size = reader.read_i32();
            // Name looks like "12001.map" — extract the map id.
            let map_id = name
                .split_once(".map")
                .and_then(|(head, _)| head.parse::<u32>().ok());
            if let Some(map_id) = map_id {
                if size > 0 && position >= 0 {
                    entries.push((map_id, position as usize, size as usize));
                }
            }
        }

        // Walk each body for `geolBaseAtt`; malformed bodies are skipped
        // (Kotlin parity: per-entry try/catch, `geolBaseAtt > 0` only).
        for (map_id, position, size) in entries {
            if let Some(geol) = Self::read_geol_base_att(bytes, position, size) {
                if geol > 0 {
                    result.insert(map_id, geol);
                }
            }
        }
        Ok(result)
    }

    /// Walk a map body in `MapData.New` field order; `None` when the walk
    /// would overrun the body bounds.
    fn read_geol_base_att(bytes: &[u8], position: usize, size: usize) -> Option<u8> {
        let end = position.checked_add(size)?;
        if end > bytes.len() {
            return None;
        }
        let mut r = DatReader::new(bytes.to_vec());
        r.seek(position);

        need(&mut r, end, 8)?; // width(i32) + height(i32)
        let map_pic_count = u8_at(&mut r, end)?;
        need(&mut r, end, map_pic_count as usize * 6)?; // pics: picId(2) + posX(2) + posY(2)
        // blockWidth(u16) + blockHeight(u16), then w × h block bytes.
        if r.position() + 4 > end {
            return None;
        }
        let bw = r.read_u16_raw() as usize;
        let bh = r.read_u16_raw() as usize;
        need(&mut r, end, bw.checked_mul(bh)?)?;
        if r.position() + 2 > end {
            return None;
        }
        let wave_count = r.read_u16_raw() as usize;
        need(&mut r, end, wave_count.checked_mul(6)?)?; // waves: blockX(2)+blockY(2)+soundId(1)+dist(1)
        if r.position() + 2 > end {
            return None;
        }
        let element_count = r.read_u16_raw() as usize;
        need(&mut r, end, element_count.checked_mul(8)?)?; // elements: picId(4)+posX(2)+posY(2)
        need(&mut r, end, 1)?; // npcCount(u8)
        u8_at(&mut r, end) // geolBaseAtt(u8)
    }
}

/// Require `n` body bytes, then skip them.
fn need(r: &mut DatReader, end: usize, n: usize) -> Option<()> {
    if r.position().checked_add(n)? > end {
        return None;
    }
    r.skip(n);
    Some(())
}

/// Read one raw byte inside the body bounds.
fn u8_at(r: &mut DatReader, end: usize) -> Option<u8> {
    if r.position() + 1 > end || r.position() + 1 > r.len() {
        return None;
    }
    Some(r.read_u8_raw())
}
