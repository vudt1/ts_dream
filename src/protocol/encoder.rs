//! Primitive wire encoders (Chapter 2 §2.1).
//!
//! These produce uppercase-hex strings exactly as the C# helpers do. The
//! C# `smethod_13` emits `AscW(ch).ToString("X2")` — min 2 digits, no upper
//! padding — and `smethod_12` emits a little-endian u32 as 4 hex bytes.

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
        b.get(0).copied().unwrap_or(0),
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
/// Mirrors C# `smethod_4`: parses two hex digits per byte. An odd-length or
/// invalid trailing group aborts (returns `None`, mirroring the C# MsgBox +
/// truncated-array behaviour); the caller decides whether to discard.
pub fn bytes(h: &str) -> Option<Vec<u8>> {
    let mut out = Vec::with_capacity(h.len() / 2);
    let trimmed = h.trim();
    if trimmed.len() % 2 != 0 {
        return None;
    }
    let mut iter = trimmed.as_bytes().chunks(2);
    while let Some(pair) = iter.next() {
        let s = std::str::from_utf8(pair).ok()?;
        let b = u8::from_str_radix(s, 16).ok()?;
        out.push(b);
    }
    Some(out)
}

/// XOR each byte with the wire key. `smethod_5`.
pub fn xor01(bytes: &[u8]) -> Vec<u8> {
    bytes.iter().map(|b| b ^ super::XOR_KEY).collect()
}

/// Per-char 2 ASCII hex digits of the **low byte** (`& 0xFF`). `smethod_13`.
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

/// `smethod_3`: bytes -> uppercase hex (same as [`hex`], kept for parity of
/// the packet-builder naming).
pub fn sm3(bytes: &[u8]) -> String {
    hex(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn le16_little_endian() {
        assert_eq!(le16(7168), "001C");
        assert_eq!(le16(1), "0100");
        assert_eq!(le16(0x1234), "3412");
    }

    #[test]
    fn le32_little_endian() {
        assert_eq!(le32(3), "03000000");
        assert_eq!(le32(0x11223344), "44332211");
    }

    #[test]
    fn u16_u32_from_bytes() {
        assert_eq!(u16_le(0x00, 0x1C), 0x1C00);
        assert_eq!(u32_le(0x03, 0x00, 0x00, 0x00), 3);
    }

    #[test]
    fn hex_and_bytes_roundtrip() {
        assert_eq!(hex(&[0x0A, 0x0B]), "0A0B");
        assert_eq!(bytes("0A0B"), Some(vec![0x0A, 0x0B]));
        assert_eq!(bytes("0A0"), None);
    }

    #[test]
    fn xor01_inverts() {
        let data = [0xF4u8, 0x44, 0x00, 0xFF];
        let x = xor01(&data);
        assert_eq!(x, data.iter().map(|b| b ^ 0xAD).collect::<Vec<_>>());
        assert_eq!(xor01(&x), data.to_vec());
    }

    #[test]
    fn strhex_low_byte() {
        assert_eq!(strhex(b"A"), "41");
        assert_eq!(strhex(b"hi"), "6869");
        assert_eq!(strhex_of("TSVN"), "5453564E");
    }
}
