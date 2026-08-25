//! Compound / Crafting formula loader for `Compound.Dat`.

use crate::data::reader::DatReader;
use crate::error::{Result, TsError};

/// A crafting / compounding formula recipe.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct CompoundDef {
    pub compound_d: u16,
    pub material_a: u8,
    pub material_b: u8,
    pub material_c: u8,
    pub a_degree: u8,
    pub b_degree: u8,
    pub c_degree: u8,
}

pub struct CompoundDatLoader;

impl CompoundDatLoader {
    /// Load all compounding recipes from binary slice of `Compound.Dat`.
    pub fn load(bytes: &[u8]) -> Result<Vec<CompoundDef>> {
        if bytes.len() % 8 != 0 {
            return Err(TsError::Data(format!(
                "Invalid Compound.Dat length {} (not a multiple of 8)",
                bytes.len()
            )));
        }

        let mut reader = DatReader::with_keys(bytes.to_vec(), 3, 211, 64444, 168229221);
        let mut list = Vec::new();

        while reader.can_read() && reader.remaining() >= 8 {
            let compound_d = reader.read_u16();
            let material_a = reader.read_u8();
            let material_b = reader.read_u8();
            let material_c = reader.read_u8();
            let a_degree = reader.read_u8();
            let b_degree = reader.read_u8();
            let c_degree = reader.read_u8();

            list.push(CompoundDef {
                compound_d,
                material_a,
                material_b,
                material_c,
                a_degree,
                b_degree,
                c_degree,
            });
        }

        Ok(list)
    }
}
