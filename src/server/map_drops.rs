//! Map item drops (C# `Data.ItemDropOnMap` / `PickupItemOnMap`).
//!
//! Thrown items (`op 0x17` sub 3 → `HomdoDropItem`) land on the shared map
//! registry and are recovered by any player within pickup range (`op 0x17`
//! sub 2). The registry is server-global exactly like the C# static
//! `ItemDropOnMap` dictionary (keyed by `(map_id, slot)`); golden replay and
//! unit tests drive it through the exported helpers directly.

use crate::server::session::InventoryItem;
use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

/// A drop lying on a map tile.
#[derive(Debug, Clone, Default)]
pub struct DropItem {
    pub map_x: u16,
    pub map_y: u16,
    /// The full item payload (slot is ignored for drops).
    pub item: InventoryItem,
}

fn registry() -> &'static Mutex<HashMap<(u16, u8), DropItem>> {
    static REGISTRY: OnceLock<Mutex<HashMap<(u16, u8), DropItem>>> = OnceLock::new();
    REGISTRY.get_or_init(|| Mutex::new(HashMap::new()))
}

/// Place a drop under the `(map_id, slot)` key. Replaces any prior entry.
pub fn drop(map_id: u16, slot: u8, item: InventoryItem, x: u16, y: u16) {
    registry().lock().unwrap().insert(
        (map_id, slot),
        DropItem {
            map_x: x,
            map_y: y,
            item,
        },
    );
}

/// Look up a drop slot on a map.
pub fn get(map_id: u16, slot: u8) -> Option<DropItem> {
    registry().lock().unwrap().get(&(map_id, slot)).cloned()
}

/// Remove and return a drop slot on a map.
pub fn take(map_id: u16, slot: u8) -> Option<DropItem> {
    registry().lock().unwrap().remove(&(map_id, slot))
}

/// Clear every drop (test/restart aid).
pub fn clear_all() {
    registry().lock().unwrap().clear();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn drop_take_and_get() {
        clear_all();
        let item = InventoryItem {
            id: 1001,
            count: 2,
            ..Default::default()
        };
        drop(12001, 3, item.clone(), 400, 500);
        let got = get(12001, 3).unwrap();
        assert_eq!(got.item.id, 1001);
        assert_eq!(got.item.count, 2);
        assert_eq!(got.map_x, 400);
        let taken = take(12001, 3).unwrap();
        assert_eq!(taken.item.id, 1001);
        assert!(get(12001, 3).is_none());
        clear_all();
    }
}
