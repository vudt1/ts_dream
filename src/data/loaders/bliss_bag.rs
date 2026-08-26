//! BlissBag loader for `BlissBag.Dat`.
//!
//! Decodes lucky box / fortune bag item drop rates and item packages.

use crate::error::{Result, TsError};
use std::collections::HashMap;

/// An individual reward item entry inside a bliss bag.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct BlissBagItem {
    pub item_id: u16,
    pub count: u32,
    pub pr: u32,
    pub kind: u8,
}

/// A bliss bag definition containing all its reward items.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct BlissBagDef {
    pub bag_item_id: u16,
    pub kind_count: u8,
    pub items: Vec<BlissBagItem>,
}

pub struct BlissBagDatLoader;

impl BlissBagDatLoader {
    const FIELD_LENGTH: usize = 190;

    fn decode_u16(val: u16) -> u16 {
        (val ^ 0xECEA).wrapping_sub(9)
    }

    fn decode_u8(val: u8) -> u8 {
        (val ^ 0xFD).wrapping_sub(9)
    }

    /// Load all bliss bags from binary slice of `BlissBag.Dat`.
    pub fn load(bytes: &[u8]) -> Result<HashMap<u16, BlissBagDef>> {
        if !bytes.len().is_multiple_of(Self::FIELD_LENGTH) {
            return Err(TsError::Data(format!(
                "Invalid BlissBag.Dat length {} (not a multiple of {})",
                bytes.len(),
                Self::FIELD_LENGTH
            )));
        }

        let num_records = bytes.len() / Self::FIELD_LENGTH;
        let mut result = HashMap::new();

        // Record 0 is usually dummy, start from 1
        for i in 1..num_records {
            let offset = i * Self::FIELD_LENGTH;
            let chunk = &bytes[offset..offset + Self::FIELD_LENGTH];

            let raw_boxid = u16::from_le_bytes([chunk[0], chunk[1]]);
            let box_id = Self::decode_u16(raw_boxid);
            let nb_items = chunk[2] as usize;
            let nb_getitems = Self::decode_u8(chunk[3]);
            let items_data = &chunk[4..];

            let mut items = Vec::new();
            let mut num2 = 6;
            for _ in 0..nb_items {
                if num2 + 6 > items_data.len() {
                    break;
                }
                let raw_item = u16::from_le_bytes([items_data[num2], items_data[num2 + 1]]);
                let item_id = Self::decode_u16(raw_item);
                let qty = Self::decode_u8(items_data[num2 + 2]);
                let raw_odds = u16::from_le_bytes([items_data[num2 + 3], items_data[num2 + 4]]);
                let odds = Self::decode_u16(raw_odds);
                let nb_get = Self::decode_u8(items_data[num2 + 5]);

                items.push(BlissBagItem {
                    item_id,
                    count: qty as u32,
                    pr: odds as u32,
                    kind: nb_get,
                });
                num2 += 6;
            }

            if box_id != 0 {
                result.insert(
                    box_id,
                    BlissBagDef {
                        bag_item_id: box_id,
                        kind_count: nb_getitems,
                        items,
                    },
                );
            }
        }

        Ok(result)
    }
}
