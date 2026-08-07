//! Character stat math: derived max HP/SP and equipment bonuses.
//!
//! Keeps the stat-derivation rules (C# `get_hp_max` / `get_sp_max` plus the
//! equipment-bonus aggregation) in one place, so no caller re-implements them.

use crate::battle::engine::{get_hp_max, get_sp_max};
use crate::server::session::InventoryItem;

/// Equipment bonus stats aggregated from the equipped item list (`trangbi`).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct GearBonuses {
    pub int2: u32,
    pub atk2: u32,
    pub def2: u32,
    pub hpx2: u32,
    pub spx2: u32,
    pub agi2: u32,
}

impl GearBonuses {
    /// Sum the flat stat bonuses of every equipped item (`id > 0`), including
    /// the elemental `_2` fields and the per-item element bonus (C#
    /// `UpdateStatusWhenUseItem`, Client.cs:8478-9196).
    ///
    /// Per equipment slot the C# accumulates both the base (`_X1`) and elemental
    /// (`_X2`) stat, then — when the item's element matches the player's element
    /// (or is the "all-elements" element `5`) — adds `_GiatriThuoctinh` to each
    /// nonzero field, and likewise `_Long == 5` adds `_GiatriLong`. This doubles
    /// the bonus when both the `_X1` and `_X2` components are nonzero, matching
    /// the per-field `if (num > 0)` guards in the C#.
    pub fn from_gear(trangbi: &[InventoryItem], player_element: u8) -> Self {
        let mut b = GearBonuses::default();
        for item in trangbi {
            if item.id == 0 {
                continue;
            }
            b.int2 += item.int1.max(0) as u32 + item.int2.max(0) as u32;
            b.atk2 += item.atk1.max(0) as u32 + item.atk2.max(0) as u32;
            b.def2 += item.def1.max(0) as u32 + item.def2.max(0) as u32;
            b.hpx2 += item.hpx1.max(0) as u32 + item.hpx2.max(0) as u32;
            b.spx2 += item.spx1.max(0) as u32 + item.spx2.max(0) as u32;
            b.agi2 += item.agi1.max(0) as u32 + item.agi2.max(0) as u32;
            let thuoctinh_bonus =
                if item.thuoctinh == player_element || item.thuoctinh == 5 {
                    u32::from(item.giatri_thuoctinh)
                } else {
                    0
                };
            let long_bonus = if item.long_val == player_element || item.long_val == 5 {
                u32::from(item.giatri_long)
            } else {
                0
            };
            let bonus = thuoctinh_bonus + long_bonus;
            if bonus > 0 {
                // C# guards each field with `if (num > 0)` before adding, so a
                // slot with both `_X1` and `_X2` nonzero receives the bonus twice.
                b.int2 += u32::from(item.int1 > 0) * bonus + u32::from(item.int2 > 0) * bonus;
                b.atk2 += u32::from(item.atk1 > 0) * bonus + u32::from(item.atk2 > 0) * bonus;
                b.def2 += u32::from(item.def1 > 0) * bonus + u32::from(item.def2 > 0) * bonus;
                b.hpx2 += u32::from(item.hpx1 > 0) * bonus + u32::from(item.hpx2 > 0) * bonus;
                b.spx2 += u32::from(item.spx1 > 0) * bonus + u32::from(item.spx2 > 0) * bonus;
                b.agi2 += u32::from(item.agi1 > 0) * bonus + u32::from(item.agi2 > 0) * bonus;
            }
        }
        b
    }
}

/// The derived values a character exposes: max HP/SP plus gear bonuses.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CharacterSheet {
    pub hp_max: u16,
    pub sp_max: u16,
    pub gear: GearBonuses,
}

impl CharacterSheet {
    /// Recompute the derived values from base stats and equipped gear.
    pub fn recompute(
        reborn: i64,
        job: i64,
        level: i64,
        hpx: i64,
        spx: i64,
        player_element: u8,
        trangbi: &[InventoryItem],
    ) -> Self {
        let gear = GearBonuses::from_gear(trangbi, player_element);
        let hp_max = get_hp_max(reborn, job, level, hpx) as u16 + gear.hpx2 as u16;
        let sp_max = get_sp_max(reborn, job, level, spx) as u16 + gear.spx2 as u16;
        CharacterSheet {
            hp_max,
            sp_max,
            gear,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_gear_yields_base_max_hp_sp() {
        let sheet = CharacterSheet::recompute(0, 0, 1, 0, 0, 1, &[]);
        assert_eq!(sheet.gear, GearBonuses::default());
        assert!(sheet.hp_max > 0);
        assert!(sheet.sp_max > 0);
    }

    #[test]
    fn gear_bonuses_aggregate_nonzero_items() {
        let trangbi = vec![
            InventoryItem {
                id: 1000,
                atk1: 15,
                hpx1: 20,
                ..Default::default()
            },
            InventoryItem {
                id: 0,
                atk1: 999,
                ..Default::default()
            },
        ];
        let gear = GearBonuses::from_gear(&trangbi, 1);
        assert_eq!(gear.atk2, 15);
        assert_eq!(gear.hpx2, 20);
        assert_eq!(gear.int2, 0);
    }

    #[test]
    fn gear_bonuses_include_elemental_2_stats() {
        // Item carries a base _1 and an elemental _2 field; both are summed.
        let trangbi = vec![InventoryItem {
            id: 1000,
            int1: 5,
            int2: 7,
            thuoctinh: 1,
            giatri_thuoctinh: 10,
            ..Default::default()
        }];
        // Player element 1 matches the item → _GiarTriThuoctinh (10) added to each
        // nonzero int field (`_X1` and `_X2` both nonzero → +20).
        let gear = GearBonuses::from_gear(&trangbi, 1);
        assert_eq!(gear.int2, 5 + 7 + 20); //
        // Player element 2 does not match (nor == 5): no bonus.
        let gear = GearBonuses::from_gear(&trangbi, 2);
        assert_eq!(gear.int2, 5 + 7);
    }

    #[test]
    fn gear_element_bonus_applies_per_nonzero_field_and_element_5() {
        // Both _1 and _2 nonzero → the bonus is added twice, mirroring the C#
        // per-field guards.
        let trangbi = vec![InventoryItem {
            id: 1000,
            int1: 1,
            int2: 2,
            giatri_thuoctinh: 10,
            giatri_long: 4,
            long_val: 1,
            thuoctinh: 5, // all-elements also matches
            ..Default::default()
        }];
        let gear = GearBonuses::from_gear(&trangbi, 1);
        // thuoctinh==5 matches; long_val==1 matches → 3 + 10*2 + 4*2
        assert_eq!(gear.int2, 1 + 2 + 20 + 8);
    }
}
