//! Inventory rules: stacking, slot bounds, add/remove.
//!
//! The slot/stack invariants of a homdo-style bag (25 slots, stack ≤ 255) live
//! here so every mutation path shares them.

use crate::server::session::InventoryItem;

/// The number of usable bag slots (1-based indices `1..=25`).
pub const HOMDO_SLOTS: u8 = 25;

/// Per-slot stack cap for mergeable items (C# hard-coded `50` in `HomdoAddItem`
/// and `HomdoMoveItem`, `Data.cs:3600` / `Data.cs:3291-3305`).
pub const STACK_CAP: u16 = 50;

/// Build an inventory item from its static template (`Data.Item`), carrying the
/// base `_1`/`_2` stats, element and elemental bonus. Falls back to a bare
/// id/count item when the id is not in the static table.
pub fn from_template(data: &crate::data::loader::GameData, id: u16, count: u8) -> InventoryItem {
    data.items
        .get(&i64::from(id))
        .map(|def| InventoryItem::from_template(def, count))
        .unwrap_or_else(|| InventoryItem { id, count, ..Default::default() })
}

/// Find the first free slot in `1..=HOMDO_SLOTS`, or `None` when full.
pub fn free_slot(bag: &[InventoryItem]) -> Option<u8> {
    (1..=HOMDO_SLOTS).find(|slot| !bag.iter().any(|i| i.slot == *slot && i.id > 0))
}

/// Add `item` to a homdo-style bag, stacking onto existing non-full slots when
/// possible and capping any single stack at 50 (C# `HomdoAddItem` merge rule,
/// `Data.cs:3291-3305`). Returns the slot(s) actually written — a capped merge
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

#[cfg(test)]
mod tests {
    use super::*;

    fn item(id: u16, count: u8) -> InventoryItem {
        InventoryItem {
            id,
            count,
            ..Default::default()
        }
    }

    #[test]
    fn add_fills_free_slots_in_order() {
        let mut bag = vec![item(1001, 1), item(1002, 1)];
        bag[0].slot = 1;
        bag[1].slot = 2;
        assert_eq!(add_item(&mut bag, item(1003, 1)), vec![3]);
        assert_eq!(bag[2].slot, 3);
        assert_eq!(bag[2].id, 1003);
    }

    #[test]
    fn add_stacks_onto_nonfull_existing_slot() {
        let mut bag = vec![item(1001, 40)];
        bag[0].slot = 1;
        // Cap-50 merge: fills slot 1 to 50, remainder (40) goes to a new slot.
        // Both slots are returned so the caller persists the straddle.
        assert_eq!(add_item(&mut bag, item(1001, 50)), vec![1, 2]);
        assert_eq!(bag[0].count, 50);
        assert_eq!(bag.len(), 2);
        assert_eq!(bag[1].id, 1001);
        assert_eq!(bag[1].count, 40);
    }

    #[test]
    fn add_rejects_when_full() {
        let mut bag: Vec<InventoryItem> = (1..=25)
            .map(|slot| InventoryItem {
                slot,
                id: 9000 + u16::from(slot),
                count: 1,
                ..Default::default()
            })
            .collect();
        assert!(add_item(&mut bag, item(999, 1)).is_empty());
    }

    #[test]
    fn remove_partial_and_full() {
        let mut bag = vec![item(1001, 3), item(1001, 2)];
        assert_eq!(remove_item(&mut bag, 1001, 4), 4);
        assert_eq!(bag[0].count, 1);
        assert_eq!(remove_item(&mut bag, 1001, 1), 1);
        assert!(bag.is_empty());
    }
}
