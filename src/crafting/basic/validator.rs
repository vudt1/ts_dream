//! Input validation for basic compound operations according to Delphi `FUN_0079A5E4`.

use crate::crafting::error::CraftError;
use crate::data::loader::GameData;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct CraftInputSlot {
    pub slot: u8,
    pub item_id: u16,
    pub qty: u8,
}

impl CraftInputSlot {
    pub fn new(slot: u8, item_id: u16, qty: u8) -> Self {
        Self { slot, item_id, qty }
    }
}

/// Validates 3 input slots for compounding.
///
/// Returns a list of valid (original_index, CraftInputSlot) pairs if valid,
/// or a `CraftError` if any check fails.
pub fn validate_compound_inputs(
    slots: &[CraftInputSlot; 3],
    game_data: &GameData,
) -> Result<Vec<(usize, CraftInputSlot)>, CraftError> {
    let mut valid_slots = Vec::with_capacity(3);

    for (idx, slot) in slots.iter().enumerate() {
        if slot.item_id > 0 && slot.qty > 0 {
            valid_slots.push((idx, *slot));
        }
    }

    // Rule 1: Need at least 2 valid items (FUN_0079A5E4 code 1)
    if valid_slots.len() < 2 {
        return Err(CraftError::NeedAtLeastTwoItems);
    }

    // Rule 2 & 3: Validate each active slot
    for (_, slot) in &valid_slots {
        // ID range: 10001..=50000
        if !(10001..=50000).contains(&slot.item_id) {
            return Err(CraftError::InvalidItemForCompound);
        }

        // Must exist in Item.dat catalog
        let def = game_data
            .item_defs
            .get(&slot.item_id)
            .ok_or(CraftError::InvalidItemForCompound)?;

        // Prohibited item kinds: {20, 21, 22}
        if (20..=22).contains(&def.kind) {
            return Err(CraftError::InvalidItemForCompound);
        }

        // Single slot quantity limit: <= 50 (FUN_0079A5E4 code 3)
        if slot.qty > 50 {
            return Err(CraftError::QuantityExceeded);
        }
    }

    Ok(valid_slots)
}
