//! Battle engine core (Chapter 6) — pure math + domain that is testable
//! without a live DB or wire capture. The async per-battle task (turn loop,
//! tick, targeting) builds on these primitives.

use crate::data::texps::texp_get_lv_up;
use crate::protocol::encoder;

/// EXP level-ups from a battle reward, via the Texps array (§6.6).
pub fn exp_lv_up(
    texps: &[crate::data::tables::TexpRow],
    lv: i64,
    reborn: usize,
    texp: i64,
) -> i64 {
    texp_get_lv_up(texps, lv, reborn, texp)
}

/// 20-cell grid keyed by `row-col` (hex). Row 0..3, col 0..4.
pub fn war_key(row: u8, col: u8) -> String {
    format!("{row:x}{col:x}")
}

/// A single grid cell (Chapter 6 §6.0).
#[derive(Debug, Clone, Default)]
pub struct WarInfo {
    pub typ: u8,
    pub id: i64,
    pub id_npc_on_map: i64,
    pub id_char: i64,
    pub row: u8,
    pub col: u8,
    pub hp_max: i64,
    pub sp_max: i64,
    pub hp: i64,
    pub sp: i64,
    pub lv: i64,
    pub thuoctinh: i64,
    pub leader_id: i64,
    pub id_skill: i64,
    pub row_attack: u8,
    pub col_attack: u8,
    pub int1: i64,
    pub atk: i64,
    pub def: i64,
    pub agi: i64,
    pub reborn: i64,
    pub team: i64,
    pub attacked: bool,
    pub random: i64,
    pub exp: i64,
}

impl WarInfo {
    /// The `_Packet` 23-byte snapshot (`ChangedWar`, §6.1):
    /// Type:X2 | le32(Id) | le16(IdNpcOnMap) | le32(IdChar) | row,col |
    /// le16(HpMax) | le16(SpMax) | le16(Hp) | le16(Sp) | lv, TT.
    pub fn packet_hex(&self) -> String {
        let mut s = String::with_capacity(46);
        s.push_str(&format!("{:02X}", self.typ));
        s.push_str(&encoder::le32(self.id as u32));
        s.push_str(&encoder::le16(self.id_npc_on_map as u16));
        s.push_str(&encoder::le32(self.id_char as u32));
        s.push_str(&format!("{:02X}{:02X}", self.row, self.col));
        s.push_str(&encoder::le16(clamp16(self.hp_max)));
        s.push_str(&encoder::le16(clamp16(self.sp_max)));
        s.push_str(&encoder::le16(clamp16(self.hp)));
        s.push_str(&encoder::le16(clamp16(self.sp)));
        s.push_str(&format!(
            "{:02X}{:02X}",
            clamp_u8(self.lv),
            clamp_u8(self.thuoctinh)
        ));
        s
    }
}

fn clamp16(v: i64) -> u16 {
    v.clamp(0, 0xFFFF) as u16
}

fn clamp_u8(v: i64) -> u8 {
    v.clamp(0, 0xFF) as u8
}

/// `getHpMax(rb, job, lvl, hpx)` (Data.cs:5537) — resolved exactly (§6.6).
pub fn get_hp_max(rb: i64, job: i64, lvl: i64, hpx: i64) -> i64 {
    let lv = lvl as f64;
    let p = lv.powf(0.35);
    let val = match rb {
        0 => (p + 1.0) * (hpx as f64) * 2.0 + 80.0 + lv,
        1 => (p + 2.0) * (hpx as f64) * 2.0 + 180.0 + lv,
        _ => match job {
            1 => (p * 2.0 + 25.0) * (hpx as f64) + 280.0 + lv,
            2 => (p * 3.0 + 30.0) * (hpx as f64) + 380.0 + lv,
            3 => (p + 11.5) * (hpx as f64) * 2.0 + 180.0 + lv,
            _ => (p + 10.5) * (hpx as f64) * 2.0 + 180.0 + lv,
        },
    };
    val.floor() as i64
}

/// `getSpMax(rb, job, lvl, spx)` (Data.cs:5553) (§6.6).
pub fn get_sp_max(rb: i64, job: i64, lvl: i64, spx: i64) -> i64 {
    let lv = lvl as f64;
    let p = lv.powf(0.25);
    let val = match rb {
        0 => p * (spx as f64) * 2.0 + 60.0 + lv,
        1 => p * (spx as f64) * 2.0 + 110.0 + lv,
        _ => match job {
            1 | 2 => p * (spx as f64) * 2.0 + 160.0 + lv,
            3 => p * (spx as f64) * 3.0 + 310.0 + lv,
            _ => p * (spx as f64) * 3.5 + 410.0 + lv,
        },
    };
    val.floor() as i64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn packet_hex_23_bytes() {
        let w = WarInfo {
            typ: 2,
            id: 300003,
            hp_max: 311,
            sp_max: 411,
            hp: 311,
            sp: 411,
            lv: 101,
            thuoctinh: 1,
            ..Default::default()
        };
        let hex = w.packet_hex();
        assert_eq!(hex.len(), 46); // 23 bytes
    }

    #[test]
    fn hpmax_floor() {
        // rb 0, job, lvl 1, hpx 6: floor((1+1)*12+80+1)=floor(105)=105
        assert_eq!(get_hp_max(0, 0, 1, 6), 105);
    }

    #[test]
    fn spmax_floor() {
        // rb 0, lvl 1, spx 6: floor((1)*12+60+1)=floor(73)=73
        assert_eq!(get_sp_max(0, 0, 1, 6), 73);
    }
}