//! Consolidated static data loaders, encoding, formulas & system alert tests.

mod common;

#[path = "data_loaders/encoding.rs"]
mod encoding;
#[path = "data_loaders/ground.rs"]
mod ground;
#[path = "data_loaders/item.rs"]
mod item;
#[path = "data_loaders/max_hp_sp_formula.rs"]
mod max_hp_sp_formula;
#[path = "data_loaders/rank.rs"]
mod rank;
#[path = "data_loaders/skill_dat_rebuild.rs"]
mod skill_dat_rebuild;
#[path = "data_loaders/system_alert.rs"]
mod system_alert;
#[path = "data_loaders/texps_formula.rs"]
mod texps_formula;
#[path = "data_loaders/warps.rs"]
mod warps;
