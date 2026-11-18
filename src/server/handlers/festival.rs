//! Seasonal Festivals Wire Parity & Handlers (ADR 0011, Ticket 02).
//!
//! Provides:
//! - `S→C 0x24 0x19 [lore_key: u32 LE]`: Festival lore UI trigger (`TMR_FestivalInfoForm`),
//!   matching client `FUN_0051d914` `(double)u32 == record.double_at_0x00`.
//! - `SeasonalNotice` (`S→C 0x17 0x39 <sub2> [sub3]`): Type-safe seasonal notice channel
//!   modeling the 66 valid client branches (`0x01..=0x0D` and `0x15..=0x4A`, excluding gap
//!   `0x0E..=0x14` and non-festival branches).
//! - `build_firework_effect_raw` (`S→C 0x17 0x46 <sub3> [x: u16 LE][y: u16 LE][count: u8]`):
//!   Firework particle effect `TFireLight` (`FUN_0062024c`).

use crate::protocol::encoder;
use crate::server::dispatcher::{OpcodeCtx, OutFrame};
use crate::server::seasonal_festival::{
    has_festival_claim, record_festival_claim, season_period_key, CLAIM_TYPE_LORE_SHOWN,
    FESTIVAL_DOAN_NGO, FESTIVAL_GIANG_SINH, FESTIVAL_HALLOWEEN, FESTIVAL_TET_NGUYEN_DAN,
    FESTIVAL_TET_NGUYEN_TIEU, FESTIVAL_THAT_TICH, FESTIVAL_TRUNG_THU, FESTIVAL_VALENTINE,
};

/// Build festival UI trigger raw frame: `S→C 24 19 [lore_key: u32 LE]`.
pub fn build_festival_ui_trigger_raw(lore_key: u32) -> Vec<u8> {
    let mut payload = Vec::with_capacity(6);
    payload.extend_from_slice(&[0x24, 0x19]);
    payload.extend_from_slice(&lore_key.to_le_bytes());
    let len = payload.len() as u16;
    let mut frame = Vec::with_capacity(4 + payload.len());
    frame.extend_from_slice(&[0xF4, 0x44]);
    frame.extend_from_slice(&len.to_le_bytes());
    frame.extend_from_slice(&payload);
    frame
}

/// Build festival UI trigger hex frame.
pub fn festival_ui_trigger_frame(lore_key: u32) -> String {
    encoder::hex(&build_festival_ui_trigger_raw(lore_key))
}

/// Type-safe representation of Seasonal Festival notices emitted over `S→C 0x17 0x39`.
///
/// Client `FUN_00722cd4` dispatches 66 branches (`0x01..=0x0D` and `0x15..=0x4A`, with
/// `0x0E..=0x14` absent). Non-festival branches (`0x07`, `0x17..=0x1A`, `0x21..=0x24`,
/// `0x26..=0x32`, `0x36..=0x38`, `0x3C`, `0x3E`, `0x40`, `0x41`, `0x48`, `0x4A`) are
/// excluded from `SeasonalNotice` emission.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SeasonalNotice {
    // --- 1-byte `sub2` notices ---
    TetCountdownStart,       // 0x01
    TetCountdownEnd,         // 0x02
    DoanNgoZongzi,           // 0x03
    DoanNgoEnd,              // 0x04
    HalloweenDemonKingStart, // 0x05
    HalloweenDemonKingEnd,   // 0x06
    TetRedEnvelopeStart,     // 0x08
    TetRedEnvelopeEnd,       // 0x09
    TetFireworkClear,        // 0x0A (side effect: FUN_00620b20)
    TetCardStart,            // 0x0B
    TetCardEnd,              // 0x0C
    TetCooking,              // 0x0D
    NoelLuLongStart,         // 0x15
    NoelLuLongEnd,           // 0x16
    HalloweenPhongDo1,       // 0x1B
    HalloweenPhongDo2,       // 0x1C
    HalloweenPhongDoMarquee, // 0x1D
    MidAutumnBbqStart,       // 0x1E
    MidAutumnBbqClear,       // 0x1F (side effect: FUN_0074927c)
    MidAutumnBbqMarquee,     // 0x20
    MidAutumnMoonCake,       // 0x25
    NoelGift1,               // 0x29
    NoelGift2,               // 0x2A
    MidAutumnJadeRabbit,     // 0x3A

    // --- 2-byte `sub2 + sub3` notices ---
    NoelPoke(u8),            // 0x2B, sub3: 1 | 2
    LanternFestival(u8),     // 0x2C, sub3: 1..=3
    DoanNgoLeiFengTower(u8), // 0x33, sub3: 1..=3
    ValentineGarden(u8),     // 0x34, sub3: 1..=3 (2=clear flag, 3=fireworks)
    QixiFestival(u8),        // 0x35, sub3: 1 | 2
    MidAutumnFestival(u8),   // 0x39, sub3: 1..=6
    NoelGiftRain(u8),        // 0x3B, sub3: 1..=5
    LanternDragonSpirit(u8), // 0x3D, sub3: 1..=4
    LanternDoubleExp(u8),    // 0x3F, sub3: 1..=3
    DoanNgoRealgarWine(u8),  // 0x42, sub3: 1..=4
    NoelVillage(u8),         // 0x43, sub3: 1..=4
    NianBeast(u8),           // 0x44, sub3: 1 | 2
    TetSpringBlessing(u8),   // 0x45, sub3: 1 | 2
    TetHuyXuan(u8),          // 0x46, sub3: 1..=4
    TetTaLaoDau(u8),         // 0x47, sub3: 1 | 2
    LanternRiddle(u8),       // 0x49, sub3: 1 | 2
}

impl SeasonalNotice {
    /// Returns `true` if `sub2` is one of the 66 valid client switch cases in `FUN_00722cd4`
    /// (`0x01..=0x0D` and `0x15..=0x4A`, excluding the `0x0E..=0x14` gap).
    pub fn is_valid_client_branch(sub2: u8) -> bool {
        matches!(sub2, 0x01..=0x0D | 0x15..=0x4A)
    }

    /// Parse a `(sub2, Option<sub3>)` pair into a valid festival `SeasonalNotice`.
    /// Rejects the `0x0E..=0x14` gap, non-festival codes (`0x07`, `0x2D..=0x32`,
    /// `0x36..=0x38`, `0x40`, `0x41`, etc.), and invalid/missing `sub3` values.
    pub fn from_festival_code(sub2: u8, sub3: Option<u8>) -> Option<Self> {
        match (sub2, sub3) {
            (0x01, None) => Some(Self::TetCountdownStart),
            (0x02, None) => Some(Self::TetCountdownEnd),
            (0x03, None) => Some(Self::DoanNgoZongzi),
            (0x04, None) => Some(Self::DoanNgoEnd),
            (0x05, None) => Some(Self::HalloweenDemonKingStart),
            (0x06, None) => Some(Self::HalloweenDemonKingEnd),
            (0x08, None) => Some(Self::TetRedEnvelopeStart),
            (0x09, None) => Some(Self::TetRedEnvelopeEnd),
            (0x0A, None) => Some(Self::TetFireworkClear),
            (0x0B, None) => Some(Self::TetCardStart),
            (0x0C, None) => Some(Self::TetCardEnd),
            (0x0D, None) => Some(Self::TetCooking),
            (0x15, None) => Some(Self::NoelLuLongStart),
            (0x16, None) => Some(Self::NoelLuLongEnd),
            (0x1B, None) => Some(Self::HalloweenPhongDo1),
            (0x1C, None) => Some(Self::HalloweenPhongDo2),
            (0x1D, None) => Some(Self::HalloweenPhongDoMarquee),
            (0x1E, None) => Some(Self::MidAutumnBbqStart),
            (0x1F, None) => Some(Self::MidAutumnBbqClear),
            (0x20, None) => Some(Self::MidAutumnBbqMarquee),
            (0x25, None) => Some(Self::MidAutumnMoonCake),
            (0x29, None) => Some(Self::NoelGift1),
            (0x2A, None) => Some(Self::NoelGift2),
            (0x3A, None) => Some(Self::MidAutumnJadeRabbit),
            (0x2B, Some(s @ 1..=2)) => Some(Self::NoelPoke(s)),
            (0x2C, Some(s @ 1..=3)) => Some(Self::LanternFestival(s)),
            (0x33, Some(s @ 1..=3)) => Some(Self::DoanNgoLeiFengTower(s)),
            (0x34, Some(s @ 1..=3)) => Some(Self::ValentineGarden(s)),
            (0x35, Some(s @ 1..=2)) => Some(Self::QixiFestival(s)),
            (0x39, Some(s @ 1..=6)) => Some(Self::MidAutumnFestival(s)),
            (0x3B, Some(s @ 1..=5)) => Some(Self::NoelGiftRain(s)),
            (0x3D, Some(s @ 1..=4)) => Some(Self::LanternDragonSpirit(s)),
            (0x3F, Some(s @ 1..=3)) => Some(Self::LanternDoubleExp(s)),
            (0x42, Some(s @ 1..=4)) => Some(Self::DoanNgoRealgarWine(s)),
            (0x43, Some(s @ 1..=4)) => Some(Self::NoelVillage(s)),
            (0x44, Some(s @ 1..=2)) => Some(Self::NianBeast(s)),
            (0x45, Some(s @ 1..=2)) => Some(Self::TetSpringBlessing(s)),
            (0x46, Some(s @ 1..=4)) => Some(Self::TetHuyXuan(s)),
            (0x47, Some(s @ 1..=2)) => Some(Self::TetTaLaoDau(s)),
            (0x49, Some(s @ 1..=2)) => Some(Self::LanternRiddle(s)),
            _ => None,
        }
    }

    pub fn sub2(&self) -> u8 {
        match self {
            Self::TetCountdownStart => 0x01,
            Self::TetCountdownEnd => 0x02,
            Self::DoanNgoZongzi => 0x03,
            Self::DoanNgoEnd => 0x04,
            Self::HalloweenDemonKingStart => 0x05,
            Self::HalloweenDemonKingEnd => 0x06,
            Self::TetRedEnvelopeStart => 0x08,
            Self::TetRedEnvelopeEnd => 0x09,
            Self::TetFireworkClear => 0x0A,
            Self::TetCardStart => 0x0B,
            Self::TetCardEnd => 0x0C,
            Self::TetCooking => 0x0D,
            Self::NoelLuLongStart => 0x15,
            Self::NoelLuLongEnd => 0x16,
            Self::HalloweenPhongDo1 => 0x1B,
            Self::HalloweenPhongDo2 => 0x1C,
            Self::HalloweenPhongDoMarquee => 0x1D,
            Self::MidAutumnBbqStart => 0x1E,
            Self::MidAutumnBbqClear => 0x1F,
            Self::MidAutumnBbqMarquee => 0x20,
            Self::MidAutumnMoonCake => 0x25,
            Self::NoelGift1 => 0x29,
            Self::NoelGift2 => 0x2A,
            Self::MidAutumnJadeRabbit => 0x3A,
            Self::NoelPoke(_) => 0x2B,
            Self::LanternFestival(_) => 0x2C,
            Self::DoanNgoLeiFengTower(_) => 0x33,
            Self::ValentineGarden(_) => 0x34,
            Self::QixiFestival(_) => 0x35,
            Self::MidAutumnFestival(_) => 0x39,
            Self::NoelGiftRain(_) => 0x3B,
            Self::LanternDragonSpirit(_) => 0x3D,
            Self::LanternDoubleExp(_) => 0x3F,
            Self::DoanNgoRealgarWine(_) => 0x42,
            Self::NoelVillage(_) => 0x43,
            Self::NianBeast(_) => 0x44,
            Self::TetSpringBlessing(_) => 0x45,
            Self::TetHuyXuan(_) => 0x46,
            Self::TetTaLaoDau(_) => 0x47,
            Self::LanternRiddle(_) => 0x49,
        }
    }

    pub fn sub3(&self) -> Option<u8> {
        match *self {
            Self::NoelPoke(s)
            | Self::LanternFestival(s)
            | Self::DoanNgoLeiFengTower(s)
            | Self::ValentineGarden(s)
            | Self::QixiFestival(s)
            | Self::MidAutumnFestival(s)
            | Self::NoelGiftRain(s)
            | Self::LanternDragonSpirit(s)
            | Self::LanternDoubleExp(s)
            | Self::DoanNgoRealgarWine(s)
            | Self::NoelVillage(s)
            | Self::NianBeast(s)
            | Self::TetSpringBlessing(s)
            | Self::TetHuyXuan(s)
            | Self::TetTaLaoDau(s)
            | Self::LanternRiddle(s) => Some(s),
            _ => None,
        }
    }

    /// Encode into raw wire frame (`F4 44 + len + 17 39 + sub2 [+ sub3]`).
    pub fn to_raw_frame(&self) -> Vec<u8> {
        let sub2 = self.sub2();
        match self.sub3() {
            Some(sub3) => vec![0xF4, 0x44, 0x04, 0x00, 0x17, 0x39, sub2, sub3],
            None => vec![0xF4, 0x44, 0x03, 0x00, 0x17, 0x39, sub2],
        }
    }

    /// Encode into an `OutFrame` hex frame.
    pub fn to_frame(&self) -> OutFrame {
        OutFrame::new(encoder::hex(&self.to_raw_frame()))
    }
}

/// Opening `S→C 0x17 0x39` notice broadcast when a seasonal festival transitions `Idle -> Active`.
pub fn start_notice_for_festival(festival_id: &str) -> Option<SeasonalNotice> {
    match festival_id {
        FESTIVAL_TET_NGUYEN_DAN => Some(SeasonalNotice::TetCountdownStart),
        FESTIVAL_TET_NGUYEN_TIEU => Some(SeasonalNotice::LanternFestival(1)),
        FESTIVAL_VALENTINE => Some(SeasonalNotice::ValentineGarden(1)),
        FESTIVAL_DOAN_NGO => Some(SeasonalNotice::DoanNgoZongzi),
        FESTIVAL_THAT_TICH => Some(SeasonalNotice::QixiFestival(1)),
        FESTIVAL_TRUNG_THU => Some(SeasonalNotice::MidAutumnFestival(1)),
        FESTIVAL_HALLOWEEN => Some(SeasonalNotice::HalloweenDemonKingStart),
        FESTIVAL_GIANG_SINH => Some(SeasonalNotice::NoelLuLongStart),
        _ => None,
    }
}

/// Closing `S→C 0x17 0x39` notice broadcast when a seasonal festival transitions `Active -> Idle`.
pub fn stop_notice_for_festival(festival_id: &str) -> Option<SeasonalNotice> {
    match festival_id {
        FESTIVAL_TET_NGUYEN_DAN => Some(SeasonalNotice::TetCountdownEnd),
        FESTIVAL_TET_NGUYEN_TIEU => Some(SeasonalNotice::LanternFestival(2)),
        FESTIVAL_VALENTINE => Some(SeasonalNotice::ValentineGarden(2)),
        FESTIVAL_DOAN_NGO => Some(SeasonalNotice::DoanNgoEnd),
        FESTIVAL_THAT_TICH => Some(SeasonalNotice::QixiFestival(2)),
        FESTIVAL_TRUNG_THU => Some(SeasonalNotice::MidAutumnFestival(2)),
        FESTIVAL_HALLOWEEN => Some(SeasonalNotice::HalloweenDemonKingEnd),
        FESTIVAL_GIANG_SINH => Some(SeasonalNotice::NoelLuLongEnd),
        _ => None,
    }
}

/// Build firework particle effect raw frame:
/// `S→C 0x17 0x46 <sub3> [x: u16 LE][y: u16 LE][count: u8]` (payload length 8).
pub fn build_firework_effect_raw(sub: u8, x: u16, y: u16, count: u8) -> Vec<u8> {
    let [x_lo, x_hi] = x.to_le_bytes();
    let [y_lo, y_hi] = y.to_le_bytes();
    vec![
        0xF4, 0x44, 0x08, 0x00, 0x17, 0x46, sub, x_lo, x_hi, y_lo, y_hi, count,
    ]
}

/// Build firework particle effect hex frame.
pub fn firework_effect_frame(sub: u8, x: u16, y: u16, count: u8) -> String {
    encoder::hex(&build_firework_effect_raw(sub, x, y, count))
}

/// Handle `C→S 0x24 0x19` festival lore request.
///
/// Only responds with `S→C 0x24 0x19 [u32 LE]` if there is an `Active` seasonal festival
/// with a valid, non-colliding `unique_lore_key` in `ctx.data.festivals`.
pub async fn handle_festival(ctx: &mut OpcodeCtx<'_>) {
    if ctx.opcode == 0x24 && ctx.sub == 0x19 {
        let Some(hub) = ctx.env.hub else {
            return;
        };
        let lore_key = hub
            .festivals
            .list_festivals()
            .into_iter()
            .filter(|f| f.is_active)
            .find_map(|f| {
                f.lore_record_id
                    .and_then(|rec_id| ctx.data.festivals.get(&rec_id))
                    .and_then(|def| def.unique_lore_key)
            });
        if let Some(key) = lore_key {
            ctx.out.send(festival_ui_trigger_frame(key));
        }
    }
}

/// Emit seasonal festival login sync frames on login completion (`OP_LOGIN_COMPLETE`):
/// 1. `S→C 0x17 0x39` for all currently `Active` seasonal festivals that have a notice.
/// 2. At most `1` `S→C 0x24 0x19` lore popup for the first `Active` festival with a valid
///    `unique_lore_key` not yet claimed (`lore_shown`) in `character_festival_claims` for
///    `season-<season_seq>`, persisting the claim to SQLite.
pub async fn emit_login_festival_sync(ctx: &mut OpcodeCtx<'_>, player_id: u32) {
    let Some(hub) = ctx.env.hub else {
        return;
    };

    let active_festivals: Vec<_> = hub
        .festivals
        .list_festivals()
        .into_iter()
        .filter(|f| f.is_active)
        .collect();

    for fest in &active_festivals {
        if let Some(notice) = start_notice_for_festival(&fest.id) {
            ctx.out.send(notice.to_frame().frame);
        }
    }

    let Some(pool) = ctx.env.pool else {
        return;
    };

    for fest in &active_festivals {
        let Some(lore_key) = fest
            .lore_record_id
            .and_then(|rec_id| ctx.data.festivals.get(&rec_id))
            .and_then(|def| def.unique_lore_key)
        else {
            continue;
        };

        let period_key = season_period_key(fest.season_seq);
        match has_festival_claim(pool, player_id, &fest.id, &period_key, CLAIM_TYPE_LORE_SHOWN)
            .await
        {
            Ok(false) => {
                ctx.out.send(festival_ui_trigger_frame(lore_key));
                if let Err(e) = record_festival_claim(
                    pool,
                    player_id,
                    &fest.id,
                    &period_key,
                    CLAIM_TYPE_LORE_SHOWN,
                )
                .await
                {
                    tracing::warn!(
                        player_id,
                        festival_id = %fest.id,
                        period_key = %period_key,
                        error = %e,
                        "Failed to persist festival lore_shown claim"
                    );
                }
                break;
            }
            Ok(true) => continue,
            Err(e) => {
                tracing::warn!(
                    player_id,
                    festival_id = %fest.id,
                    period_key = %period_key,
                    error = %e,
                    "Failed to query festival lore_shown claim"
                );
            }
        }
    }
}
