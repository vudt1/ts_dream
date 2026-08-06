//! Inventory rules: stacking, slot bounds, add/remove.
//!
//! The slot/stack invariants of a homdo-style bag (25 slots, stack ≤ 255) live
//! here so every mutation path shares them.

use crate::server::session::InventoryItem;

/// The number of usable bag slots (1-based indices `1..=25`).
pub const HOMDO_SLOTS: u8 = 25;

/// Find the first free slot in `1..=HOMDO_SLOTS`, or `None` when full.
pub fn free_slot(bag: &[InventoryItem]) -> Option<u8> {
    (1..=HOMDO_SLOTS).find(|slot| !bag.iter().any(|i| i.slot == *slot && i.id > 0))
}

/// Add `item` to a homdo-style bag, stacking onto an existing slot when
/// possible. Returns the slot used, or `None` when the bag is full.
pub fn add_item(bag: &mut Vec<InventoryItem>, mut item: InventoryItem) -> Option<u8> {
    if item.count > 0 {
        for existing in bag.iter_mut() {
            if existing.id == item.id && existing.count < 255 {
                existing.count = existing.count.saturating_add(item.count);
                return Some(existing.slot);
            }
        }
    }
    let slot = free_slot(bag)?;
    item.slot = slot;
    bag.push(item);
    Some(slot)
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
        assert_eq!(add_item(&mut bag, item(1003, 1)), Some(3));
        assert_eq!(bag[2].slot, 3);
        assert_eq!(bag[2].id, 1003);
    }

    #[test]
    fn add_stacks_onto_existing_slot() {
        let mut bag = vec![item(1001, 100)];
        assert_eq!(add_item(&mut bag, item(1001, 50)), Some(0));
        assert_eq!(bag[0].count, 150);
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
        assert_eq!(add_item(&mut bag, item(999, 1)), None);
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
