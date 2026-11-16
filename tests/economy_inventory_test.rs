//! Consolidated economy, inventory, storage & pet management test suite.

mod common;

#[path = "economy_inventory/economy_wire.rs"]
mod economy_wire;
#[path = "economy_inventory/inventory_gear_bonus.rs"]
mod inventory_gear_bonus;
#[path = "economy_inventory/inventory_roundtrip.rs"]
mod inventory_roundtrip;
#[path = "economy_inventory/inventory_storage_type.rs"]
mod inventory_storage_type;
#[path = "economy_inventory/inventory_template_hydrate.rs"]
mod inventory_template_hydrate;
#[path = "economy_inventory/pet_equip_unequip.rs"]
mod pet_equip_unequip;
#[path = "economy_inventory/pet_hotkey_flow.rs"]
mod pet_hotkey_flow;
#[path = "economy_inventory/pet_recall_13_02.rs"]
mod pet_recall_13_02;
#[path = "economy_inventory/pet_stat_frame.rs"]
mod pet_stat_frame;
