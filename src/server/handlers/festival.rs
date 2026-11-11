//! Seasonal Festivals & Static Dat Parity Subsystem (Ticket 08).
//!
//! Provides:
//! - Festival UI trigger frame `S→C 24 19` with date DWORD `19000110` (`0x0121EAC6`).
//! - Seasonal festival announcement broadcaster emitting 74 announcements via `S→C 17 39`.

use crate::data::loaders::festival::FESTIVAL_PARITY_DATE_DWORD;
use crate::protocol::encoder;
use crate::server::dispatcher::OpcodeCtx;

/// Build festival UI trigger raw frame: `S→C 24 19 [date_dword: 4B LE]`.
/// Default date DWORD `19000110` (`0x0121EAC6`) matching client `Festival.Dat` timestamp 11.0.
pub fn build_festival_ui_trigger_raw(date_dword: Option<u32>) -> Vec<u8> {
    let dword = date_dword.unwrap_or(FESTIVAL_PARITY_DATE_DWORD);
    let mut payload = vec![0x24, 0x19];
    payload.extend_from_slice(&dword.to_le_bytes());
    let len = payload.len() as u16;
    let mut frame = Vec::with_capacity(4 + payload.len());
    frame.extend_from_slice(&[0xF4, 0x44]);
    frame.extend_from_slice(&len.to_le_bytes());
    frame.extend_from_slice(&payload);
    frame
}

/// Build festival UI trigger hex frame.
pub fn festival_ui_trigger_frame(date_dword: Option<u32>) -> String {
    encoder::hex(&build_festival_ui_trigger_raw(date_dword))
}

/// Build seasonal festival announcement frame: `S→C 17 39 [announcement_id: u8]`.
/// Supports codes 1..=74.
pub fn build_seasonal_announcement_raw(announcement_id: u8) -> Vec<u8> {
    vec![0xF4, 0x44, 0x03, 0x00, 0x17, 0x39, announcement_id]
}

/// Build seasonal festival announcement hex frame.
pub fn seasonal_announcement_frame(announcement_id: u8) -> String {
    encoder::hex(&build_seasonal_announcement_raw(announcement_id))
}

/// Handle festival requests.
pub async fn handle_festival(ctx: &mut OpcodeCtx<'_>) {
    if ctx.opcode == 0x24 && ctx.sub == 0x19 {
        // Open festival UI
        ctx.out.send(festival_ui_trigger_frame(None));
    }
}
