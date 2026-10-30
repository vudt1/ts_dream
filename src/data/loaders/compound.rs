//! Compound / Crafting formula loader for `Compound.Dat`.

use crate::data::reader::DatReader;
use crate::error::{Result, TsError};

/// A crafting / compounding formula recipe.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct CompoundDef {
    /// Resulting equipment kind (1..52, maps to `item.kind`).
    pub result_kind: u16,
    /// Sorted material codes [mat0, mat1, mat2] (values in 0..=37).
    pub materials: [u8; 3],
    /// Matching sorted required degrees [deg0, deg1, deg2].
    pub degrees: [u8; 3],
    /// 32-bit lookup key constructed by concatenating sorted materials as decimal strings.
    pub lookup_key: u32,
}

pub struct CompoundDatLoader;

impl CompoundDatLoader {
    /// Compute the decimal lookup key and sorted materials/degrees according to Delphi `FUN_005F2528`.
    pub fn compute_key_and_sort(
        raw_mats: [u8; 3],
        raw_degs: [u8; 3],
    ) -> (u32, [u8; 3], [u8; 3]) {
        let mut mats = raw_mats;
        let mut degs = raw_degs;

        // Exchange sort mats and degs ascending by mat
        for i in 0..3 {
            for j in 0..3 {
                if i < j && mats[j] < mats[i] {
                    mats.swap(i, j);
                    degs.swap(i, j);
                }
            }
        }

        let key_str = format!("{}{}{}", mats[0], mats[1], mats[2]);
        let key = key_str.parse::<u32>().unwrap_or(0);
        (key, mats, degs)
    }

    /// Load all compounding recipes from binary slice of `Compound.Dat`.
    ///
    /// Skips the 8-byte header slot 0 (u16 magic = 2) if present,
    /// parses 1,393 records, computes lookup_key, and returns them sorted by lookup_key ascending.
    pub fn load(bytes: &[u8]) -> Result<Vec<CompoundDef>> {
        if !bytes.len().is_multiple_of(8) {
            return Err(TsError::Data(format!(
                "Invalid Compound.Dat length {} (not a multiple of 8)",
                bytes.len()
            )));
        }

        // Check if 8-byte header is present: slot 0 has raw bytes where u16 LE == 2.
        let skip_header = bytes.len() >= 8 && u16::from_le_bytes([bytes[0], bytes[1]]) == 2;
        let data_slice = if skip_header {
            &bytes[8..]
        } else {
            bytes
        };

        let mut reader = DatReader::with_keys(data_slice.to_vec(), 3, 211, 64444, 0);
        let mut list = Vec::new();

        while reader.can_read() && reader.remaining() >= 8 {
            let result_kind = reader.read_u16();
            let material_a = reader.read_u8();
            let material_b = reader.read_u8();
            let material_c = reader.read_u8();
            let a_degree = reader.read_u8();
            let b_degree = reader.read_u8();
            let c_degree = reader.read_u8();

            let (lookup_key, materials, degrees) = Self::compute_key_and_sort(
                [material_a, material_b, material_c],
                [a_degree, b_degree, c_degree],
            );

            list.push(CompoundDef {
                result_kind,
                materials,
                degrees,
                lookup_key,
            });
        }

        // Sort by lookup_key ascending (stable sort) matching Delphi secondary table order
        list.sort_by_key(|r| r.lookup_key);

        Ok(list)
    }
}
