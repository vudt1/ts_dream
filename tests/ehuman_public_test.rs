//! Ticket 13 — `ehuman` must survive the removal of `BattleRoleData` /
//! `BattleRoleSerializer` and stay reachable through the public re-export paths.
//!
//! `ehuman` is the `EHuman` entity-kind table used as the `type` byte of the
//! `0B FA` / `0B 05` battle records (`01_battle_start_scene.md §2.4`). These
//! imports deliberately go through the crate public API, not a private module.

use ts_dream::protocol::codecs::ehuman as codecs_ehuman;
use ts_dream::protocol::ehuman;

#[test]
fn ehuman_reachable_through_protocol_reexport() {
    assert_eq!(ehuman::NONE, 0);
    assert_eq!(ehuman::PLAYER, 1);
    assert_eq!(ehuman::PLAYERS, 2);
    assert_eq!(ehuman::MAP_NPC, 3);
    assert_eq!(ehuman::FOLLOW_NPC, 4);
    assert_eq!(ehuman::AUTO_FOLLOW, 14);
    assert_eq!(ehuman::PET_NPC, 19);
    assert_eq!(ehuman::DUPLICATE, 255);
    // `NPC` is an alias of `MAP_NPC`.
    assert_eq!(ehuman::NPC, ehuman::MAP_NPC);
}

#[test]
fn ehuman_reachable_through_codecs_module() {
    assert_eq!(codecs_ehuman::PLAYER, 1);
    assert_eq!(codecs_ehuman::SOLDIER, 11);
    assert_eq!(codecs_ehuman::AUTOMANUAL_PLAYER, 28);
}
