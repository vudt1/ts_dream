//! Ground map-terrain loader for `ground_server.mmg` / `ground.mmg` (unencrypted).
//!
//! Source lives in repo root `Data/ground_server.mmg` (stripped server extract
//! of client `ground.mmg`, keeping `width`, `height`, and `geolBaseAtt`); see
//! `resolve_data_file`.
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

use crate::error::Result;
use std::collections::HashMap;

/// Extracted per-map metadata from `ground_server.mmg` / `ground.mmg`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GroundMapMeta {
    pub width: i32,
    pub height: i32,
    pub geol_base_att: u8,
}

type GroundIndexEntry = (u32, [u8; 21], usize, usize);

/// Loads `ground_server.mmg` / `ground.mmg` directly from `&[u8]` without
/// per-map buffer cloning.
pub struct GroundMmgLoader;

impl GroundMmgLoader {
    /// Index entry size: name(1 + 20) + position(4) + size(4) = 29.
    pub const INDEX_ENTRY_SIZE: usize = 29;
    /// Name field size for fixed ASCII.
    pub const NAME_FIELD_SIZE: usize = 20;
    /// Stripped server body size: width(4) + height(4) + pic(1) + bw(2) + bh(2)
    /// + wave(2) + elem(2) + npc(1) + geolBaseAtt(1) = 19 bytes.
    pub const SERVER_BODY_SIZE: usize = 19;

    /// Load `map_id -> geolBaseAtt` (entries with `geolBaseAtt == 0` are dropped).
    pub fn load(bytes: &[u8]) -> Result<HashMap<u32, u8>> {
        let metas = Self::load_with_meta(bytes)?;
        Ok(metas
            .into_iter()
            .filter(|(_, m)| m.geol_base_att > 0)
            .map(|(id, m)| (id, m.geol_base_att))
            .collect())
    }

    /// Load `map_id -> GroundMapMeta { width, height, geol_base_att }` for all
    /// valid indexed map bodies.
    pub fn load_with_meta(bytes: &[u8]) -> Result<HashMap<u32, GroundMapMeta>> {
        let mut result = HashMap::new();
        let Some((entries, _)) = Self::parse_index(bytes) else {
            return Ok(result);
        };

        for (map_id, _raw_name, position, size) in entries {
            if let Some(meta) = Self::read_map_meta(bytes, position, size) {
                result.insert(map_id, meta);
            }
        }
        Ok(result)
    }

    /// Extract a minimal `ground_server.mmg` buffer in `MapData.New` format
    /// (keeping `width`, `height`, `geolBaseAtt` and zeroing client visual/block
    /// arrays). Returns `(buffer, map_count)`.
    pub fn extract_server_mmg(bytes: &[u8]) -> Result<(Vec<u8>, usize)> {
        let Some((entries, _)) = Self::parse_index(bytes) else {
            return Ok((vec![0, 0], 0));
        };

        let mut out_bodies = Vec::with_capacity(entries.len() * Self::SERVER_BODY_SIZE);
        let mut out_index: Vec<([u8; 21], i32, i32)> = Vec::with_capacity(entries.len());

        for (_map_id, raw_name, position, size) in entries {
            let Some(meta) = Self::read_map_meta(bytes, position, size) else {
                continue;
            };
            let body_pos = out_bodies.len() as i32;
            out_bodies.extend_from_slice(&meta.width.to_le_bytes());
            out_bodies.extend_from_slice(&meta.height.to_le_bytes());
            out_bodies.push(0); // mapPicCount = 0
            out_bodies.extend_from_slice(&0u16.to_le_bytes()); // blockWidth = 0
            out_bodies.extend_from_slice(&0u16.to_le_bytes()); // blockHeight = 0
            out_bodies.extend_from_slice(&0u16.to_le_bytes()); // waveCount = 0
            out_bodies.extend_from_slice(&0u16.to_le_bytes()); // elementCount = 0
            out_bodies.push(0); // npcCount = 0
            out_bodies.push(meta.geol_base_att);
            out_index.push((raw_name, body_pos, Self::SERVER_BODY_SIZE as i32));
        }

        let count = out_index.len();
        let mut out =
            Vec::with_capacity(out_bodies.len() + count * Self::INDEX_ENTRY_SIZE + 2);
        out.extend_from_slice(&out_bodies);
        for (raw_name, pos, sz) in &out_index {
            out.extend_from_slice(raw_name);
            out.extend_from_slice(&pos.to_le_bytes());
            out.extend_from_slice(&sz.to_le_bytes());
        }
        out.extend_from_slice(&(count as u16).to_le_bytes());
        Ok((out, count))
    }

    fn parse_index(bytes: &[u8]) -> Option<(Vec<GroundIndexEntry>, usize)> {
        if bytes.len() < 2 {
            return None;
        }
        let count = read_u16_le(bytes, bytes.len() - 2)? as usize;
        let index_start = bytes
            .len()
            .checked_sub(2 + count.saturating_mul(Self::INDEX_ENTRY_SIZE))?;

        let mut entries = Vec::with_capacity(count.min(100_000));
        let mut pos = index_start;
        for _ in 0..count {
            if pos + Self::INDEX_ENTRY_SIZE > bytes.len() - 2 {
                break;
            }
            let raw_name: [u8; 21] = bytes[pos..pos + 21].try_into().ok()?;
            let name_len = raw_name[0] as usize;
            let name = if name_len > 0 && name_len <= Self::NAME_FIELD_SIZE {
                String::from_utf8_lossy(&raw_name[1..1 + name_len])
            } else {
                std::borrow::Cow::Borrowed("")
            };
            let position = read_i32_le(bytes, pos + 21)?;
            let size = read_i32_le(bytes, pos + 25)?;
            pos += Self::INDEX_ENTRY_SIZE;

            let map_id = name
                .split_once(".map")
                .and_then(|(head, _)| head.parse::<u32>().ok());
            if let Some(map_id) = map_id {
                if size > 0 && position >= 0 {
                    entries.push((map_id, raw_name, position as usize, size as usize));
                }
            }
        }
        Some((entries, count))
    }

    /// Walk a map body in `MapData.New` field order directly on `&[u8]`;
    /// `None` when the walk would overrun the body bounds.
    fn read_map_meta(bytes: &[u8], position: usize, size: usize) -> Option<GroundMapMeta> {
        let end = position.checked_add(size)?;
        if end > bytes.len() {
            return None;
        }
        let mut pos = position;
        if pos.checked_add(8)? > end {
            return None;
        }
        let width = read_i32_le(bytes, pos)?;
        let height = read_i32_le(bytes, pos + 4)?;
        pos += 8;

        let map_pic_count = *bytes.get(pos)?;
        pos += 1;
        if pos.checked_add(1 + map_pic_count as usize * 6)? > end + 1 {
            return None;
        }
        pos = pos.checked_add(map_pic_count as usize * 6)?;
        if pos > end {
            return None;
        }

        if pos.checked_add(4)? > end {
            return None;
        }
        let bw = read_u16_le(bytes, pos)? as usize;
        let bh = read_u16_le(bytes, pos + 2)? as usize;
        pos += 4;
        pos = pos.checked_add(bw.checked_mul(bh)?)?;
        if pos > end {
            return None;
        }

        if pos.checked_add(2)? > end {
            return None;
        }
        let wave_count = read_u16_le(bytes, pos)? as usize;
        pos += 2;
        pos = pos.checked_add(wave_count.checked_mul(6)?)?;
        if pos > end {
            return None;
        }

        if pos.checked_add(2)? > end {
            return None;
        }
        let element_count = read_u16_le(bytes, pos)? as usize;
        pos += 2;
        pos = pos.checked_add(element_count.checked_mul(8)?)?;
        if pos > end {
            return None;
        }

        // npcCount(u8) + geolBaseAtt(u8)
        if pos.checked_add(2)? > end {
            return None;
        }
        let geol_base_att = *bytes.get(pos + 1)?;
        Some(GroundMapMeta {
            width,
            height,
            geol_base_att,
        })
    }
}

#[inline]
fn read_u16_le(bytes: &[u8], pos: usize) -> Option<u16> {
    let slice = bytes.get(pos..pos + 2)?;
    Some(u16::from_le_bytes([slice[0], slice[1]]))
}

#[inline]
fn read_i32_le(bytes: &[u8], pos: usize) -> Option<i32> {
    let slice = bytes.get(pos..pos + 4)?;
    Some(i32::from_le_bytes([slice[0], slice[1], slice[2], slice[3]]))
}
