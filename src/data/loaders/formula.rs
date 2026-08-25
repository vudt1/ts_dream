//! Formula loader for `Formula.Dat`.
//!
//! Maps to Kotlin `FormulaDatLoader` / `Calculator.lua`.

use crate::data::reader::DatReader;
use crate::error::{Result, TsError};

/// Parameters from `Formula.Dat` used for stat, HP, SP, and damage calculations.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct FormulaParams {
    pub w1: f64,
    pub w2: f64,
    pub w3: f64,
    pub w4: f64,
    pub w5: f64,
    pub w6: f64,
    pub w7: f64,
    pub w9: f64,
    pub w10: f64,
    pub w13: f64,
    pub w14: f64,
    pub w15: f64,
    pub w16: f64,
    pub w20: f64,
    pub w21: f64,
    pub w25: f64,
    pub w28: f64,
    pub w30: f64,
    pub w31: f64,
    pub w35: f64,
    pub w36: f64,
    pub extra_exp: i32,
    pub base_hp: u16,
    pub base_sp: u16,
    pub w37: f64,
    pub w40: f64,
    pub w38: f64,
    pub w39: f64,
    pub w45: f64,
    pub w50: f64,
    pub w51: f64,
    pub att_agi_scope: u16,
    pub ran: u8,
}

pub struct FormulaDatLoader;

impl FormulaDatLoader {
    /// Load `FormulaParams` from byte slice of `Formula.Dat`.
    pub fn load(bytes: &[u8]) -> Result<FormulaParams> {
        if bytes.len() < 235 {
            return Err(TsError::Data(format!(
                "Formula.Dat buffer too small: {} bytes (expected >= 235)",
                bytes.len()
            )));
        }
        let mut reader = DatReader::new(bytes.to_vec());
        Ok(FormulaParams {
            w1: reader.read_f64(),
            w2: reader.read_f64(),
            w3: reader.read_f64(),
            w4: reader.read_f64(),
            w5: reader.read_f64(),
            w6: reader.read_f64(),
            w7: reader.read_f64(),
            w9: reader.read_f64(),
            w10: reader.read_f64(),
            w13: reader.read_f64(),
            w14: reader.read_f64(),
            w15: reader.read_f64(),
            w16: reader.read_f64(),
            w20: reader.read_f64(),
            w21: reader.read_f64(),
            w25: reader.read_f64(),
            w28: reader.read_f64(),
            w30: reader.read_f64(),
            w31: reader.read_f64(),
            w35: reader.read_f64(),
            w36: reader.read_f64(),
            extra_exp: reader.read_i32(),
            base_hp: reader.read_u16(),
            base_sp: reader.read_u16(),
            w37: reader.read_f64(),
            w40: reader.read_f64(),
            w38: reader.read_f64(),
            w39: reader.read_f64(),
            w45: reader.read_f64(),
            w50: reader.read_f64(),
            w51: reader.read_f64(),
            att_agi_scope: reader.read_u16(),
            ran: reader.read_u8(),
        })
    }
}
