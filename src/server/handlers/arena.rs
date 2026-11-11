//! Competitive Arena Tournaments, Scoreboards, and Weekly Reset Subsystem (Ticket 06).
//!
//! Provides protocol builders, tournament notifications, live scoreboard encoding,
//! and Monday 00:00 timed item revocation logic matching client `aLogin.exe` wire parity.

use crate::protocol::encoder;
use crate::server::dispatcher::OpcodeCtx;
use crate::server::session::Session;
use chrono::{DateTime, Datelike, Timelike, Utc};

// Timed championship item constants
pub const ITEM_NHAN_LOAN_DAU: u16 = 46077;
pub const ITEM_NHAN_DOAN_P: u16 = 46078;
pub const ITEM_VAN_CHUONG_VO_GIOI: u16 = 46079;

/// Competitive tournament modes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TournamentMode {
    SoloPk = 1,
    TeamPk = 2,
    Melee = 3,
    Celestial = 4,
    Survival = 5,
}

/// Global tournament announcement notice codes for `S→C 25 03 [noticeCode: u8]`.
/// 10 codes across 5 tournament modes (preparation, commencement, conclusion).
pub const NOTICE_SOLO_PK_PREPARE: u8 = 0x01;
pub const NOTICE_SOLO_PK_START: u8 = 0x02;
pub const NOTICE_TEAM_PK_PREPARE: u8 = 0x03;
pub const NOTICE_TEAM_PK_START: u8 = 0x04;
pub const NOTICE_MELEE_PREPARE: u8 = 0x05;
pub const NOTICE_MELEE_START: u8 = 0x06;
pub const NOTICE_CELESTIAL_PREPARE: u8 = 0x07;
pub const NOTICE_CELESTIAL_START: u8 = 0x08;
pub const NOTICE_SURVIVAL_PREPARE: u8 = 0x09;
pub const NOTICE_SURVIVAL_START: u8 = 0x0A;

/// Match status states for `S→C 14 28 [state: u8]`.
pub const MATCH_STATE_START: u8 = 1;
pub const MATCH_STATE_END: u8 = 2;

/// Timed item revocation notice codes for `S→C 17 6F [code: u8]`.
pub const REVOKE_CHAMPION_TRANSFER: u8 = 0x01; // "Người chủ mới là ..."
pub const REVOKE_NHAN_LOAN_DAU: u8 = 0x02;     // "Do thời hạn đã được 1 tuần, hệ thống thu hồi nhẫn loạn đấu mà bạn có"
pub const REVOKE_NHAN_DOAN_P: u8 = 0x03;       // "Vì thời hạn duy trì đã đến 1 tuần, hệ thống sẽ thu hồi chiếc nhẫn đoàn P mà bạn đang có"
pub const REVOKE_VAN_CHUONG_VO_GIOI: u8 = 0x04;// "Vì thời hạn duy trì đã đến 1 tuần, hệ thống sẽ thu hồi văn chương vô giới mà bạn đang có"

/// Point exchange rejection reason codes for `S→C 17 6E [code: u8]`.
pub const POINT_EXCHANGE_REJECT_UNDER_50: u8 = 0x01; // "Số điểm tối thiểu phải có 50 điểm mới được tham gia chức năng này"
pub const POINT_EXCHANGE_REJECT_ONLINE_200H: u8 = 0x02; // "Online chưa đủ 200 giờ"

/// Build global alert frame: `S→C 25 03 [noticeCode: u8]`.
pub fn build_tournament_notice_raw(notice_code: u8) -> Vec<u8> {
    vec![0xF4, 0x44, 0x03, 0x00, 0x25, 0x03, notice_code]
}

/// Build global alert hex frame.
pub fn tournament_notice_frame(notice_code: u8) -> String {
    encoder::hex(&build_tournament_notice_raw(notice_code))
}

/// Build match status frame: `S→C 14 28 [state: u8]` (1 = start, 2 = end).
pub fn build_match_status_raw(state: u8) -> Vec<u8> {
    vec![0xF4, 0x44, 0x03, 0x00, 0x14, 0x28, state]
}

/// Build match status hex frame.
pub fn match_status_frame(state: u8) -> String {
    encoder::hex(&build_match_status_raw(state))
}

/// Build tournament scoreboard data frame: `S→C 24 09 [mode: u8][flag: u8][count: 2B LE][records: count * 7B]`.
pub fn build_tournament_scoreboard_raw(mode: u8, flag: u8, records: &[[u8; 7]]) -> Vec<u8> {
    let count = records.len() as u16;
    let payload_len = 2 + 1 + 1 + 2 + records.len() * 7;
    let mut frame = Vec::with_capacity(4 + payload_len);
    frame.extend_from_slice(&[0xF4, 0x44]);
    frame.extend_from_slice(&(payload_len as u16).to_le_bytes());
    frame.push(0x24);
    frame.push(0x09);
    frame.push(mode);
    frame.push(flag);
    frame.extend_from_slice(&count.to_le_bytes());
    for record in records {
        frame.extend_from_slice(record);
    }
    frame
}

/// Build tournament scoreboard hex frame.
pub fn tournament_scoreboard_frame(mode: u8, flag: u8, records: &[[u8; 7]]) -> String {
    encoder::hex(&build_tournament_scoreboard_raw(mode, flag, records))
}

/// Build tournament scoreboard open dialog frame: `S→C 24 0A`.
pub fn build_tournament_scoreboard_open_raw() -> Vec<u8> {
    vec![0xF4, 0x44, 0x02, 0x00, 0x24, 0x0A]
}

/// Build tournament scoreboard open hex frame.
pub fn tournament_scoreboard_open_frame() -> String {
    encoder::hex(&build_tournament_scoreboard_open_raw())
}

/// Build point exchange reject frame: `S→C 17 6E [code: u8]`.
pub fn build_point_exchange_reject_raw(code: u8) -> Vec<u8> {
    vec![0xF4, 0x44, 0x03, 0x00, 0x17, 0x6E, code]
}

/// Build point exchange reject hex frame.
pub fn point_exchange_reject_frame(code: u8) -> String {
    encoder::hex(&build_point_exchange_reject_raw(code))
}

/// Build timed item revocation frame: `S→C 17 6F [code: u8]`.
pub fn build_timed_item_revoke_raw(code: u8) -> Vec<u8> {
    vec![0xF4, 0x44, 0x03, 0x00, 0x17, 0x6F, code]
}

/// Build timed item revocation hex frame.
pub fn timed_item_revoke_frame(code: u8) -> String {
    encoder::hex(&build_timed_item_revoke_raw(code))
}

/// Check if current time is Monday 00:00 (within a 10-minute window or since last reset).
pub fn is_monday_reset_due(now: DateTime<Utc>, last_reset: Option<DateTime<Utc>>) -> bool {
    // Monday is weekday 0 in chrono (or Monday)
    if now.weekday() != chrono::Weekday::Mon {
        return false;
    }
    // If last_reset was already performed on this same Monday, skip
    if let Some(lr) = last_reset {
        if lr.year() == now.year() && lr.ordinal() == now.ordinal() {
            return false;
        }
    }
    // During the 00:xx hour on Monday
    now.hour() == 0
}

/// Champion record to check for revocation.
#[derive(Debug, Clone)]
pub struct ChampionTenure {
    pub char_id: u32,
    pub awarded_at: DateTime<Utc>,
    pub item_id: u16,
    pub still_ranked: bool,
}

/// Process weekly reset scanning active champions for expired 7-day timed items or lost ranking.
/// Revokes timed items (`Nhẫn loạn đấu`, `Nhẫn đoàn P`, `Văn chương vô giới`),
/// removes them from inventory (bag `homdo`, equipped `trangbi`, or auxiliary `tuideo`),
/// recomputes stats if equipped items were removed, and returns revocation frames to send.
pub fn process_weekly_arena_reset(
    session: &mut Session,
    champion_records: &[ChampionTenure],
    now: DateTime<Utc>,
) -> Vec<String> {
    let mut frames = Vec::new();
    let mut stats_dirty = false;

    for champ in champion_records {
        if champ.char_id != session.id {
            continue;
        }

        let days_held = (now - champ.awarded_at).num_days();
        let is_expired = days_held >= 7 || !champ.still_ranked;

        if is_expired {
            let mut removed = false;

            // 1. Check and remove from equipped items (trangbi)
            if let Some(pos) = session.trangbi.iter().position(|i| i.id == champ.item_id) {
                session.trangbi.remove(pos);
                removed = true;
                stats_dirty = true;
            }

            // 2. Check and remove from main inventory (homdo)
            if let Some(pos) = session.homdo.iter().position(|i| i.id == champ.item_id) {
                session.homdo.remove(pos);
                removed = true;
            }

            // 3. Check and remove from auxiliary bag (tuideo)
            if let Some(pos) = session.tuideo.iter().position(|i| i.id == champ.item_id) {
                session.tuideo.remove(pos);
                removed = true;
            }

            if removed {
                let code = match champ.item_id {
                    ITEM_NHAN_LOAN_DAU | 23182..=23184 => REVOKE_NHAN_LOAN_DAU,
                    ITEM_NHAN_DOAN_P | 23185..=23187 => REVOKE_NHAN_DOAN_P,
                    ITEM_VAN_CHUONG_VO_GIOI | 23188..=23190 => REVOKE_VAN_CHUONG_VO_GIOI,
                    _ => REVOKE_NHAN_LOAN_DAU,
                };
                frames.push(timed_item_revoke_frame(code));
            }
        }
    }

    if stats_dirty {
        session.recompute_stats();
    }

    frames
}

/// Handle arena / scoreboard incoming client requests.
pub async fn handle_arena(ctx: &mut OpcodeCtx<'_>) {
    // Check opcode
    match ctx.opcode {
        0x24 => {
            // Sub-opcode 0x09: scoreboard request
            // Sub-opcode 0x0A: scoreboard open
            if ctx.sub == 0x09 {
                // Default empty scoreboard
                ctx.out.send(tournament_scoreboard_frame(1, 0, &[]));
            } else if ctx.sub == 0x0A {
                ctx.out.send(tournament_scoreboard_open_frame());
            }
        }
        0x25 => {
            // Sub-opcode 0x03: notice / info
            if ctx.sub == 0x03 && !ctx.payload.is_empty() {
                let notice_code = ctx.payload[0];
                ctx.out.send(tournament_notice_frame(notice_code));
            }
        }
        0x17 => {
            // Sub-opcode 0x6E: Point exchange
            // If points under 50, reject with S->C 17 6E 01
            if ctx.sub == 0x6E {
                // Check points (e.g. from session game points or payload)
                let points = if ctx.payload.len() >= 4 {
                    u32::from_le_bytes([ctx.payload[0], ctx.payload[1], ctx.payload[2], ctx.payload[3]])
                } else {
                    0
                };
                if points < 50 {
                    ctx.out.send(point_exchange_reject_frame(POINT_EXCHANGE_REJECT_UNDER_50));
                }
            }
        }
        _ => {}
    }
}
