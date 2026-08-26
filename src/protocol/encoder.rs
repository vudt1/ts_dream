//! Primitive wire encoders (Chapter 2 §2.1).
//!
//! These produce uppercase-hex strings exactly as the legacy wire helpers
//! did: char codes emit min 2 digits with no upper padding (`AscW(ch)`
//! formatted as "X2"), and integers emit little-endian byte sequences.

/// 2-byte little-endian hex. E.g. `7168 -> "001C"`.
pub fn le16(v: u16) -> String {
    format!("{:02X}{:02X}", v & 0xFF, (v >> 8) & 0xFF)
}

/// 4-byte little-endian hex. E.g. `3 -> "03000000"`.
pub fn le32(v: u32) -> String {
    format!(
        "{:02X}{:02X}{:02X}{:02X}",
        v & 0xFF,
        (v >> 8) & 0xFF,
        (v >> 16) & 0xFF,
        (v >> 24) & 0xFF
    )
}

/// little-endian u16 assembled from two bytes b0 (low) and b1 (high).
pub fn u16_le(b0: u8, b1: u8) -> u16 {
    u16::from_le_bytes([b0, b1])
}

/// little-endian u32 assembled from four bytes.
pub fn u32_le(b0: u8, b1: u8, b2: u8, b3: u8) -> u32 {
    u32::from_le_bytes([b0, b1, b2, b3])
}

/// little-endian u32 from a byte slice (missing trailing bytes read as 0).
pub fn u32_le_slice(b: &[u8]) -> u32 {
    u16::from_le_bytes([
        b.first().copied().unwrap_or(0),
        b.get(1).copied().unwrap_or(0),
    ]) as u32
        | ((u16::from_le_bytes([
            b.get(2).copied().unwrap_or(0),
            b.get(3).copied().unwrap_or(0),
        ]) as u32)
            << 16)
}

/// Uppercase hex of a byte array. `[0A,0B] -> "0A0B"`.
pub fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{:02X}", b)).collect()
}

/// Hex string -> bytes. `"03000000" -> 03 00 00 00`.
///
/// Parses two hex digits per byte. An odd-length or invalid trailing group
/// aborts (returns `None`); the caller decides whether to discard the packet.
pub fn bytes(h: &str) -> Option<Vec<u8>> {
    let mut out = Vec::with_capacity(h.len() / 2);
    let trimmed = h.trim();
    if !trimmed.len().is_multiple_of(2) {
        return None;
    }
    for pair in trimmed.as_bytes().chunks(2) {
        let s = std::str::from_utf8(pair).ok()?;
        let b = u8::from_str_radix(s, 16).ok()?;
        out.push(b);
    }
    Some(out)
}

/// XOR each byte with the wire key.
pub fn xor01(bytes: &[u8]) -> Vec<u8> {
    bytes.iter().map(|b| b ^ super::XOR_KEY).collect()
}

/// Per-char 2 ASCII hex digits of the **low byte** (`& 0xFF`).
///
/// Names are VISCII byte strings, so each char is ≤ 0xFF and this is exactly
/// one byte per char. Used for every name-bearing packet's payload.
pub fn strhex(s: &[u8]) -> String {
    s.iter().map(|b| format!("{:02X}", b)).collect()
}

/// Same as [`strhex`] but from a `&str` of chars; only ASCII/≤0xFF chars are
/// meaningful (used for ASCII-only server-authored name material that fits
/// in one byte anyway).
pub fn strhex_of(s: &str) -> String {
    s.chars()
        .map(|c| format!("{:02X}", (c as u32) & 0xFF))
        .collect()
}

/// Bytes -> uppercase hex (same as [`hex`], kept under its packet-builder
/// alias).
pub fn sm3(bytes: &[u8]) -> String {
    hex(bytes)
}
