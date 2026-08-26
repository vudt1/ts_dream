//! Inventory rules: stacking, slot bounds, add/remove.
//!
//! The slot/stack invariants of a homdo-style bag (25 slots, stack ≤ 255) live
//! here so every mutation path shares them.

use crate::server::session::InventoryItem;

/// The number of usable bag slots (1-based indices `1..=25`).
pub const HOMDO_SLOTS: u8 = 25;

/// Per-slot stack cap for mergeable items (hard-coded `50` in the bag
/// add/move rules).
pub const STACK_CAP: u16 = 50;

/// Build an inventory item from its static template (`Data.Item`), carrying the
/// base `_1`/`_2` stats, element and elemental bonus. Falls back to a bare
/// id/count item when the id is not in the static table.
pub fn from_template(data: &crate::data::loader::GameData, id: u16, count: u8) -> InventoryItem {
    data.items
        .get(&i64::from(id))
        .map(|def| InventoryItem::from_template(def, count))
        .unwrap_or_else(|| InventoryItem {
            id,
            count,
            ..Default::default()
        })
}

/// Find the first free slot in `1..=HOMDO_SLOTS`, or `None` when full.
pub fn free_slot(bag: &[InventoryItem]) -> Option<u8> {
    (1..=HOMDO_SLOTS).find(|slot| !bag.iter().any(|i| i.slot == *slot && i.id > 0))
}

/// Add `item` to a homdo-style bag, stacking onto existing non-full slots when
/// possible and capping any single stack at 50. Returns the slot(s) actually
/// written — a capped merge
/// can straddle an existing stack **and** a fresh slot, so callers must persist
/// every returned slot (a single-slot return would drop the straddle increment
/// on reload). Returns an empty `Vec` when the bag is full (nothing added).
pub fn add_item(bag: &mut Vec<InventoryItem>, mut item: InventoryItem) -> Vec<u8> {
    if item.count == 0 {
        return Vec::new();
    }
    let mut affected: Vec<u8> = Vec::new();
    // Merge onto existing same-id stacks that are not yet full (count < 50).
    while item.count > 0 {
        let mut found = false;
        for existing in bag.iter_mut() {
            if existing.id == item.id && u16::from(existing.count) < STACK_CAP {
                let room = (STACK_CAP - u16::from(existing.count)).min(u16::from(item.count));
                existing.count += room as u8;
                item.count -= room as u8;
                if !affected.contains(&existing.slot) {
                    affected.push(existing.slot);
                }
                found = true;
                break;
            }
        }
        if !found {
            break;
        }
    }
    if item.count > 0 {
        if let Some(slot) = free_slot(bag) {
            item.slot = slot;
            bag.push(item);
            affected.push(slot);
        }
    }
    affected
}

/// Check that adding the complete item will not silently drop any remainder.
pub fn can_add_item(bag: &[InventoryItem], item: &InventoryItem) -> bool {
    let before: u32 = bag
        .iter()
        .filter(|existing| existing.id == item.id)
        .map(|existing| u32::from(existing.count))
        .sum();
    let mut trial = bag.to_vec();
    add_item(&mut trial, item.clone());
    let after: u32 = trial
        .iter()
        .filter(|existing| existing.id == item.id)
        .map(|existing| u32::from(existing.count))
        .sum();
    after.saturating_sub(before) == u32::from(item.count)
}

/// Remove up to `count` of `item_id` from a homdo-style bag; returns the number
/// actually removed.
pub fn remove_item(bag: &mut Vec<InventoryItem>, item_id: u16, count: u32) -> u32 {
    let mut removed = 0u32;
    for item in bag.iter_mut() {
        if item.id != item_id || item.count == 0 {
            continue;
        }
        let take = (count - removed).min(item.count as u32) as u8;
        item.count -= take;
        removed += take as u32;
        if removed >= count {
            break;
        }
    }
    bag.retain(|i| i.count > 0 || i.id == 0);
    removed
}
