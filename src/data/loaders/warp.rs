//! Warp loader for `Warp.Dat` and `Warp_C.dat`.

use crate::data::reader::DatReader;
use crate::error::{Result, TsError};
use std::collections::HashMap;

/// Warp point definition.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct WarpDef {
    pub name: u32,
    pub scene: u16,
    pub mark: u16,
    pub x: i32,
    pub y: i32,
}

pub struct WarpDatLoader;

impl WarpDatLoader {
    /// Load warp definitions from `Warp.Dat` (PC binary) or `Warp_C.dat` (mobile binary).
    pub fn load(bytes: &[u8]) -> Result<HashMap<usize, WarpDef>> {
        if bytes.len() < 16 {
            return Err(TsError::Data("Warp binary buffer too small".to_string()));
        }

        let mut result = HashMap::new();

        // Check if mobile format (starts with non-zero count or 16-byte records)
        let count = i32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
        if count > 0 && bytes.len() == 4 + (count as usize * 16) {
            let mut reader = DatReader::new(bytes.to_vec());
            let count = reader.read_i32() as usize;
            for i in 0..count {
                let warp = WarpDef {
                    name: reader.read_u32_raw(),
                    scene: reader.read_u16_raw(),
                    mark: reader.read_u16_raw(),
                    x: reader.read_i32(),
                    y: reader.read_i32(),
                };
                result.insert(i, warp);
            }
            return Ok(result);
        }

        // PC format (690 bytes or 23 bytes per record with DecodeAll)
        if bytes.len() >= 23 * 3 {
            let count = (bytes.len() / 23).saturating_sub(3);
            let mut reader = DatReader::new(bytes.to_vec());
            reader.decode_all(Some(23), Some(count));

            let mut idx = 0;
            while reader.can_read() && reader.remaining() >= 23 {
                let name = reader.read_u32_raw();
                let scene = reader.read_u16_raw();
                let mark = reader.read_u16_raw();
                let x = reader.read_i32();
                let y = reader.read_i32();
                // 3 padding bytes
                let _ = reader.read_bytes(3);

                result.insert(
                    idx,
                    WarpDef {
                        name,
                        scene,
                        mark,
                        x,
                        y,
                    },
                );
                idx += 1;
            }
        }

        Ok(result)
    }
}
