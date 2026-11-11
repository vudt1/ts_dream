//! Server-to-client full character stats & learned skills packet codec (`05 03`).
//!
//! Encodes the 113-byte base stats header (`F4 44 <len> 05 03 ...`) followed by a
//! variable-length tail of `(skill_id: u16 LE, level: u8)` 3-byte records.

use crate::protocol::{encoder, frame, OPCODE_STATS};
use crate::server::session::Session;

/// Typed builder for the `05 03` (`OPCODE_STATS`) full character sheet and skill list packet.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct StatsPacketBuilder {
    pub thuoctinh: u8,
    pub hp: u16,
    pub sp: u16,
    pub int1: u16,
    pub atk: u16,
    pub def: u16,
    pub agi: u16,
    pub hpx: u16,
    pub spx: u16,
    pub lv: u8,
    pub texp: u32,
    pub skill_point: u16,
    pub point: u16,
    pub tiengtam: u32,
    pub hp_max: u16,
    pub sp_max: u16,
    pub atk2: u32,
    pub def2: u32,
    pub int2: u32,
    pub agi2: u32,
    pub hpx2: u32,
    pub spx2: u32,
    pub reborn: u8,
    pub job: u8,
    pub skills: Vec<(u16, u8)>,
}

impl StatsPacketBuilder {
    /// Creates a zero-initialized `StatsPacketBuilder`.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Populates a `StatsPacketBuilder` from an active player [`Session`].
    #[must_use]
    pub fn from_session(s: &Session) -> Self {
        Self {
            thuoctinh: s.thuoctinh,
            hp: s.hp,
            sp: s.sp,
            int1: s.int1,
            atk: s.atk,
            def: s.def,
            agi: s.agi,
            hpx: s.hpx,
            spx: s.spx,
            lv: s.level,
            texp: s.texp,
            skill_point: s.skill_point,
            point: s.point,
            tiengtam: u32::from(s.tiengtam),
            hp_max: s.hp_max,
            sp_max: s.sp_max,
            atk2: s.atk2,
            def2: s.def2,
            int2: s.int2,
            agi2: s.agi2,
            hpx2: s.hpx2,
            spx2: s.spx2,
            reborn: s.reborn,
            job: s.job,
            skills: s.skills.clone(),
        }
    }

    /// Builds the raw unframed payload starting with the `[0x05, 0x03]` opcode header
    /// (113-byte fixed header + `3 * skills.len()` bytes).
    #[must_use]
    pub fn build_payload(&self) -> Vec<u8> {
        let mut buf = Vec::with_capacity(113 + self.skills.len() * 3);

        // [0..2]: Opcode 05 03
        buf.extend_from_slice(&[0x05, 0x03]);
        // [2]: Element / ThuocTinh (1B)
        buf.push(self.thuoctinh);
        // [3..5]: Current HP (u16 LE)
        buf.extend_from_slice(&self.hp.to_le_bytes());
        // [5..7]: Current SP (u16 LE)
        buf.extend_from_slice(&self.sp.to_le_bytes());
        // [7..9]: Base INT (u16 LE)
        buf.extend_from_slice(&self.int1.to_le_bytes());
        // [9..11]: Base ATK (u16 LE)
        buf.extend_from_slice(&self.atk.to_le_bytes());
        // [11..13]: Base DEF (u16 LE)
        buf.extend_from_slice(&self.def.to_le_bytes());
        // [13..15]: Base AGI (u16 LE)
        buf.extend_from_slice(&self.agi.to_le_bytes());
        // [15..17]: Base HPX (u16 LE)
        buf.extend_from_slice(&self.hpx.to_le_bytes());
        // [17..19]: Base SPX (u16 LE)
        buf.extend_from_slice(&self.spx.to_le_bytes());
        // [19]: Level (u8)
        buf.push(self.lv);
        // [20..24]: Total EXP (u32 LE)
        buf.extend_from_slice(&self.texp.to_le_bytes());
        // [24..26]: Free Skill Points (u16 LE)
        buf.extend_from_slice(&self.skill_point.to_le_bytes());
        // [26..28]: Free Stat Points (u16 LE)
        buf.extend_from_slice(&self.point.to_le_bytes());
        // [28..32]: Fame / TiengTam (u32 LE)
        buf.extend_from_slice(&self.tiengtam.to_le_bytes());
        // [32..34]: Max HP (u16 LE)
        buf.extend_from_slice(&self.hp_max.to_le_bytes());
        // [34..36]: Max SP (u16 LE)
        buf.extend_from_slice(&self.sp_max.to_le_bytes());
        // [36..40]: Bonus ATK2 (u32 LE)
        buf.extend_from_slice(&self.atk2.to_le_bytes());
        // [40..44]: Bonus DEF2 (u32 LE)
        buf.extend_from_slice(&self.def2.to_le_bytes());
        // [44..48]: Bonus INT2 (u32 LE)
        buf.extend_from_slice(&self.int2.to_le_bytes());
        // [48..52]: Bonus AGI2 (u32 LE)
        buf.extend_from_slice(&self.agi2.to_le_bytes());
        // [52..56]: Bonus HPX2 (u32 LE)
        buf.extend_from_slice(&self.hpx2.to_le_bytes());
        // [56..60]: Bonus SPX2 (u32 LE)
        buf.extend_from_slice(&self.spx2.to_le_bytes());
        // [60..70]: 5 x 500u16 LE constants (F4 01 x 5 = 10B)
        for _ in 0..5 {
            buf.extend_from_slice(&500u16.to_le_bytes());
        }
        // [70..96]: 26 zero bytes
        buf.extend_from_slice(&[0u8; 26]);
        // [96]: Reborn flag (u8)
        buf.push(self.reborn);
        // [97]: Job class ID (u8)
        buf.push(self.job);
        // [98..113]: 15 zero bytes
        buf.extend_from_slice(&[0u8; 15]);

        debug_assert_eq!(buf.len(), 113);

        // [113..]: Learned skills array: (skill_id: u16 LE, level: u8)
        for &(skill_id, level) in &self.skills {
            buf.extend_from_slice(&skill_id.to_le_bytes());
            buf.push(level);
        }

        buf
    }

    /// Builds the full `F4 44 <len> 05 03 ...` uppercase hex frame string.
    #[must_use]
    pub fn build_frame_hex(&self) -> String {
        let payload = self.build_payload();
        frame(OPCODE_STATS, &encoder::hex(&payload[2..]))
    }
}
