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
    /// Sum the flat stat bonuses of every equipped item (`id > 0`).
    pub fn from_gear(trangbi: &[InventoryItem]) -> Self {
        let mut b = GearBonuses::default();
        for item in trangbi {
            if item.id > 0 {
                b.int2 += item.int1.max(0) as u32;
                b.atk2 += item.atk1.max(0) as u32;
                b.def2 += item.def1.max(0) as u32;
                b.hpx2 += item.hpx1.max(0) as u32;
                b.spx2 += item.spx1.max(0) as u32;
                b.agi2 += item.agi1.max(0) as u32;
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
        trangbi: &[InventoryItem],
    ) -> Self {
        let gear = GearBonuses::from_gear(trangbi);
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
        let sheet = CharacterSheet::recompute(0, 0, 1, 0, 0, &[]);
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
        let gear = GearBonuses::from_gear(&trangbi);
        assert_eq!(gear.atk2, 15);
        assert_eq!(gear.hpx2, 20);
        assert_eq!(gear.int2, 0);
    }
}
