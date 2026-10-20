//! Character entity kind constants — matches `define.lua:EHuman`.
//!
//! Relocated from the removed `battle_role.rs`: the `type` byte of the `0B FA`
//! (`FUN_006438ac`) / `0B 05` (`FUN_00644294`) battle records takes exactly
//! these codes, so the table is the only part of that module verified against
//! the PC client (`01_battle_start_scene.md §2.4`).
//!
//! Note: this is a constant table only. The former 42-byte
//! `BattleRoleData`/`BattleRoleSerializer` layout came from TS Mobile Kotlin
//! and does **not** exist in the PC client — do not resurrect it.

pub const NONE: u8 = 0;
pub const PLAYER: u8 = 1;
pub const PLAYERS: u8 = 2;
pub const MAP_NPC: u8 = 3;
pub const FOLLOW_NPC: u8 = 4;
pub const SCENE_ELM: u8 = 5;
pub const GUARD_NPC: u8 = 6;
pub const MINE_NPC: u8 = 7;
pub const OWNER_NPC: u8 = 8;
pub const DIVIDE: u8 = 9;
pub const SOLDIER: u8 = 11;
pub const MACHINE: u8 = 12;
pub const CTRL_MACH: u8 = 13;
pub const AUTO_FOLLOW: u8 = 14;
pub const CTRL_SOLD: u8 = 15;
pub const FOREIGN_NPC: u8 = 16;
pub const CART_NPC: u8 = 17;
pub const CHAOS_GOD: u8 = 18;
pub const PET_NPC: u8 = 19;
pub const QUESTION: u8 = 20;
pub const WORLD_BOSS: u8 = 21;
pub const AUTOMANUAL_PLAYER: u8 = 28;
pub const AUTOMANUAL_NPC: u8 = 29;
pub const HOUSE_WARRIOR: u8 = 30;
pub const BATTLE_ROYALE_NPC: u8 = 31;
pub const DUPLICATE: u8 = 255;
pub const NPC: u8 = MAP_NPC;
