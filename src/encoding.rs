//! Text encoding contract (Chapter 4).
//!
//! Wire encoding is **VISCII 1.1** — every byte 0x00–0xFF maps to one
//! Vietnamese character. Names are held in memory as `Vec<u8>` of VISCII
//! bytes; UTF-8 mojibake (Npcs/Items) is reversed back to VISCII car-by-char.
//!
//! Garble exceptions are replicated bug-for-bug so the Rust output diffs
//! byte-exactly against captured C# traffic (Chapter 4 §4.3/§4.6).

use std::collections::HashMap;

/// Reverse mojibake map: a mojibake Unicode char -> its VISCII byte.
///
/// From research 03 §4.1:
/// - U+0000–U+007F -> same byte (ASCII).
/// - U+0080–U+009F -> byte = codepoint (C1 pass-through).
/// - U+00A0–U+00FF -> byte = codepoint (Latin-1 = CP1252 in this range).
/// - CP1252 punctuation defined in 0x80–0x9F -> its byte.
pub fn reverse_mojibake_map() -> HashMap<u32, u8> {
    let mut m = HashMap::new();
    for cp in 0u32..=0x7F {
        m.insert(cp, cp as u8);
    }
    for cp in 0x80u32..=0xFF {
        m.insert(cp, cp as u8);
    }
    // CP1252-defined punctuation in the C1 range maps to its distinct byte.
    let cp1252: &[(u32, u8)] = &[
        (0x20AC, 0x80), // €
        (0x201A, 0x82), // ‚
        (0x0192, 0x83), // ƒ
        (0x201E, 0x84), // „
        (0x2026, 0x85), // …
        (0x2020, 0x86), // †
        (0x2021, 0x87), // ‡
        (0x02C6, 0x88), // ˆ
        (0x2030, 0x89), // ‰
        (0x0160, 0x8A), // Š
        (0x2039, 0x8B), // ‹
        (0x0152, 0x8C), // Œ
        (0x017D, 0x8E), // Ž
        (0x2018, 0x91), // ‘
        (0x2019, 0x92), // ’
        (0x201C, 0x93), // “
        (0x201D, 0x94), // ”
        (0x2022, 0x95), // •
        (0x2013, 0x96), // –
        (0x2014, 0x97), // —
        (0x02DC, 0x98), // ˜
        (0x2122, 0x99), // ™
        (0x0161, 0x9A), // š
        (0x203A, 0x9B), // ›
        (0x0153, 0x9C), // œ
        (0x017E, 0x9E), // ž
        (0x0178, 0x9F), // Ÿ
    ];
    for &(cp, byte) in cp1252 {
        m.insert(cp, byte);
    }
    m
}

/// True if a 32-bit char is the single unmappable mojibake char (U+0103 `ă`).
pub fn is_unmappable(cp: u32) -> bool {
    cp == 0x0103
}

/// Map a mojibake Unicode char to its VISCII byte.
///
/// Returns the byte for mappable chars, and for the single unmappable `ă`
/// normalizes to `0xE5` (VISCII ă) as the spec §4.4 instructs.
pub fn char_to_viscii(cp: u32, map: &HashMap<u32, u8>) -> u8 {
    if is_unmappable(cp) {
        return 0xE5;
    }
    map.get(&cp).copied().unwrap_or(0x3F) // '?' fallback
}

/// Translate a Unicode string (the in-memory mojibake text) into VISCII bytes.
pub fn to_viscii(s: &str) -> Vec<u8> {
    let map = reverse_mojibake_map();
    s.chars().map(|c| char_to_viscii(c as u32, &map)).collect()
}

/// The VISCII byte→Unicode table used for display (TextEncoder.cs) plus the
/// `0xD0→Đ`, `0xDD→Đ` additions from smethod_17.
pub fn viscii_to_unicode(byte: u8) -> char {
    match byte {
        0x02 => '\u{1EB2}', // Ẳ
        0x05 => '\u{1EB4}', // Ẵ
        0x06 => '\u{1EAA}', // Ẫ
        0x14 => '\u{1EF6}', // Ỷ
        0x19 => '\u{1EF8}', // Ỹ
        0x1E => '\u{1EF4}', // Ỵ
        0x20 => ' ',
        0xD0 | 0xDD => 'Đ',
        b if b <= 0x7F => b as char,
        // Latin-1 pass-through for the rest (0x80–0xFF).
        b => b as char,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mojibake_roundtrip_simple() {
        // "D¤u Ch¤m Höi" = VISCII 44 A4 75 20 43 68 A4 6D 20 48 F6 69
        let s = "D¤u Ch¤m Höi";
        let v = to_viscii(s);
        assert_eq!(v, vec![0x44, 0xA4, 0x75, 0x20, 0x43, 0x68, 0xA4, 0x6D, 0x20, 0x48, 0xF6, 0x69]);
    }

    #[test]
    fn cp1252_punct_maps_back() {
        // „ U+201E -> byte 0x84
        assert_eq!(to_viscii("„"), vec![0x84]);
        // † U+2020 -> 0x86
        assert_eq!(to_viscii("†"), vec![0x86]);
    }

    #[test]
    fn unmappable_anh_normalized() {
        // single genuine ă U+0103 -> VISCII 0xE5
        assert_eq!(to_viscii("ă"), vec![0xE5]);
    }

    #[test]
    fn ascii_passthrough() {
        assert_eq!(to_viscii("TSVN"), b"TSVN".to_vec());
    }
}