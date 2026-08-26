//! Map item drops.
//!
//! Thrown items (`op 0x17` sub 3 → `HomdoDropItem`) land on the shared map
//! registry and are recovered by any player within pickup range (`op 0x17`
//! sub 2). The registry is server-global and keyed by `(map_id, slot)`;
//! golden replay and integration tests drive it through the exported helpers
//! directly, each suite owning a disjoint `map_id` band.

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

/// Allocate a free drop slot (1..=255) for a map, mirroring the legacy
/// throw-item flow which scans ascending and takes the first empty slot.
/// Returns `None` when the map has no free slot (the drop is refused and the
/// item stays in the player's bag).
pub fn allocate(map_id: u16, item: InventoryItem, x: u16, y: u16) -> Option<u8> {
    let mut reg = registry().lock().unwrap();
    for slot in 1..=255u8 {
        use std::collections::hash_map::Entry;
        let entry = reg.entry((map_id, slot));
        let Entry::Vacant(v) = entry else {
            continue;
        };
        v.insert(DropItem {
            map_x: x,
            map_y: y,
            item,
        });
        return Some(slot);
    }
    None
}

/// Look up a drop slot on a map.
pub fn get(map_id: u16, slot: u8) -> Option<DropItem> {
    registry().lock().unwrap().get(&(map_id, slot)).cloned()
}

/// Remove and return a drop slot on a map.
pub fn take(map_id: u16, slot: u8) -> Option<DropItem> {
    registry().lock().unwrap().remove(&(map_id, slot))
}

/// Clear every drop on one map.
///
/// Idempotent per-map reset. Tests that seed drops must clean up with this
/// scoped reset (or targeted [`take`] calls) instead of [`clear_all`], which
/// wipes every map's entries and races with parallel tests that own their own
/// map ids.
pub fn clear_map(map_id: u16) {
    registry()
        .lock()
        .unwrap()
        .retain(|(m, _), _| *m != map_id);
}

/// Clear every drop on every map (restart aid; wipes all maps at once).
pub fn clear_all() {
    registry().lock().unwrap().clear();
}
