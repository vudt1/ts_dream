//! Feather socketing engine for elemental upgrades (+1..+10).

use crate::crafting::error::CraftError;
use crate::crafting::socketing::rate_table::socket_success_rate;
use crate::server::session::InventoryItem;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FeatherInfo {
    pub id: u16,
    /// Element: 1=Earth (Địa), 2=Water (Thủy), 3=Fire (Hỏa), 4=Wind (Phong), 0=Universal (Đồng Thoại Vũ)
    pub element: u8,
    /// Feather level: 1..=10
    pub level: u8,
}

/// Identifies if `item_id` is a socketable feather and extracts its element and level.
pub fn parse_feather(id: u16) -> Option<FeatherInfo> {
    match id {
        24021 => Some(FeatherInfo { id, element: 1, level: 1 }),
        24022 => Some(FeatherInfo { id, element: 2, level: 1 }),
        24023 => Some(FeatherInfo { id, element: 3, level: 1 }),
        24024 => Some(FeatherInfo { id, element: 4, level: 1 }),
        24025 => Some(FeatherInfo { id, element: 1, level: 2 }),
        24026 => Some(FeatherInfo { id, element: 2, level: 2 }),
        24027 => Some(FeatherInfo { id, element: 3, level: 2 }),
        24028 => Some(FeatherInfo { id, element: 4, level: 2 }),
        24031..=24039 => {
            let level = (id - 24031 + 1) as u8;
            Some(FeatherInfo { id, element: 0, level })
        }
        24041 => Some(FeatherInfo { id, element: 1, level: 7 }),
        24042 => Some(FeatherInfo { id, element: 2, level: 7 }),
        24043 => Some(FeatherInfo { id, element: 3, level: 7 }),
        24044 => Some(FeatherInfo { id, element: 4, level: 7 }),
        24052 => Some(FeatherInfo { id, element: 1, level: 8 }),
        24053 => Some(FeatherInfo { id, element: 2, level: 8 }),
        24054 => Some(FeatherInfo { id, element: 3, level: 8 }),
        24055 => Some(FeatherInfo { id, element: 4, level: 8 }),
        24056 => Some(FeatherInfo { id, element: 1, level: 9 }),
        24057 => Some(FeatherInfo { id, element: 2, level: 9 }),
        24058 => Some(FeatherInfo { id, element: 3, level: 9 }),
        24059 => Some(FeatherInfo { id, element: 4, level: 9 }),
        24064 => Some(FeatherInfo { id, element: 0, level: 9 }),
        24060 => Some(FeatherInfo { id, element: 1, level: 10 }),
        24061 => Some(FeatherInfo { id, element: 2, level: 10 }),
        24062 => Some(FeatherInfo { id, element: 3, level: 10 }),
        24063 => Some(FeatherInfo { id, element: 4, level: 10 }),
        24065 => Some(FeatherInfo { id, element: 0, level: 10 }),
        _ => None,
    }
}

/// Outcome of a feather socketing attempt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SocketOutcome {
    pub success: bool,
    pub new_element: u8,
    pub new_element_value: u8,
    pub is_element_switched: bool,
}

/// Executes feather socketing on equipment.
///
/// RNG is provided as `roll` in 0..100 (or `rng_roll` function).
pub fn socket_feather(
    equip: &mut InventoryItem,
    equip_kind: u8,
    feather_id: u16,
    rng_roll_100: u32, // 0..99 or 1..100
) -> Result<SocketOutcome, CraftError> {
    // 1. Verify equipment type: kind in 1..=14
    if !(1..=14).contains(&equip_kind) {
        return Err(CraftError::InvalidEquipmentType);
    }

    // 2. Parse feather
    let feather = parse_feather(feather_id).ok_or(CraftError::InvalidFeather)?;

    // 3. Case A: Equipment currently has no element (element == 0)
    if equip.long_val == 0 {
        // Must be an elemental feather (Earth/Water/Fire/Wind)
        if feather.element == 0 {
            return Err(CraftError::InvalidFeather);
        }
        // Must be a level 1 feather
        if feather.level != 1 {
            return Err(CraftError::InvalidFeatherLevel);
        }
        equip.long_val = feather.element;
        equip.giatri_long = 1;

        return Ok(SocketOutcome {
            success: true,
            new_element: feather.element,
            new_element_value: 1,
            is_element_switched: false,
        });
    }

    // 4. Case B: Switching element (different element feather used on equipment)
    if feather.element > 0 && feather.element != equip.long_val {
        if feather.level != 1 {
            return Err(CraftError::InvalidFeatherLevel);
        }
        equip.long_val = feather.element;
        equip.giatri_long = 1;

        return Ok(SocketOutcome {
            success: true,
            new_element: feather.element,
            new_element_value: 1,
            is_element_switched: true,
        });
    }

    // 5. Case C: Upgrading existing element (same element feather or universal feather)
    let current_level = equip.giatri_long;
    if current_level >= 10 {
        return Err(CraftError::EquipmentMaxLevel);
    }
    let target_level = current_level + 1;

    // Requires feather level to match target_level
    if feather.level != target_level {
        return Err(CraftError::InvalidFeatherLevel);
    }

    let rate = socket_success_rate(target_level);
    // Success check: roll in 0..100 is strictly less than rate (e.g. rate 80 -> 0..79 succeeds)
    let roll = rng_roll_100 % 100;
    let success = roll < rate;

    if success {
        equip.giatri_long = target_level;
        Ok(SocketOutcome {
            success: true,
            new_element: equip.long_val,
            new_element_value: target_level,
            is_element_switched: false,
        })
    } else {
        // Penalty on failure:
        // <= 3: keep current level
        // >= 4: decrease by 1 level
        if current_level >= 4 {
            equip.giatri_long = current_level - 1;
        }
        Ok(SocketOutcome {
            success: false,
            new_element: equip.long_val,
            new_element_value: equip.giatri_long,
            is_element_switched: false,
        })
    }
}
