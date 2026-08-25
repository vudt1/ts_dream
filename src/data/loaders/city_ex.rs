//! CityEx loader for `CityEx.Dat`.

use crate::data::reader::DatReader;
use crate::error::{Result, TsError};

/// City extra data row containing double precision coefficients.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct CityExDef {
    pub data: [f64; 7],
}

pub struct CityExDatLoader;

impl CityExDatLoader {
    const RECORD_SIZE: usize = 56;

    /// Load CityEx data rows from binary slice of `CityEx.Dat`.
    pub fn load(bytes: &[u8]) -> Result<Vec<CityExDef>> {
        if bytes.len() < Self::RECORD_SIZE * 3 {
            return Err(TsError::Data("CityEx.Dat buffer too small".to_string()));
        }

        let count = (bytes.len() / Self::RECORD_SIZE).saturating_sub(3);
        let mut reader = DatReader::new(bytes.to_vec());
        reader.decode_all(Some(Self::RECORD_SIZE), Some(count));

        let mut list = Vec::new();
        while reader.can_read() && reader.remaining() >= Self::RECORD_SIZE {
            let mut row = [0.0f64; 7];
            for val in &mut row {
                *val = reader.read_f64();
            }
            list.push(CityExDef { data: row });
        }

        Ok(list)
    }
}
