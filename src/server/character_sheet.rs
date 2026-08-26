//! Character stat math: derived max HP/SP and equipment bonuses.
//!
//! Keeps the stat-derivation rules (base max HP/SP curves plus the
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
    /// the elemental `_2` fields and the per-item element bonus.
    ///
    /// Per equipment slot, accumulate both the base (`_X1`) and elemental
    /// (`_X2`) stat; then — when the item's element matches the player's
    /// element (or is the "all-elements" element `5`) — add `_GiatriThuoctinh`
    /// to each nonzero field, and likewise `_Long == 5` adds `_GiatriLong`.
    /// This doubles the bonus when both the `_X1` and `_X2` components are
    /// nonzero (each nonzero field receives it once).
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
                // Each nonzero field adds the bonus, so a slot with both `_X1`
                // and `_X2` nonzero receives it twice.
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
